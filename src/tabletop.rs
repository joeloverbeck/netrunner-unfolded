//! Bevy consumer of the same match/controller used by headless checks.
use crate::{
    assets::{self, PreparedCollection},
    game::{Action, CardView, Observation},
    play::{Admission, Play},
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
use unfolded_tabletop::{card, token};

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
    CloseInspection,
    History,
    Restart,
    Quit,
}

pub fn run(root: &Path) -> Result<(), String> {
    let verified = assets::inspect(root).map_err(|error| error.to_string())?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: verified.root.to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Netrunner Unfolded · Brân at HQ".into(),
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
    app.add_plugins(MeshPickingPlugin)
        .insert_resource(ClearColor(Color::srgb(0.025, 0.04, 0.055)))
        .init_resource::<Controller>()
        .init_resource::<Interface>();
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
    mut cameras: Query<&mut Camera, With<Camera3d>>,
) {
    for mut camera in &mut cameras {
        camera.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(
                (window.physical_width() as f32 * 0.70).max(1.) as u32,
                window.physical_height().max(1),
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
                    .unwrap_or("")
                    .into();
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::Restart if ui.inspected.is_none() && !ui.history_open => {
                ui.error = controller.0.restart().err().unwrap_or("").into();
            }
            _ => {}
        }
    }
}

fn fixed_opponent(mut controller: ResMut<Controller>, ui: Res<Interface>, time: Res<Time>) {
    if controller.0.admission() == &Admission::Ready
        && ui.inspected.is_none()
        && !ui.history_open
        && time.elapsed_secs_f64() >= ui.respond_after
    {
        let _ = controller.0.respond();
    }
}

#[derive(Clone, PartialEq)]
struct Drawn {
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
) {
    let current = Drawn {
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
    if ready && let Some(view) = &current.observation {
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
    if ready && current.history_open {
        commands
            .spawn((
                InterfaceRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(70.),
                    height: percent(100.),
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
        commands
            .spawn((
                InterfaceRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(70.),
                    height: percent(100.),
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
                    .unwrap_or_else(|| "backs/corp.png".into());
                modal.spawn((
                    ImageNode::new(art.image(&key)),
                    Node {
                        width: percent(52.),
                        aspect_ratio: Some(746. / 1033.),
                        ..default()
                    },
                ));
                modal
                    .spawn(Node {
                        width: percent(40.),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(20.),
                        ..default()
                    })
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
            BackgroundColor(Color::srgb(0.12, 0.25, 0.30)),
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
    let key = card_view
        .printing
        .filter(|_| face_up)
        .map(|code| format!("cards/{code}.png"))
        .unwrap_or_else(|| "backs/corp.png".into());
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
        ];
        app.add_plugins(MinimalPlugins)
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
        .init_resource::<Controller>();
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
        let verified = assets::inspect(Path::new(&root)).unwrap();
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
        install(&mut app, verified);
        let target =
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
                                physical_size: UVec2::new(1120, 1000),
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
