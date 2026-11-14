//! Bevy consumer of the same match/controller used by headless checks.
use crate::{
    assets::{self, PreparedCollection},
    game::{Action, CardView, Observation, Player},
    play::{Admission, Play, SaveOptions},
};
use bevy::{
    app::AppExit,
    asset::UntypedAssetId,
    camera::Viewport,
    prelude::*,
    render::{
        Render, RenderApp,
        render_asset::RenderAssets,
        renderer::{RenderQueue, render_system},
        texture::GpuImage,
    },
    ui_widgets::{Activate, Button as UiButton, ScrollArea},
    window::PrimaryWindow,
};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use unfolded_tabletop::{card, count_label, token};
mod turns;

#[derive(Resource, Default)]
struct Controller(Play);

#[derive(Resource, Clone, Default)]
struct FinalAdmission(Arc<Mutex<Admission>>);

#[derive(Resource)]
struct Artwork {
    verified: PreparedCollection,
    handles: Vec<Handle<Image>>,
    font: Handle<Font>,
}

impl Artwork {
    fn image(&self, key: &str) -> Handle<Image> {
        self.verified
            .images
            .iter()
            .position(|image| image.key == key)
            .map(|index| self.handles[index].clone())
            .expect("selected image key")
    }
}

#[derive(Resource, Clone)]
struct Uploaded {
    ids: Vec<AssetId<Image>>,
    ready: Arc<AtomicBool>,
}

#[derive(Resource, Default)]
struct Interface {
    inspected: Option<CardView>,
    inspected_owner: Option<Player>,
    history_open: bool,
    error: String,
    respond_after: f64,
}

#[derive(Component)]
struct BoardPiece;
#[derive(Component)]
struct InterfaceRoot;
#[derive(Component, Clone, Copy)]
enum ButtonCommand {
    Choose(u64, Action),
    ChooseInspected(u64, Action),
    CloseInspection,
    History,
    Restart,
    Quit,
}

pub fn run(root: &Path) -> Result<(), String> {
    run_with_save(root, None)
}

pub fn run_with_save(root: &Path, save: Option<SaveOptions>) -> Result<(), String> {
    let verified = assets::inspect(root).map_err(|error| error.to_string())?;
    run_selected(verified, Play::new(3, save))
}

/// Loads a prepared engineering continuation; it cannot create a turn preset.
pub fn run_turns_with_save(root: &Path, save: SaveOptions, seat: Player) -> Result<(), String> {
    let verified = assets::inspect_turns(root).map_err(|error| error.to_string())?;
    run_selected(verified, Play::load_turns(save, seat))
}

fn run_selected(verified: PreparedCollection, play: Play) -> Result<(), String> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: verified.root.to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Netrunner Unfolded".into(),
                    resolution: (1600, 1000).into(),
                    resize_constraints: bevy::window::WindowResizeConstraints {
                        min_width: 1280.,
                        min_height: 800.,
                        ..default()
                    },
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(Controller(play));
    install(&mut app, verified);
    run_app(app)
}

fn run_app(mut app: App) -> Result<(), String> {
    app.init_resource::<FinalAdmission>().add_systems(
        Last,
        |controller: Res<Controller>, outcome: Res<FinalAdmission>| {
            *outcome.0.lock().expect("admission report") = controller.0.admission().clone();
        },
    );
    let outcome = app.world().resource::<FinalAdmission>().clone();
    let exit = app.run();
    let admission = outcome.0.lock().expect("admission report").clone();
    match admission {
        Admission::Refused(reason) => Err(reason),
        Admission::Loading => {
            Err("Artwork loading was not completed. No match was created.".into())
        }
        Admission::Ready if exit == AppExit::Success => Ok(()),
        _ => Err("The renderer exited unsuccessfully.".into()),
    }
}

fn install(app: &mut App, verified: PreparedCollection) {
    app.add_plugins((MeshPickingPlugin, count_label::CountLabelPlugin))
        .insert_resource(ClearColor(Color::srgb(0.025, 0.04, 0.055)))
        .init_resource::<Controller>()
        .init_resource::<Interface>()
        .insert_resource(unfolded_tabletop::camera::Orbit::new((Vec3::ZERO, 27.)));
    let handles: Vec<Handle<Image>> = verified
        .images
        .iter()
        .map(|image| {
            app.world()
                .resource::<AssetServer>()
                .load(image.key.clone())
        })
        .collect();
    let uploaded = Uploaded {
        ids: handles.iter().map(Handle::id).collect(),
        ready: Arc::new(AtomicBool::new(false)),
    };
    let font = app.world().resource::<AssetServer>().load("ui.ttf");
    app.insert_resource(uploaded.clone())
        .insert_resource(Artwork {
            verified,
            handles,
            font,
        });
    app.sub_app_mut(RenderApp)
        .insert_resource(uploaded)
        .add_systems(Render, observe_uploaded_images.after(render_system));
    app.add_systems(Startup, setup).add_systems(
        Update,
        (
            admit_artwork,
            fixed_opponent,
            draw_interface,
            apply_ui_font,
            camera_viewport,
            turns::camera_controls,
            turns::position_labels,
        )
            .chain(),
    );
}

// This consumer needs upload completion, while Arkham retains its own frame-rate
// and window-health policy. No game is created merely from an asset handle.
fn observe_uploaded_images(
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
    uploaded: Res<Uploaded>,
) {
    if uploaded.ids.iter().all(|id| images.get(*id).is_some()) {
        let ready = uploaded.ready.clone();
        queue.on_submitted_work_done(move || ready.store(true, Ordering::Release));
    } else {
        uploaded.ready.store(false, Ordering::Release);
    }
}

#[allow(clippy::too_many_arguments)] // Bevy system resources are separate scheduler inputs.
fn admit_artwork(
    mut controller: ResMut<Controller>,
    art: Res<Artwork>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    mut font_context: ResMut<bevy::text::FontCx>,
    uploaded: Res<Uploaded>,
    time: Res<Time>,
) {
    if matches!(controller.0.admission(), Admission::Refused(_)) {
        return;
    }
    let mut ids: Vec<UntypedAssetId> = art
        .handles
        .iter()
        .map(|handle| handle.id().untyped())
        .collect();
    ids.push(art.font.id().untyped());
    let mut wrong_pixels = None;
    let report = unfolded_tabletop::loading::required_assets(&server, &ids, |id| {
        if id == art.font.id().untyped() {
            let Some(font) = fonts.get(&art.font) else {
                return false;
            };
            if font.data.data() != art.verified.font.as_slice() {
                wrong_pixels = Some("ui.ttf: renderer font differs from the verified font".into());
                return false;
            }
            return unfolded_tabletop::loading::font_registered(font, &mut font_context);
        }
        let index = ids
            .iter()
            .position(|candidate| *candidate == id)
            .expect("required image");
        let Some(image) = images.get(id.typed::<Image>()) else {
            return false;
        };
        let expected = &art.verified.images[index];
        let matches = image.width() == expected.width
            && image.height() == expected.height
            && image.data.as_deref() == Some(expected.rgba.as_slice());
        if !matches {
            wrong_pixels = Some(format!(
                "{}: renderer image differs from verified artwork",
                expected.key
            ));
        }
        matches
    });
    if let Some(reason) =
        wrong_pixels.or_else(|| (!report.failures.is_empty()).then(|| report.failures.join("\n")))
    {
        controller.0.refuse(reason);
    } else if report.loaded && uploaded.ready.load(Ordering::Acquire) {
        controller.0.admit();
    } else if controller.0.admission() == &Admission::Ready {
        controller.0.refuse(
            "Required artwork became unavailable. Play stopped; close and repair the collection."
                .into(),
        );
    } else if time.elapsed_secs_f64() > 30.0 {
        controller.0.refuse(format!("Required artwork did not finish loading for rendering. {} Close and check the prepared collection.", report.pending.join("\n")));
    }
}

fn apply_ui_font(
    art: Res<Artwork>,
    controller: Res<Controller>,
    mut text: Query<&mut TextFont, Added<TextFont>>,
) {
    for mut font in &mut text {
        font.font =
            bevy::text::FontSource::Handle(if controller.0.admission() == &Admission::Ready {
                art.font.clone()
            } else {
                Handle::default()
            });
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 12.5, 14.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.1, -0.5, 0.)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 500.,
        ..default()
    });
}

fn camera_viewport(
    window: Single<&Window, With<PrimaryWindow>>,
    controller: Res<Controller>,
    mut cameras: Query<&mut Camera, With<Camera3d>>,
) {
    for mut camera in &mut cameras {
        camera.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(
                (window.physical_width() as f32
                    * if turns::is_ordinary(&controller) {
                        1.
                    } else {
                        0.70
                    })
                .max(1.) as u32,
                (window.physical_height() as f32
                    * if turns::is_ordinary(&controller) {
                        0.76
                    } else {
                        1.
                    })
                .max(1.) as u32,
            ),
            ..default()
        });
    }
}

fn buttons(
    event: On<Activate>,
    commands: Query<&ButtonCommand>,
    mut controller: ResMut<Controller>,
    mut ui: ResMut<Interface>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Ok(command) = commands.get(event.entity) {
        match *command {
            ButtonCommand::Quit => {
                exit.write(AppExit::Success);
            }
            ButtonCommand::CloseInspection => {
                ui.inspected = None;
                ui.inspected_owner = None;
                ui.history_open = false;
            }
            ButtonCommand::History if ui.inspected.is_none() => ui.history_open = true,
            ButtonCommand::Choose(revision, action)
                if ui.inspected.is_none() && !ui.history_open =>
            {
                ui.error = controller
                    .0
                    .choose(revision, action)
                    .err()
                    .unwrap_or_default();
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::ChooseInspected(revision, action)
                if ui.inspected.is_some() && !ui.history_open =>
            {
                ui.error = controller
                    .0
                    .choose(revision, action)
                    .err()
                    .unwrap_or_default();
                if ui.error.is_empty() {
                    ui.inspected = None;
                    ui.inspected_owner = None;
                }
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::Restart if ui.inspected.is_none() && !ui.history_open => {
                ui.error = controller.0.restart().err().unwrap_or_default();
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            _ => {}
        }
    }
}

fn fixed_opponent(mut controller: ResMut<Controller>, mut ui: ResMut<Interface>, time: Res<Time>) {
    if controller.0.admission() == &Admission::Ready
        && ui.inspected.is_none()
        && !ui.history_open
        && ui.error.is_empty()
        && time.elapsed_secs_f64() >= ui.respond_after
        && let Err(error) = controller.0.respond()
    {
        ui.error = error;
    }
}

#[derive(Clone, PartialEq)]
struct Drawn {
    viewport: Vec2,
    observation: Option<Observation>,
    admission: Admission,
    inspected: Option<CardView>,
    history_open: bool,
    error: String,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Bevy system parameters.
fn draw_interface(
    mut commands: Commands,
    controller: Res<Controller>,
    ui: Res<Interface>,
    art: Res<Artwork>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    previous: Query<Entity, Or<(With<BoardPiece>, With<InterfaceRoot>)>>,
    mut drawn: Local<Option<Drawn>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let current = Drawn {
        viewport: window
            .as_ref()
            .map_or(Vec2::new(1600., 1000.), |window| window.size()),
        observation: controller.0.observation(),
        admission: controller.0.admission().clone(),
        inspected: ui.inspected.clone(),
        history_open: ui.history_open,
        error: ui.error.clone(),
    };
    if drawn.as_ref() == Some(&current) {
        return;
    }
    for entity in &previous {
        commands.entity(entity).despawn();
    }
    let ready = current.admission == Admission::Ready;
    let ordinary = ready
        && current
            .observation
            .as_ref()
            .is_some_and(|view| view.active_player.is_some());
    if ready && let Some(view) = &current.observation {
        if ordinary {
            turns::draw_board(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                view,
                controller.0.human(),
            );
            turns::draw_dock(&mut commands, view, &ui, controller.0.human());
        } else {
            draw_board(&mut commands, &mut meshes, &mut materials, &art, view);
            commands
                .spawn((
                    InterfaceRoot,
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(2.),
                        top: px(14.),
                        width: percent(66.),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    label(row, "Archives · left", 18., Color::WHITE);
                    label(row, "Run on HQ · centre", 22., Color::srgb(0.45, 1., 0.78));
                    label(row, "R&D · right", 18., Color::WHITE);
                });
        }
    }
    if !ordinary {
        commands.spawn((InterfaceRoot, ScrollArea, Node { position_type: PositionType::Absolute, left: percent(70.), top: px(0.), width: percent(30.), height: percent(100.), padding: UiRect::all(px(22.)), flex_direction: FlexDirection::Column, row_gap: px(14.), overflow: Overflow::scroll_y(), ..default() }, BackgroundColor(Color::srgb(0.045, 0.065, 0.085)))).with_children(|panel| {
        label(panel, "NETRUNNER UNFOLDED", 20., Color::srgb(0.55, 0.85, 0.88));
        label(panel, if ready { "Brân at HQ" } else { "Bran at HQ" }, 32., Color::WHITE);
        match &current.admission {
            Admission::Loading => label(panel, "Checking required artwork and renderer...", 21., Color::WHITE),
            Admission::Refused(reason) => {
                label(panel, "Play stopped", 28., Color::srgb(1., 0.5, 0.4));
                label(panel, reason, 19., Color::WHITE);
                label(panel, &format!("Artwork folder: {}\nRepair or prepare the collection, then restart.", art.verified.root.display()), 17., Color::WHITE);
            }
            Admission::Ready => {
                let view = current.observation.as_ref().expect("admitted match");
                label(panel, "You are the Corporation", 19., Color::srgb(0.7, 0.8, 0.86));
                label(panel, &format!("{} credits · HQ: {} cards", view.corp_credits, view.hq_count), 22., Color::WHITE);
                label(panel, view.phase, 22., Color::WHITE);
                if current.inspected.is_none() && !current.history_open {
                    if let Some(decision) = &view.decision {
                        for action in &decision.choices { button(panel, action.label(), ButtonCommand::Choose(decision.revision, *action)); }
                    } else if view.run_result.is_some() {
                        button(panel, "Start this position again", ButtonCommand::Restart);
                    } else { label(panel, "Runner is deciding…", 18., Color::WHITE); }
                }
                if !current.error.is_empty() { label(panel, &current.error, 18., Color::srgb(1., 0.5, 0.4)); }
                label(panel, "What happened", 22., Color::srgb(0.55, 0.85, 0.88));
                if let Some(line) = view.history.last() { label(panel, line, 18., Color::srgb(0.85, 0.89, 0.91)); }
                if current.inspected.is_none() && !current.history_open { button(panel, "Run history", ButtonCommand::History); }
                label(panel, "Click a card to inspect its face and current text. This run ends without declaring a match winner.", 16., Color::srgb(0.65, 0.73, 0.79));
            }
        }
        button(panel, "Quit", ButtonCommand::Quit);
    });
    }
    if ready && current.history_open {
        commands
            .spawn((
                InterfaceRoot,
                ScrollArea,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(if ordinary { 100. } else { 70. }),
                    height: percent(100.),
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(px(36.)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(24.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.025, 0.04, 0.055)),
            ))
            .with_children(|modal| {
                label(modal, "Run history", 30., Color::WHITE);
                for line in &current
                    .observation
                    .as_ref()
                    .expect("admitted match")
                    .history
                {
                    label(modal, line, 21., Color::WHITE);
                }
                button(modal, "Close history", ButtonCommand::CloseInspection);
            });
    }
    if ready && let Some(card) = &current.inspected {
        // Fit the selected two-column inspection inside its padded frame.
        // Width alone overflows vertically on wide/maximized windows.
        let panel_width = current.viewport.x * if ordinary { 0.63 } else { 0.7 };
        let panel_height = current.viewport.y * if ordinary { 0.7 } else { 1. };
        let image_width = ((panel_width - 60.) * 0.52)
            .min((panel_height - 60.) * 746. / 1033.)
            .max(1.);
        commands
            .spawn((
                InterfaceRoot,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(20.),
                    top: px(20.),
                    width: percent(if ordinary { 63. } else { 70. }),
                    height: percent(if ordinary { 70. } else { 100. }),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px(30.)),
                    column_gap: px(24.),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.01, 0.02, 0.03, 0.97)),
                Pickable::default(),
            ))
            .with_children(|modal| {
                let key = card
                    .printing
                    .map(|code| format!("cards/{code}.png"))
                    .unwrap_or_else(|| {
                        turns::back(ui.inspected_owner.unwrap_or(Player::Corp)).into()
                    });
                modal.spawn((
                    ImageNode::new(art.image(&key)),
                    Node {
                        width: px(image_width),
                        aspect_ratio: Some(746. / 1033.),
                        border_radius: BorderRadius::all(CornerRadius::circular(percent(
                            card::CORNER_FRACTION * 100.,
                        ))),
                        ..default()
                    },
                ));
                modal
                    .spawn((
                        ScrollArea,
                        Node {
                            width: percent(40.),
                            height: percent(100.),
                            overflow: Overflow::scroll_y(),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(20.),
                            ..default()
                        },
                    ))
                    .with_children(|text| {
                        label(
                            text,
                            card.title.as_deref().unwrap_or("Facedown card"),
                            28.,
                            Color::WHITE,
                        );
                        label(
                            text,
                            card.rules_text.unwrap_or("This face is hidden from you."),
                            21.,
                            Color::WHITE,
                        );
                        label(
                            text,
                            "Current effective text · CR 26.03 / NSG card data",
                            15.,
                            Color::srgb(0.65, 0.73, 0.79),
                        );
                        if ordinary
                            && let Some(decision) =
                                &current.observation.as_ref().expect("admitted").decision
                        {
                            for action in &decision.choices {
                                if turns::action_card(*action) == card.instance
                                    && card.instance.is_some()
                                {
                                    button(
                                        text,
                                        &action.description(current.observation.as_ref().unwrap()),
                                        ButtonCommand::ChooseInspected(decision.revision, *action),
                                    );
                                }
                            }
                        }
                        button(text, "Close inspection", ButtonCommand::CloseInspection);
                    });
            });
    }
    *drawn = Some(current);
}

fn label(parent: &mut ChildSpawnerCommands, text: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: size.into(),
            ..default()
        },
        TextColor(color),
    ));
}

fn button(parent: &mut ChildSpawnerCommands, text: &str, command: ButtonCommand) {
    themed_button(parent, text, command, Color::srgb(0.12, 0.25, 0.30));
}

fn themed_button(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    command: ButtonCommand,
    tint: Color,
) {
    parent
        .spawn((
            UiButton,
            command,
            Node {
                padding: UiRect::axes(px(16.), px(12.)),
                min_height: px(46.),
                width: percent(100.),
                border_radius: BorderRadius::all(px(5.)),
                ..default()
            },
            BackgroundColor(tint),
        ))
        .observe(buttons)
        .with_children(|button| label(button, text, 19., Color::WHITE));
}

fn draw_board(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    view: &Observation,
) {
    commands.spawn((
        BoardPiece,
        Mesh3d(meshes.add(Cuboid::new(13., 0.3, 15.))),
        MeshMaterial3d(materials.add(Color::srgb(0.22, 0.34, 0.31))),
        Transform::from_xyz(0., -0.20, 0.),
    ));
    for (name, x, z) in [("HQ", 0., 3.3), ("Archives", -4.2, 3.3), ("R&D", 4.2, 3.3)] {
        let color = if name == "HQ" {
            Color::srgb(0.09, 0.35, 0.28)
        } else {
            Color::srgb(0.08, 0.19, 0.21)
        };
        commands.spawn((
            BoardPiece,
            Mesh3d(meshes.add(Cuboid::new(2.6, 0.02, 3.1))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_xyz(x, -0.025, z),
            Name::new(name),
        ));
    }
    spawn_card(
        commands,
        meshes,
        materials,
        art,
        view.corp_identity.clone(),
        Transform::from_xyz(4.3, 0.08, 6.),
        true,
    );
    spawn_card(
        commands,
        meshes,
        materials,
        art,
        view.runner_identity.clone(),
        Transform::from_xyz(4.3, 0.08, -5.5)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        true,
    );
    for (index, card) in view.ice.iter().enumerate() {
        let z = if view.ice.len() == 1 && card.printing == Some("30039") {
            -1.7
        } else {
            0.7 - index as f32 * 2.4
        };
        if view.approached_ice == Some(index)
            || (view
                .decision
                .as_ref()
                .is_some_and(|d| d.choices.contains(&Action::InstallPalisade))
                && card.printing == Some("30039"))
        {
            commands.spawn((
                BoardPiece,
                Mesh3d(meshes.add(Cuboid::new(3., 0.025, 2.2))),
                MeshMaterial3d(materials.add(Color::srgb(0.12, 0.65, 0.55))),
                Transform::from_xyz(0., 0.005, z),
            ));
        }
        spawn_card(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            Transform::from_xyz(0., 0.08, z)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            card.rezzed,
        );
    }
    for card in &view.hand {
        spawn_card(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            Transform::from_xyz(-1.7, 0.08, 6.),
            true,
        );
    }
    if let Some(card) = &view.installing {
        spawn_card(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            Transform::from_xyz(0., 0.15, 0.7)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            false,
        );
    }
    for card in &view.archives {
        spawn_card(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            Transform::from_xyz(-4.2, 0.08, 3.3),
            true,
        );
    }
    for index in 0..view.corp_credits {
        let root = token::spawn_body(
            commands,
            meshes,
            materials,
            token::Body {
                name: "1 credit".into(),
                mesh: Cylinder::new(0.38, 0.15).into(),
                color: Color::srgb(0.56, 0.62, 0.63),
                transform: Transform::from_xyz(-4.9 + index as f32 * 0.85, 0.09, 6.),
            },
        );
        commands.entity(root).insert(BoardPiece);
        token::add_face(
            commands,
            meshes,
            materials,
            root,
            token::Face {
                mesh: Circle::new(0.38).into(),
                image: art.image("tokens/credit.png"),
                alpha: AlphaMode::Opaque,
                transform: Transform::from_xyz(0., 0.077, 0.)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            },
        );
    }
}

fn spawn_card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    card_view: CardView,
    transform: Transform,
    face_up: bool,
) {
    spawn_card_for(
        commands,
        meshes,
        materials,
        art,
        card_view,
        transform,
        face_up,
        Player::Corp,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_card_for(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    card_view: CardView,
    transform: Transform,
    face_up: bool,
    owner: Player,
) {
    let key = card_view
        .printing
        .filter(|_| face_up)
        .map(|code| format!("cards/{code}.png"))
        .unwrap_or_else(|| turns::back(owner).into());
    let body = meshes.add(card::body(1.8, 0.06, 2.5));
    let face = meshes.add(card::face(1.8, 2.5));
    let paper = materials.add(Color::srgb(0.72, 0.75, 0.73));
    let picture = materials.add(StandardMaterial {
        base_color_texture: Some(art.image(&key)),
        unlit: true,
        ..default()
    });
    commands
        .spawn((
            BoardPiece,
            Mesh3d(body),
            MeshMaterial3d(paper),
            transform,
            Name::new(
                card_view
                    .title
                    .clone()
                    .unwrap_or_else(|| "Facedown card".into()),
            ),
        ))
        .with_child((
            Mesh3d(face),
            MeshMaterial3d(picture),
            Transform::from_xyz(0., 0.032, 0.),
        ))
        .observe(
            move |mut click: On<PointerClick>,
                  mut ui: ResMut<Interface>,
                  controller: Res<Controller>| {
                click.propagate(false);
                if click.button == bevy::picking::pointer::PointerButton::Primary
                    && controller.0.admission() == &Admission::Ready
                    && ui.inspected.is_none()
                    && !ui.history_open
                {
                    ui.inspected = Some(card_view.clone());
                    ui.inspected_owner = Some(owner);
                }
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_runner_return_preserves_the_final_admission_outcome() {
        for refused in [false, true] {
            let mut app = App::new();
            app.init_resource::<Controller>();
            app.set_runner(move |mut app| {
                app.world_mut().resource_mut::<Controller>().0.admit();
                if refused {
                    app.world_mut()
                        .resource_mut::<Controller>()
                        .0
                        .refuse("repair ui.ttf".into());
                }
                app.update();
                AppExit::Success
            });
            assert_eq!(
                run_app(app),
                if refused {
                    Err("repair ui.ttf".into())
                } else {
                    Ok(())
                }
            );
        }
    }

    fn interface_app() -> App {
        let mut app = App::new();
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let font = fonts.add(Font::from_bytes(vec![]));
        let keys = [
            "cards/30039.png",
            "cards/30072.png",
            "cards/30076.png",
            "cards/30077.png",
            "tokens/credit.png",
            "backs/corp.png",
            "backs/runner.png",
            "tokens/click-red.png",
            "tokens/click-blue.png",
            "tokens/advancement.png",
            "cards/30006.png",
            "cards/30013.png",
            "cards/30026.png",
            "cards/30029.png",
            "cards/30074.png",
            "cards/30030.png",
            "cards/30040.png",
            "cards/30064.png",
            "cards/30067.png",
            "cards/30075.png",
        ];
        app.add_plugins((MinimalPlugins, count_label::CountLabelPlugin))
            .add_message::<AppExit>()
            .init_resource::<Controller>()
            .init_resource::<Interface>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(Artwork {
                verified: PreparedCollection {
                    root: "/test-only".into(),
                    font: vec![],
                    images: keys
                        .iter()
                        .map(|key| assets::VerifiedImage {
                            key: (*key).into(),
                            width: 0,
                            height: 0,
                            rgba: vec![],
                        })
                        .collect(),
                },
                handles: (0..keys.len())
                    .map(|_| images.add(Image::default()))
                    .collect(),
                font,
            })
            .insert_resource(images)
            .insert_resource(fonts)
            .add_systems(Update, (draw_interface, apply_ui_font).chain());
        app.world_mut().resource_mut::<Controller>().0.admit();
        app.update();
        app
    }

    #[test]
    fn ordinary_table_uses_one_token_with_exact_counts_including_zero_and_updates() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        use unfolded_tabletop::count_label::Label;

        fn assert_counts(world: &mut World, expected: &[(&str, &str)]) {
            for (name, amount) in expected {
                let tokens: Vec<_> = world
                    .query::<(Entity, &Name)>()
                    .iter(world)
                    .filter(|(_, actual)| actual.as_str() == *name)
                    .map(|(entity, _)| entity)
                    .collect();
                assert_eq!(tokens.len(), 1, "{name} must occupy one token");
                let captions: Vec<_> = world
                    .query::<(&Label, &Text)>()
                    .iter(world)
                    .filter(|(label, _)| label.token == tokens[0])
                    .map(|(_, text)| text.0.as_str())
                    .collect();
                assert_eq!(captions, [*amount], "{name} must show its exact count");
            }
        }

        for seat in [Player::Corp, Player::Runner] {
            let mut position = EngineeringPosition::economy(3);
            position.corp_credits = 12;
            position.runner_credits = 0;
            let mut office =
                EngineeringCard::new(CardKind::OffworldOffice, Zone::Root(Server::Remote(1)));
            office.installed_turn = Some(1);
            office.advancement = 2;
            position.cards.push(office);
            let mut play = Play::engineering(position, None, seat);
            play.admit();
            assert_eq!(play.admission(), &Admission::Ready);
            let mut app = interface_app();
            app.insert_resource(Controller(play));
            app.update();
            assert_counts(
                app.world_mut(),
                &[
                    ("Corp Credit pool", "12"),
                    ("Corp Click pool", "3"),
                    ("Runner Credit pool", "0"),
                    ("Runner Click pool", "0"),
                    ("Corp Advancement", "2"),
                ],
            );
            if seat == Player::Corp {
                for _ in 0..2 {
                    app.world_mut()
                        .resource_mut::<Controller>()
                        .0
                        .respond()
                        .unwrap();
                    app.update();
                }
                activate(&mut app, Action::GainCredit);
                assert_counts(
                    app.world_mut(),
                    &[
                        ("Corp Credit pool", "13"),
                        ("Corp Click pool", "2"),
                        ("Runner Credit pool", "0"),
                        ("Runner Click pool", "0"),
                        ("Corp Advancement", "2"),
                    ],
                );
            }
        }
    }

    #[test]
    fn ordinary_table_places_each_human_near_and_opponent_far() {
        use crate::game::{EngineeringPosition, Player};
        for seat in [Player::Runner, Player::Corp] {
            let mut app = interface_app();
            let mut play = Play::engineering(EngineeringPosition::economy(3), None, seat);
            play.admit();
            app.insert_resource(Controller(play));
            app.update();
            let world = app.world_mut();
            for (name, transform) in world.query::<(&Name, &Transform)>().iter(world) {
                if name.as_str() == "The Catalyst" {
                    assert_eq!(transform.translation.z > 0., seat == Player::Runner);
                    assert_eq!(
                        (transform.rotation * Vec3::Z).z > 0.,
                        seat == Player::Runner
                    );
                }
                if name.as_str() == "The Syndicate" {
                    assert_eq!(transform.translation.z > 0., seat == Player::Corp);
                    assert_eq!((transform.rotation * Vec3::Z).z > 0., seat == Player::Corp);
                }
            }
        }
    }

    #[test]
    fn corp_mat_contains_full_ice_columns_from_either_seat() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        for seat in [Player::Runner, Player::Corp] {
            for depth in [1, 2, 6] {
                let mut position = EngineeringPosition::economy(3);
                for _ in 0..depth {
                    let mut card =
                        EngineeringCard::new(CardKind::Palisade, Zone::Protecting(Server::Hq));
                    card.rezzed = true;
                    card.installed_turn = Some(1);
                    position.cards.push(card);
                }
                let mut play = Play::engineering(position, None, seat);
                play.admit();
                assert_eq!(play.admission(), &Admission::Ready);
                let mut app = interface_app();
                app.insert_resource(Controller(play));
                app.update();
                let world = app.world();
                let meshes = world.resource::<Assets<Mesh>>();
                let bounds = |entity: EntityRef<'_>| {
                    let mesh = meshes.get(&entity.get::<Mesh3d>().unwrap().0).unwrap();
                    let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                    else {
                        panic!("mesh positions")
                    };
                    let transform = entity.get::<Transform>().unwrap();
                    vertices
                        .iter()
                        .map(|p| transform.transform_point(Vec3::from_array(*p)))
                        .fold(
                            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                            |(min, max), p| (min.min(p), max.max(p)),
                        )
                };
                let mat = world
                    .iter_entities()
                    .find(|e| {
                        e.get::<Name>()
                            .is_some_and(|n| n.as_str() == "Corp playmat")
                    })
                    .unwrap();
                let (min, max) = bounds(mat);
                let cards: Vec<_> = world
                    .iter_entities()
                    .filter(|e| e.get::<Name>().is_some_and(|n| n.as_str() == "Palisade"))
                    .collect();
                assert_eq!(cards.len(), depth);
                for card in cards {
                    let (lo, hi) = bounds(card);
                    assert!(
                        lo.x >= min.x
                            && hi.x <= max.x
                            && lo.z >= min.z + 0.5
                            && hi.z <= max.z - 0.5,
                        "{seat:?}, {depth} ICE: card {lo:?}..{hi:?} needs padding inside mat {min:?}..{max:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn native_playmats_and_opposing_grip_have_physical_clearance_in_both_seats() {
        use crate::game::EngineeringPosition;
        for seat in [Player::Runner, Player::Corp] {
            let mut app = interface_app();
            let mut play = Play::engineering(EngineeringPosition::economy(3), None, seat);
            play.admit();
            app.insert_resource(Controller(play));
            app.update();
            let world = app.world_mut();
            let meshes = world.resource::<Assets<Mesh>>();
            let bounds = |mesh: &Mesh3d, transform: &Transform| {
                let bevy::mesh::VertexAttributeValues::Float32x3(vertices) = meshes
                    .get(&mesh.0)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                else {
                    panic!("mesh positions");
                };
                vertices
                    .iter()
                    .map(|p| transform.transform_point(Vec3::from_array(*p)))
                    .fold(
                        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                        |(min, max), p| (min.min(p), max.max(p)),
                    )
            };
            let mats: Vec<_> = world
                .iter_entities()
                .filter_map(|entity| {
                    let name = entity.get::<Name>()?;
                    name.as_str().ends_with("playmat").then(|| {
                        bounds(
                            entity.get::<Mesh3d>().unwrap(),
                            entity.get::<Transform>().unwrap(),
                        )
                    })
                })
                .collect();
            assert_eq!(mats.len(), 2);
            let gap = (mats[0].0.z - mats[1].1.z).max(mats[1].0.z - mats[0].1.z);
            assert!(
                gap >= 1.,
                "playmats need a visible physical gap, found {gap}"
            );
            for token in world.iter_entities().filter(|entity| {
                entity.get::<Name>().is_some_and(|name| {
                    name.as_str().ends_with("Credit pool") || name.as_str().ends_with("Click pool")
                })
            }) {
                let owner = if token.get::<Name>().unwrap().as_str().starts_with("Corp ") {
                    Player::Corp
                } else {
                    Player::Runner
                };
                let mat_name = format!("{owner:?} playmat");
                let mat = world
                    .iter_entities()
                    .find(|entity| {
                        entity
                            .get::<Name>()
                            .is_some_and(|name| name.as_str() == mat_name)
                    })
                    .unwrap();
                let (mat_min, mat_max) = bounds(
                    mat.get::<Mesh3d>().unwrap(),
                    mat.get::<Transform>().unwrap(),
                );
                let (token_min, token_max) = bounds(
                    token.get::<Mesh3d>().unwrap(),
                    token.get::<Transform>().unwrap(),
                );
                assert!(
                    token_min.x >= mat_min.x
                        && token_max.x <= mat_max.x
                        && token_min.z >= mat_min.z
                        && token_max.z <= mat_max.z,
                    "{owner:?} {} must be wholly on its owner's mat: token {token_min:?}..{token_max:?}, mat {mat_min:?}..{mat_max:?}",
                    token.get::<Name>().unwrap()
                );
            }
            if seat == Player::Corp {
                let hand: Vec<_> = world
                    .iter_entities()
                    .filter_map(|entity| {
                        let transform = entity.get::<Transform>()?;
                        let name = entity.get::<Name>()?;
                        (name.as_str() == "Facedown card" && transform.translation.z < -6.)
                            .then(|| bounds(entity.get::<Mesh3d>().unwrap(), transform))
                    })
                    .collect();
                assert_eq!(hand.len(), 4);
                for (i, a) in hand.iter().enumerate() {
                    for b in &hand[i + 1..] {
                        let gap = (a.0.x - b.1.x).max(b.0.x - a.1.x);
                        assert!(
                            gap >= 0.2,
                            "opposing grip cards must not intersect or touch, found {gap}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn native_turn_choice_pays_the_shown_cost_and_resumes_the_same_decision() {
        use crate::game::EngineeringPosition;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("turn.json");
        let mut position = EngineeringPosition::economy(3);
        position.active = Player::Runner;
        let options = SaveOptions {
            path: path.clone(),
            recover_from: None,
        };
        let mut play = Play::engineering(position, Some(options.clone()), Player::Runner);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        for _ in 0..2 {
            app.world_mut()
                .resource_mut::<Controller>()
                .0
                .respond()
                .unwrap();
            app.update();
        }
        let view = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        let gamble = view
            .hand
            .iter()
            .find(|c| c.printing == Some("30030"))
            .unwrap()
            .instance
            .unwrap();
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "Play Sure Gamble · 1 click + 5 credits")
        );
        let before = std::fs::read(&path).unwrap();
        app.world_mut().resource_mut::<Interface>().history_open = true;
        activate(&mut app, Action::PlayCard(gamble));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        app.world_mut().resource_mut::<Interface>().history_open = false;
        app.update();
        activate(&mut app, Action::PlayCard(gamble));
        let after = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        assert_eq!(after.runner_credits, 9);
        assert_eq!(after.clicks[1], 3);
        let mut reopened = Play::load_turns(options, Player::Runner);
        reopened.admit();
        assert_eq!(reopened.observation(), Some(after));
    }

    #[test]
    fn native_run_choice_selects_whitespace_second_subroutine_and_spends_hosted_credit() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        let directory = tempfile::tempdir().unwrap();
        let options = SaveOptions {
            path: directory.path().join("run.json"),
            recover_from: None,
        };
        let mut position = EngineeringPosition::economy(42);
        position.active = Player::Runner;
        position.runner_credits = 8;
        position.cards = vec![EngineeringCard::new(CardKind::Overclock, Zone::Grip)];
        for (kind, zone, rezzed) in [
            (CardKind::Whitespace, Zone::Protecting(Server::Hq), true),
            (CardKind::Unity, Zone::Rig, false),
            (CardKind::Cleaver, Zone::Rig, false),
        ] {
            position.cards.push(EngineeringCard {
                kind,
                zone,
                rezzed,
                advancement: 0,
                installed_turn: Some(1),
            });
        }
        let mut play = Play::engineering(position, Some(options.clone()), Player::Runner);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        fn advance(app: &mut App, phase: &str) {
            for _ in 0..20 {
                let view = app
                    .world()
                    .resource::<Controller>()
                    .0
                    .observation()
                    .unwrap();
                if view.phase.starts_with(phase) {
                    return;
                }
                if view.decision.is_some() {
                    activate(app, Action::PassPriority);
                } else {
                    app.world_mut()
                        .resource_mut::<Controller>()
                        .0
                        .respond()
                        .unwrap();
                    app.update();
                }
            }
            panic!("phase not reached");
        }
        advance(&mut app, "Action phase");
        let event = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap()
            .hand[0]
            .instance
            .unwrap();
        activate(&mut app, Action::PlayCard(event));
        {
            let world = app.world_mut();
            assert!(
                world
                    .query::<&Name>()
                    .iter(world)
                    .any(|n| n.as_str() == "Runner Overclock credits")
            );
        }
        activate(&mut app, Action::RunServer(Server::Hq));
        advance(&mut app, "Encounter");
        let unity = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap()
            .rig[0]
            .instance
            .unwrap();
        activate(
            &mut app,
            Action::Break {
                breaker: unity,
                subroutines: 2,
            },
        );
        {
            let world = app.world_mut();
            assert!(
                world
                    .query::<&Text>()
                    .iter(world)
                    .any(|t| t.0 == "Pay 0 pool + 1 Overclock")
            );
        }
        activate(&mut app, Action::PayRunCredits(1));
        activate(&mut app, Action::PassPriority);
        app.world_mut()
            .resource_mut::<Controller>()
            .0
            .respond()
            .unwrap();
        app.update();
        let view = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        assert_eq!(view.runner_credits, 4);
        assert_eq!(view.run.as_ref().unwrap().hosted_credits, 4);
        assert!(view.run.as_ref().unwrap().subroutines[1].broken);
        let world = app.world_mut();
        let token = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, n)| n.as_str() == "Runner Overclock credits")
            .unwrap()
            .0;
        assert!(
            world
                .query::<(&count_label::Label, &Text)>()
                .iter(world)
                .any(|(l, t)| l.token == token && t.0 == "4")
        );
        let mut resumed = Play::load_turns(options, Player::Runner);
        resumed.admit();
        assert_eq!(resumed.observation(), Some(view));
    }

    #[test]
    fn runner_table_never_exposes_hidden_root_identity_or_its_actions() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        let mut position = EngineeringPosition::economy(3);
        let mut root_card =
            EngineeringCard::new(CardKind::OffworldOffice, Zone::Root(Server::Remote(1)));
        root_card.installed_turn = Some(1);
        position.cards.push(root_card);
        let mut play = Play::engineering(position, None, Player::Runner);
        play.admit();
        assert_eq!(play.admission(), &Admission::Ready);
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        assert!(
            !world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0.contains("Offworld"))
        );
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str().contains("Offworld"))
        );
        let backs = [
            world.resource::<Artwork>().image("backs/corp.png"),
            world.resource::<Artwork>().image("backs/runner.png"),
        ];
        for back in backs {
            assert!(
                world
                    .resource::<Assets<StandardMaterial>>()
                    .iter()
                    .any(|(_, material)| material.base_color_texture.as_ref() == Some(&back))
            );
        }
        let view = world.resource::<Controller>().0.observation().unwrap();
        let hidden = &view
            .servers
            .iter()
            .find(|s| s.server == Server::Remote(1))
            .unwrap()
            .root[0];
        assert_eq!(hidden.printing, None);
        assert_eq!(hidden.instance, None);
    }

    #[test]
    fn native_camera_captures_modals_and_dock_wheel_but_navigates_the_table() {
        use crate::game::EngineeringPosition;
        use bevy::{
            ecs::system::RunSystemOnce,
            input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
        };
        use unfolded_tabletop::camera::Orbit;
        let mut app = interface_app();
        let mut play = Play::engineering(EngineeringPosition::economy(3), None, Player::Runner);
        play.admit();
        app.insert_resource(Controller(play));
        app.insert_resource(Orbit::new((Vec3::ZERO, 27.)));
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.insert_resource(AccumulatedMouseScroll {
            unit: MouseScrollUnit::Line,
            delta: Vec2::new(0., 1.),
        });
        let mut window = Window {
            focused: true,
            resolution: (1600, 1000).into(),
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(300., 900.)));
        let window_id = app.world_mut().spawn((window, PrimaryWindow)).id();
        app.world_mut()
            .spawn((Camera3d::default(), Transform::default()));
        let initial = app.world().resource::<Orbit>().pose();
        app.world_mut()
            .run_system_once(turns::camera_controls)
            .unwrap();
        assert_eq!(app.world().resource::<Orbit>().pose(), initial);
        app.world_mut()
            .get_mut::<Window>(window_id)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(300., 300.)));
        app.world_mut().resource_mut::<Interface>().history_open = true;
        app.world_mut()
            .run_system_once(turns::camera_controls)
            .unwrap();
        assert_eq!(app.world().resource::<Orbit>().pose(), initial);
        app.world_mut().resource_mut::<Interface>().history_open = false;
        app.world_mut()
            .run_system_once(turns::camera_controls)
            .unwrap();
        assert!(app.world().resource::<Orbit>().pose().distance < initial.distance);
    }

    #[test]
    fn a_failed_required_font_keeps_repair_text_on_the_bundled_font() {
        let mut app = interface_app();
        let required = app.world().resource::<Artwork>().font.clone();
        assert_ne!(required, Handle::default());
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .remove(required.id());
        app.world_mut()
            .resource_mut::<Controller>()
            .0
            .refuse("ui.ttf could not load; repair the prepared collection".into());
        app.update();
        let world = app.world_mut();
        let repair = world
            .query::<(&Text, &TextFont)>()
            .iter(world)
            .find(|(text, _)| text.0.contains("could not load"))
            .unwrap();
        assert_eq!(
            repair.1.font,
            bevy::text::FontSource::Handle(Handle::default())
        );
        assert!(
            world
                .query_filtered::<Entity, With<ScrollArea>>()
                .iter(world)
                .next()
                .is_some()
        );
    }

    fn activate(app: &mut App, wanted: Action) {
        let world = app.world_mut();
        let entity = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::Choose(_, action) if *action == wanted)
                    .then_some(entity)
            })
            .expect("visible legal action");
        world.trigger(Activate { entity });
        app.update();
    }

    #[test]
    fn native_buttons_finish_the_run_and_modal_state_blocks_underlying_choices() {
        let mut app = interface_app();
        let before = app.world().resource::<Controller>().0.observation();
        app.world_mut().resource_mut::<Interface>().history_open = true;
        activate(&mut app, Action::InstallPalisade);
        assert_eq!(app.world().resource::<Controller>().0.observation(), before);
        app.world_mut().resource_mut::<Interface>().history_open = false;
        app.update();
        activate(&mut app, Action::InstallPalisade);
        activate(&mut app, Action::TrashBran);
        app.world_mut()
            .resource_mut::<Controller>()
            .0
            .respond()
            .unwrap();
        app.update();
        activate(&mut app, Action::RezPalisade);
        let view = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        assert_eq!(view.corp_credits, 0);
        assert_eq!(view.run_result, Some(crate::game::RunResult::EndedByIce));
        assert_eq!(view.match_winner, None);
    }

    #[test]
    fn native_table_has_solid_token_bodies_with_the_required_opaque_face() {
        let mut app = interface_app();
        let world = app.world_mut();
        let token_roots: Vec<Entity> = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .filter_map(|(entity, name)| (name.as_str() == "1 credit").then_some(entity))
            .collect();
        assert_eq!(token_roots.len(), 3);
        for entity in token_roots {
            let mesh = world
                .resource::<Assets<Mesh>>()
                .get(&world.get::<Mesh3d>(entity).unwrap().0)
                .unwrap();
            let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("positions");
            };
            assert!(vertices.iter().any(|p| p[1] < 0.) && vertices.iter().any(|p| p[1] > 0.));
            let child = world.get::<Children>(entity).unwrap()[0];
            let material = world
                .resource::<Assets<StandardMaterial>>()
                .get(
                    &world
                        .get::<MeshMaterial3d<StandardMaterial>>(child)
                        .unwrap()
                        .0,
                )
                .unwrap();
            assert_eq!(
                material.base_color_texture,
                Some(world.resource::<Artwork>().image("tokens/credit.png"))
            );
            assert_eq!(material.alpha_mode, AlphaMode::Opaque);
            let normal = world.get::<Transform>(child).unwrap().rotation * Vec3::Z;
            assert!(normal.dot(Vec3::Y) > 0.99);
        }
    }

    #[test]
    fn installed_unrezzed_ice_uses_the_corp_back_but_inspection_receives_its_authorized_face() {
        use bevy::{
            camera::RenderTarget,
            picking::{
                backend::HitData,
                events::Pointer,
                pointer::{Location, PointerButton, PointerId},
            },
            window::WindowRef,
        };
        let mut app = interface_app();
        activate(&mut app, Action::InstallPalisade);
        activate(&mut app, Action::TrashBran);
        let world = app.world_mut();
        let entity = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, name)| name.as_str() == "Palisade")
            .unwrap()
            .0;
        let face = world.get::<Children>(entity).unwrap()[0];
        let material = world
            .resource::<Assets<StandardMaterial>>()
            .get(
                &world
                    .get::<MeshMaterial3d<StandardMaterial>>(face)
                    .unwrap()
                    .0,
            )
            .unwrap();
        assert_eq!(
            material.base_color_texture,
            Some(world.resource::<Artwork>().image("backs/corp.png"))
        );
        let window = world.spawn_empty().id();
        world.trigger(PointerClick {
            entity,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: RenderTarget::Window(WindowRef::Entity(window))
                        .normalize(Some(window))
                        .unwrap(),
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(window, 1., None, None),
            duration: std::time::Duration::from_millis(20),
            count: 1,
        });
        let inspected = world.resource::<Interface>().inspected.as_ref().unwrap();
        assert_eq!(inspected.printing, Some("30072"));
        assert_eq!(inspected.title.as_deref(), Some("Palisade"));
        assert!(inspected.rules_text.unwrap().contains("Rez: 3"));
    }

    #[test]
    fn renderer_decode_failure_refuses_admission_without_creating_a_match() {
        use bevy::image::{CompressedImageFormats, ImageLoader, ImagePlugin};
        let root = tempfile::tempdir().unwrap();
        let save = root.path().join("match.json");
        std::fs::write(root.path().join("broken.png"), b"not a PNG").unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: root.path().to_string_lossy().into_owned(),
                ..default()
            },
            ImagePlugin::default(),
        ))
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
        .init_resource::<Assets<Font>>()
        .init_resource::<bevy::text::FontCx>()
        .insert_resource(Controller(Play::saved(&save)));
        let handle: Handle<Image> = app.world().resource::<AssetServer>().load("broken.png");
        app.insert_resource(Artwork {
            verified: PreparedCollection {
                root: root.path().into(),
                images: vec![assets::VerifiedImage {
                    key: "broken.png".into(),
                    width: 1,
                    height: 1,
                    rgba: vec![0; 4],
                }],
                font: vec![],
            },
            handles: vec![handle],
            font: Handle::default(),
        })
        .insert_resource(Uploaded {
            ids: vec![],
            ready: Arc::new(AtomicBool::new(false)),
        })
        .add_systems(Update, admit_artwork);
        let start = std::time::Instant::now();
        while app.world().resource::<Controller>().0.admission() == &Admission::Loading
            && start.elapsed().as_secs() < 2
        {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(matches!(
            app.world().resource::<Controller>().0.admission(),
            Admission::Refused(_)
        ));
        assert_eq!(app.world().resource::<Controller>().0.observation(), None);
        assert!(!save.exists());
    }

    #[test]
    fn failed_opponent_persistence_is_visible_and_not_retried_by_native_frames() {
        let root = tempfile::tempdir().unwrap();
        let save = root.path().join("match.json");
        let mut app = interface_app();
        let mut play = Play::saved(&save);
        play.admit();
        app.insert_resource(Controller(play));
        app.update();
        activate(&mut app, Action::InstallPalisade);
        activate(&mut app, Action::TrashBran);
        let before = app.world().resource::<Controller>().0.observation();
        let bytes = std::fs::read(&save).unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join("match.json.lock"))
            .unwrap();
        lock.lock().unwrap();
        app.world_mut().resource_mut::<Interface>().respond_after = 0.;
        app.add_systems(Update, fixed_opponent.before(draw_interface));
        app.update();
        assert!(app.world().resource::<Interface>().error.contains("busy"));
        drop(lock);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<Controller>().0.observation(), before);
        assert_eq!(std::fs::read(&save).unwrap(), bytes);
    }

    #[test]
    #[ignore = "requires NETRUNNER_ART_ROOT, Vulkan and NETRUNNER_RENDER_CAPTURE; opens no desktop window"]
    fn actual_renderer_uploads_required_images_and_refuses_changed_pixels() {
        use bevy::{
            camera::RenderTarget,
            render::{
                render_resource::TextureFormat,
                view::screenshot::{Screenshot, ScreenshotCaptured},
            },
            window::ExitCondition,
        };
        let root = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
        let output =
            std::env::var_os("NETRUNNER_RENDER_CAPTURE").expect("NETRUNNER_RENDER_CAPTURE");
        let turns_save = std::env::var_os("NETRUNNER_TURNS_SAVE");
        let ordinary = turns_save.is_some();
        let verified = if ordinary {
            assets::inspect_turns(Path::new(&root))
        } else {
            assets::inspect(Path::new(&root))
        }
        .unwrap();
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    file_path: verified.root.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>()
                .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
        );
        if let Some(path) = turns_save {
            let seat = if std::env::var("NETRUNNER_SEAT").as_deref() == Ok("corp") {
                Player::Corp
            } else {
                Player::Runner
            };
            app.insert_resource(Controller(Play::load_turns(
                SaveOptions {
                    path: path.into(),
                    recover_from: None,
                },
                seat,
            )));
            app.world_mut().spawn((
                Window {
                    resolution: (1600, 1000).into(),
                    ..default()
                },
                PrimaryWindow,
            ));
        }
        install(&mut app, verified);
        let mut target =
            app.world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::new_target_texture(
                    1600,
                    1000,
                    TextureFormat::Rgba8UnormSrgb,
                    None,
                ));
        let camera_target = target.clone();
        app.add_systems(
            Startup,
            (move |mut commands: Commands, cameras: Query<(Entity, Has<Camera3d>)>| {
                for (entity, three_d) in &cameras {
                    commands
                        .entity(entity)
                        .insert(RenderTarget::Image(camera_target.clone().into()));
                    if three_d {
                        commands.entity(entity).insert(Camera {
                            viewport: Some(Viewport {
                                physical_size: if ordinary {
                                    UVec2::new(1600, 760)
                                } else {
                                    UVec2::new(1120, 1000)
                                },
                                ..default()
                            }),
                            ..default()
                        });
                    }
                }
            })
            .after(setup),
        );
        app.finish();
        app.cleanup();
        let start = std::time::Instant::now();
        while app.world().resource::<Controller>().0.admission() == &Admission::Loading
            && start.elapsed().as_secs() < 30
        {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        assert_eq!(
            app.world().resource::<Controller>().0.admission(),
            &Admission::Ready
        );
        for _ in 0..120 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        fn capture(app: &mut App, target: Handle<Image>, output: std::path::PathBuf) {
            assert!(!output.exists(), "choose a fresh capture path");
            let captured = Arc::new(AtomicBool::new(false));
            let finished = captured.clone();
            app.world_mut().spawn(Screenshot::image(target)).observe(
                move |capture: On<ScreenshotCaptured>| {
                    capture
                        .image
                        .clone()
                        .try_into_dynamic()
                        .unwrap()
                        .save(&output)
                        .unwrap();
                    finished.store(true, Ordering::Release);
                },
            );
            let start = std::time::Instant::now();
            while !captured.load(Ordering::Acquire) && start.elapsed().as_secs() < 20 {
                app.update();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            assert!(captured.load(Ordering::Acquire), "render capture completed");
        }
        let output = std::path::PathBuf::from(output);
        capture(&mut app, target.clone(), output.clone());
        let identity = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap()
            .corp_identity;
        app.world_mut().resource_mut::<Interface>().inspected = Some(identity);
        for _ in 0..30 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        capture(
            &mut app,
            target.clone(),
            output.with_extension("inspection.png"),
        );
        if ordinary {
            let mut defects = Vec::new();
            for (width, height) in [(2560, 1440), (1280, 900)] {
                target =
                    app.world_mut()
                        .resource_mut::<Assets<Image>>()
                        .add(Image::new_target_texture(
                            width,
                            height,
                            TextureFormat::Rgba8UnormSrgb,
                            None,
                        ));
                for mut camera_target in app
                    .world_mut()
                    .query::<&mut RenderTarget>()
                    .iter_mut(app.world_mut())
                {
                    *camera_target = RenderTarget::Image(target.clone().into());
                }
                app.world_mut()
                    .query_filtered::<&mut Window, With<PrimaryWindow>>()
                    .single_mut(app.world_mut())
                    .unwrap()
                    .resolution
                    .set(width as f32, height as f32);
                // Image-target asset changes reach the renderer one frame after
                // camera sizing; suspend draws while those attachments catch up.
                for mut camera in app
                    .world_mut()
                    .query::<&mut Camera>()
                    .iter_mut(app.world_mut())
                {
                    camera.is_active = false;
                }
                for _ in 0..3 {
                    app.update();
                }
                for mut camera in app
                    .world_mut()
                    .query::<&mut Camera>()
                    .iter_mut(app.world_mut())
                {
                    camera.is_active = true;
                }
                for _ in 0..8 {
                    app.update();
                }
                let world = app.world_mut();
                let (image, image_at, parent) = world
                    .query_filtered::<(&ComputedNode, &UiGlobalTransform, &ChildOf), With<ImageNode>>()
                    .single(world)
                    .unwrap();
                let panel = world.get::<ComputedNode>(parent.parent()).unwrap();
                let panel_at = world.get::<UiGlobalTransform>(parent.parent()).unwrap();
                let image_edge =
                    (image_at.translation - panel_at.translation).abs() + image.size() / 2.;
                let content_edge = panel.size() / 2. - Vec2::splat(30.);
                if image_edge.x > content_edge.x + 1. || image_edge.y > content_edge.y + 1. {
                    defects.push(format!("inspection at {width}x{height}: image edge {image_edge} exceeds panel content {content_edge}"));
                }
                assert!((image.size().x / image.size().y - 746. / 1033.).abs() < 0.01);
                capture(
                    &mut app,
                    target.clone(),
                    output.with_extension(format!("inspection-{width}.png")),
                );
            }
            app.world_mut().resource_mut::<Interface>().inspected = None;
            for _ in 0..8 {
                app.update();
            }
            let world = app.world_mut();
            let (token, offset) = world
                .query::<&count_label::Label>()
                .iter(world)
                .map(|label| (label.token, label.offset))
                .next()
                .unwrap();
            let (camera, camera_at) = world
                .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
                .single(world)
                .unwrap();
            let ray = camera
                .viewport_to_world(camera_at, Vec2::new(640., 750.))
                .unwrap();
            let distance = ray
                .intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                .unwrap();
            world.get_mut::<Transform>(token).unwrap().translation =
                ray.get_point(distance) - offset;
            for _ in 0..8 {
                app.update();
            }
            let world = app.world_mut();
            let dock = world
                .query_filtered::<(&Node, &bevy::ui::ComputedStackIndex), With<InterfaceRoot>>()
                .iter(world)
                .find(|(node, _)| node.top == percent(76.))
                .unwrap()
                .1
                .0;
            let (label_node, label_stack) = world
                .query::<(&count_label::Label, &Node, &bevy::ui::ComputedStackIndex)>()
                .iter(world)
                .find(|(label, _, _)| label.token == token)
                .map(|(_, node, stack)| (node, stack.0))
                .unwrap();
            assert!(matches!(label_node.top, Val::Px(y) if (749.0..751.0).contains(&y)));
            if label_stack >= dock {
                defects.push(format!(
                    "token label over action panel: label stack {label_stack}, dock stack {dock}"
                ));
            }
            capture(
                &mut app,
                target.clone(),
                output.with_extension("label-under-dock.png"),
            );
            assert!(defects.is_empty(), "{}", defects.join("\n"));
        }
        app.world_mut().resource_mut::<Interface>().inspected = None;
        app.update();
        if !ordinary {
            activate(&mut app, Action::InstallPalisade);
            activate(&mut app, Action::TrashBran);
            let start = std::time::Instant::now();
            while app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap()
                .decision
                .is_none()
                && start.elapsed().as_secs() < 5
            {
                app.update();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            // Inspect the actual Corp back while the human's rez choice is pending.
            for _ in 0..30 {
                app.update();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            capture(
                &mut app,
                target.clone(),
                output.with_extension("unrezzed.png"),
            );
            activate(&mut app, Action::RezPalisade);
            assert_eq!(
                app.world()
                    .resource::<Controller>()
                    .0
                    .observation()
                    .unwrap()
                    .run_result,
                Some(crate::game::RunResult::EndedByIce)
            );
            // Give the new board and font atlases time to reach the render world.
            for _ in 0..30 {
                app.update();
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            capture(&mut app, target, output.with_extension("ended.png"));
            app.world_mut()
                .resource_mut::<Controller>()
                .0
                .restart()
                .unwrap();
            app.update();
        }
        let before = app.world().resource::<Controller>().0.observation();
        let handle = app.world().resource::<Artwork>().image("tokens/credit.png");
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .get_mut(&handle)
            .unwrap()
            .data
            .as_mut()
            .unwrap()[0] ^= 1;
        app.update();
        assert!(matches!(
            app.world().resource::<Controller>().0.admission(),
            Admission::Refused(_)
        ));
        assert_eq!(app.world().resource::<Controller>().0.observation(), before);
    }
}
