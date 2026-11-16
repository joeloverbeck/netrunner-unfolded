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
use unfolded_tabletop::{card, label_anchor, token};
pub mod seat;
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
    inspected_pile: Option<PileInspection>,
    history_open: bool,
    new_match_confirmation: Option<u64>,
    error: String,
    undo_feedback: bool,
    respond_after: f64,
}

#[derive(Clone, PartialEq)]
struct PileInspection {
    name: &'static str,
    cards: Vec<CardView>,
    index: usize,
}

#[derive(Component)]
struct BoardPiece;
#[derive(Component)]
struct InterfaceRoot;
#[derive(Component)]
struct ActionDock;
#[derive(Component)]
struct InspectionImage;
#[derive(Component, Clone, Copy)]
enum ButtonCommand {
    Choose(u64, Action),
    ChooseInspected(u64, Action),
    CloseInspection,
    PreviousCard,
    NextCard,
    History,
    Undo(u64),
    NewMatch(u64),
    ConfirmNewMatch(u64),
    CancelNewMatch,
    SelectSeat(Player),
    Quit,
}

fn phase_status(phase: &str, undo_feedback: Option<&str>) -> String {
    undo_feedback.map_or_else(|| phase.into(), |reason| format!("{phase} · {reason}"))
}

pub fn run(root: &Path) -> Result<(), String> {
    run_with_save(root, None)
}

pub fn run_with_save(root: &Path, save: Option<SaveOptions>) -> Result<(), String> {
    run_introductory_with_save(root, save, None)
}

pub fn run_introductory_with_save(
    root: &Path,
    save: Option<SaveOptions>,
    human: Option<Player>,
) -> Result<(), String> {
    let verified = assets::inspect_turns(root).map_err(|error| error.to_string())?;
    run_selected(verified, Play::introductory(save, human)?)
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
    app.add_plugins((MeshPickingPlugin, label_anchor::LabelAnchorPlugin))
        .insert_resource(ClearColor(Color::srgb(0.025, 0.04, 0.055)))
        .init_resource::<Controller>()
        .init_resource::<Interface>()
        .insert_resource(unfolded_tabletop::camera::Orbit::new((Vec3::ZERO, 27.)));
    turns::table_camera(app);
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
            turns::camera_facts_and_framing,
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
        unfolded_tabletop::camera::TableCamera,
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
                (if turns::is_ordinary(&controller) {
                    (window.height() - turns::dock_height(window.height())) * window.scale_factor()
                } else {
                    window.physical_height() as f32
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
            ButtonCommand::SelectSeat(human)
                if ui.inspected.is_none()
                    && !ui.history_open
                    && ui.new_match_confirmation.is_none() =>
            {
                ui.error = controller.0.select_seat(human).err().unwrap_or_default();
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::CloseInspection => {
                ui.inspected = None;
                ui.inspected_owner = None;
                ui.inspected_pile = None;
                ui.history_open = false;
            }
            ButtonCommand::PreviousCard | ButtonCommand::NextCard => {
                if let Some(pile) = &mut ui.inspected_pile {
                    let next = match command {
                        ButtonCommand::PreviousCard => pile.index.checked_sub(1),
                        _ => pile.index.checked_add(1),
                    };
                    if let Some(index) = next.filter(|index| *index < pile.cards.len()) {
                        pile.index = index;
                        ui.inspected = Some(pile.cards[index].clone());
                    }
                }
            }
            ButtonCommand::History
                if ui.inspected.is_none() && ui.new_match_confirmation.is_none() =>
            {
                ui.history_open = true
            }
            ButtonCommand::Choose(revision, action)
                if ui.inspected.is_none()
                    && !ui.history_open
                    && ui.new_match_confirmation.is_none() =>
            {
                ui.undo_feedback = false;
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
                ui.undo_feedback = false;
                ui.error = controller
                    .0
                    .choose(revision, action)
                    .err()
                    .unwrap_or_default();
                if ui.error.is_empty() {
                    ui.inspected = None;
                    ui.inspected_owner = None;
                    ui.inspected_pile = None;
                }
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::NewMatch(revision)
                if ui.inspected.is_none()
                    && !ui.history_open
                    && ui.new_match_confirmation.is_none() =>
            {
                ui.undo_feedback = false;
                ui.error = if controller.0.revision() != Some(revision)
                    || !controller.0.can_start_new_match()
                {
                    "The match changed. Observe the current table before starting another match."
                        .into()
                } else {
                    controller.0.cancel_opponent();
                    ui.new_match_confirmation = Some(revision);
                    String::new()
                };
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::ConfirmNewMatch(revision)
                if ui.new_match_confirmation == Some(revision) =>
            {
                ui.error = if controller.0.revision() != Some(revision) {
                    "The match changed. Return to the current table before starting another match."
                        .into()
                } else {
                    controller.0.begin_new_match().err().unwrap_or_default()
                };
                ui.new_match_confirmation = None;
            }
            ButtonCommand::CancelNewMatch => {
                ui.error = if ui.new_match_confirmation.take().is_some() {
                    String::new()
                } else {
                    controller.0.cancel_new_match().err().unwrap_or_default()
                };
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            ButtonCommand::Undo(revision)
                if ui.inspected.is_none()
                    && !ui.history_open
                    && ui.new_match_confirmation.is_none() =>
            {
                let available = controller.0.undo_reason().is_none();
                ui.undo_feedback = !available;
                ui.error = controller
                    .0
                    .undo(revision)
                    .err()
                    .filter(|_| available)
                    .unwrap_or_default();
                // No previously scheduled opponent computation survives correction.
                ui.respond_after = time.elapsed_secs_f64() + 0.9;
            }
            _ => {}
        }
    }
}

fn fixed_opponent(mut controller: ResMut<Controller>, mut ui: ResMut<Interface>, time: Res<Time>) {
    let allow_start = controller.0.admission() == &Admission::Ready
        && controller.0.opponent_pending()
        && ui.inspected.is_none()
        && !ui.history_open
        && ui.new_match_confirmation.is_none()
        && ui.error.is_empty()
        && time.elapsed_secs_f64() >= ui.respond_after;
    if let Err(error) = controller.0.poll_opponent(allow_start) {
        ui.error = error;
    }
}

#[derive(Clone, PartialEq)]
struct Drawn {
    undo: (u64, Option<&'static str>),
    undo_feedback: bool,
    viewport: Vec2,
    observation: Option<Observation>,
    admission: Admission,
    inspected: Option<CardView>,
    inspected_pile: Option<PileInspection>,
    history_open: bool,
    new_match_confirmation: Option<u64>,
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
    mut images: ResMut<Assets<Image>>,
    mut printed: Local<unfolded_tabletop::printed::Atlas>,
    ui_scale: Option<Res<UiScale>>,
    previous: Query<Entity, Or<(With<BoardPiece>, With<InterfaceRoot>)>>,
    mut drawn: Local<Option<Drawn>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let current = Drawn {
        undo_feedback: ui.undo_feedback,
        undo: (
            controller.0.revision().unwrap_or(0),
            controller.0.undo_reason(),
        ),
        viewport: window
            .as_ref()
            .map_or(Vec2::new(1600., 1000.), |window| window.size()),
        observation: controller.0.observation(),
        admission: controller.0.admission().clone(),
        inspected: ui.inspected.clone(),
        inspected_pile: ui.inspected_pile.clone(),
        history_open: ui.history_open,
        new_match_confirmation: ui.new_match_confirmation,
        error: ui.error.clone(),
    };
    if drawn.as_ref() == Some(&current) {
        return;
    }
    for entity in &previous {
        commands.entity(entity).despawn();
    }
    let ready = current.admission == Admission::Ready;
    let ordinary = turns::is_ordinary(&controller);
    let mut captions = Vec::new();
    if controller.0.needs_seat() {
        turns::draw_role_selection(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut captions,
            controller.0.can_return_to_match(),
        );
    }
    if ready && let Some(view) = &current.observation {
        turns::draw_board(
            &mut commands,
            &mut meshes,
            &mut materials,
            &art,
            &mut captions,
            view,
            controller.0.human(),
        );
        if let Some(revision) = current.new_match_confirmation {
            turns::draw_new_match_confirmation(
                &mut commands,
                revision,
                controller.0.has_saved_match(),
            );
        } else {
            turns::draw_dock(
                &mut commands,
                &art,
                view,
                &ui,
                controller.0.human(),
                current.undo,
                controller.0.can_start_new_match(),
                controller.0.archived_match(),
            );
        }
    }
    if !captions.is_empty() {
        turns::print_mat_labels(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            &mut printed,
            captions,
            ui_scale.as_ref().map_or(1., |scale| scale.0),
        );
    }
    if !ordinary {
        commands.spawn((InterfaceRoot, ScrollArea, Node { position_type: PositionType::Absolute, left: percent(70.), top: px(0.), width: percent(30.), height: percent(100.), padding: UiRect::all(px(22.)), flex_direction: FlexDirection::Column, row_gap: px(14.), overflow: Overflow::scroll_y(), ..default() }, BackgroundColor(Color::srgb(0.045, 0.065, 0.085)))).with_children(|panel| {
        label(panel, "NETRUNNER UNFOLDED", 20., Color::srgb(0.55, 0.85, 0.88));
        label(panel, "System Gateway", 32., Color::WHITE);
        match &current.admission {
            Admission::Loading => label(panel, "Checking required artwork and renderer...", 21., Color::WHITE),
            Admission::Refused(reason) => {
                label(panel, "Play stopped", 28., Color::srgb(1., 0.5, 0.4));
                label(panel, reason, 19., Color::WHITE);
                label(panel, &format!("Artwork folder: {}\nRepair or prepare the collection, then restart.", art.verified.root.display()), 17., Color::WHITE);
            }
            Admission::Ready => {}
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
                    InspectionImage,
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
                        if let Some(pile) = &current.inspected_pile {
                            label(
                                text,
                                &format!(
                                    "{} · card {} of {}",
                                    pile.name,
                                    pile.index + 1,
                                    pile.cards.len()
                                ),
                                19.,
                                Color::WHITE,
                            );
                            if pile.index > 0 {
                                button(text, "Previous card", ButtonCommand::PreviousCard);
                            }
                            if pile.index + 1 < pile.cards.len() {
                                button(text, "Next card", ButtonCommand::NextCard);
                            }
                        }
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

const CARD_THICKNESS: f32 = 0.06;

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
    let body = meshes.add(card::body(1.8, CARD_THICKNESS, 2.5));
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
                    && ui.new_match_confirmation.is_none()
                {
                    ui.inspected = Some(card_view.clone());
                    ui.inspected_owner = Some(owner);
                    ui.inspected_pile = None;
                }
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use unfolded_tabletop::label_anchor::{Anchor, LabelAnchor};

    /// The token and offset a counter's label is kept over; a board label has none.
    fn counter_anchor(label: &LabelAnchor) -> Option<(Entity, Vec3)> {
        match label.to {
            Anchor::Entity { entity, offset } => Some((entity, offset)),
            Anchor::Point(_) => None,
        }
    }

    /// A board label's left edge, not its middle, sits at its table point, and the
    /// label is hidden while that point is outside the table camera's viewport.
    #[test]
    fn board_labels_start_at_their_point_and_hide_outside_the_viewport() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        use bevy::camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo};
        let mut position = EngineeringPosition::economy(3);
        let mut office =
            EngineeringCard::new(CardKind::OffworldOffice, Zone::Root(Server::Remote(1)));
        office.installed_turn = Some(1);
        office.advancement = 2;
        position.cards.push(office);
        let mut play = Play::engineering(position, None, Player::Corp);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        let (label, anchor) = world
            .query::<(Entity, &Text, &LabelAnchor)>()
            .iter(world)
            .find(|(_, text, _)| text.0 == "Advancement")
            .map(|(entity, _, anchor)| (entity, *anchor))
            .expect("an advanced card has an Advancement board label");
        let Anchor::Point(at) = anchor.to else {
            panic!("a board label is kept over a table point");
        };
        // Measured by layout as 40 logical pixels wide, so centering would put
        // its left edge 20 pixels before the point.
        world.entity_mut(label).insert(ComputedNode {
            size: Vec2::new(80., 24.),
            inverse_scale_factor: 0.5,
            ..default()
        });
        // A 1000×500 logical target with a 90° field of view, 10 units in front
        // of the point: a point (dx, dy) off the camera's axis lands at
        // (500 + 25dx, 250 - 25dy).
        let camera = world
            .spawn((
                Camera3d::default(),
                Camera {
                    computed: ComputedCameraValues {
                        clip_from_view: PerspectiveProjection {
                            fov: std::f32::consts::FRAC_PI_2,
                            aspect_ratio: 2.,
                            near: 0.1,
                            ..default()
                        }
                        .get_clip_from_view(),
                        target_info: Some(RenderTargetInfo {
                            physical_size: UVec2::new(2000, 1000),
                            scale_factor: 2.,
                        }),
                        ..default()
                    },
                    ..default()
                },
                GlobalTransform::from_translation(at + Vec3::Z * 10.),
            ))
            .id();
        app.update();
        let node = app.world().get::<Node>(label).unwrap();
        let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
            panic!("the board label was not placed");
        };
        assert!((left - 500.).abs() < 0.01, "left edge at {left}, not 500");
        assert!((top - 250.).abs() < 0.01, "top at {top}, not 250");
        assert_eq!(
            *app.world().get::<Visibility>(label).unwrap(),
            Visibility::Visible
        );

        // The point now lands at x 1100: in front of the camera, right of the
        // 1000-pixel viewport.
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::from_translation(
                at + Vec3::new(-24., 0., 10.),
            ));
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(label).unwrap(),
            Visibility::Hidden
        );
    }

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
        interface_app_with_text(false)
    }

    fn interface_app_with_text(text_layout: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        if text_layout {
            app.add_plugins((
                AssetPlugin::default(),
                bevy::image::ImagePlugin::default(),
                bevy::text::TextPlugin,
                bevy::transform::TransformPlugin,
                bevy::camera::visibility::VisibilityPlugin,
                bevy::input::InputPlugin,
                bevy::input_focus::InputFocusPlugin,
                bevy::input_focus::InputDispatchPlugin,
                bevy::picking::PickingPlugin,
                bevy::picking::InteractionPlugin,
                bevy::ui::UiPlugin,
            ));
            app.init_resource::<Assets<TextureAtlasLayout>>();
            app.init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>();
        }
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let font = fonts.add(Font::from_bytes(if text_layout {
            bevy::text::DEFAULT_FONT_DATA.to_vec()
        } else {
            vec![]
        }));
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
            "cards/30015.png",
            "cards/30032.png",
            "cards/30012.png",
            "cards/30014.png",
            "cards/30018.png",
            "cards/30027.png",
            "cards/30033.png",
            "cards/30020.png",
            "cards/30021.png",
            "cards/30034.png",
            "cards/30037.png",
            "cards/30071.png",
            "cards/30029.png",
            "cards/30074.png",
            "cards/30030.png",
            "cards/30040.png",
            "cards/30064.png",
            "cards/30067.png",
            "cards/30028.png",
            "cards/30069.png",
            "cards/30070.png",
            "cards/30075.png",
            "cards/30042.png",
            "cards/30045.png",
            "cards/30046.png",
            "cards/30047.png",
            "cards/30073.png",
        ];
        app.add_plugins(label_anchor::LabelAnchorPlugin)
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
    fn selected_open_table_setup_uses_native_role_and_mulligan_controls() {
        let mut play = Play::introductory_seeded(42, None, Player::Corp).with_role_selection();
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query::<&Name>()
                .iter(world)
                .filter(|name| name.as_str().ends_with("playmat"))
                .count(),
            2
        );
        let role = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::SelectSeat(Player::Runner)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate { entity: role });
        app.update();
        app.world_mut()
            .resource_mut::<Controller>()
            .0
            .respond()
            .unwrap();
        app.update();
        let world = app.world_mut();
        let play = &world.resource::<Controller>().0;
        assert_eq!(play.human(), Player::Runner);
        assert_eq!(play.observation().unwrap().hand.len(), 5);
        let mulligan = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::Choose(_, Action::Mulligan)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate { entity: mulligan });
        app.update();
        let play = &app.world().resource::<Controller>().0;
        assert_eq!(play.observation().unwrap().turn_number, 1);
        assert!(play.undo_reason().unwrap().contains("information"));
    }

    #[test]
    fn unsaved_new_match_confirmation_does_not_promise_an_archive() {
        let mut play = Play::introductory_seeded(42, None, Player::Corp);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        let entity = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::NewMatch(_)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate { entity });
        app.update();
        let world = app.world_mut();
        let text = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("not saved") && text.contains("discard"),
            "{text}"
        );
        assert!(!text.contains("is archived"));
    }

    #[test]
    fn live_new_match_confirmation_and_role_selection_preserve_cancelled_game() {
        fn press(app: &mut App, caption: &str) {
            let world = app.world_mut();
            let entity = world
                .query::<(&Text, &ChildOf)>()
                .iter(world)
                .find_map(|(text, child)| (text.0 == caption).then_some(child.parent()))
                .unwrap();
            world.trigger(Activate { entity });
            app.update();
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("match.json");
        let mut play = Play::introductory_seeded(
            42,
            Some(SaveOptions {
                path: path.clone(),
                recover_from: None,
            }),
            Player::Corp,
        );
        play.admit();
        let before = play.observation().unwrap();
        let saved = std::fs::read(&path).unwrap();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        press(&mut app, "New match");
        assert!(
            !app.world().resource::<Controller>().0.needs_seat(),
            "New match must ask for confirmation first"
        );
        press(&mut app, "Cancel");
        assert_eq!(
            app.world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap(),
            before
        );
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        press(&mut app, "New match");
        press(&mut app, "Choose new seat");
        assert!(app.world().resource::<Controller>().0.needs_seat());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        press(&mut app, "Return to match");
        assert_eq!(
            app.world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap(),
            before
        );
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }

    #[test]
    fn completed_introductory_table_starts_another_match_through_role_selection() {
        let mut play = Play::introductory_seeded(42, None, Player::Corp);
        play.admit();
        for _ in 0..4000 {
            let view = play.observation().unwrap();
            if view.match_winner.is_some() {
                break;
            }
            if let Some(decision) = &view.decision {
                let action = crate::game::baseline_response(&view).unwrap();
                play.choose(decision.revision, action).unwrap();
            } else {
                play.respond().unwrap();
            }
        }
        assert!(play.can_start_new_match());
        let previous_revision = play.revision().unwrap();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        let new_match = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::NewMatch(_)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate { entity: new_match });
        app.update();
        let world = app.world_mut();
        let confirmation = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::ConfirmNewMatch(_)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate {
            entity: confirmation,
        });
        app.update();
        let world = app.world_mut();
        assert!(world.resource::<Controller>().0.needs_seat());
        let role = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .find_map(|(entity, command)| {
                matches!(command, ButtonCommand::SelectSeat(Player::Corp)).then_some(entity)
            })
            .unwrap();
        world.trigger(Activate { entity: role });
        app.update();
        let play = &app.world().resource::<Controller>().0;
        assert_eq!(play.observation().unwrap().turn_number, 0);
        assert_eq!(play.observation().unwrap().hand.len(), 5);
        assert!(play.revision().unwrap() > previous_revision);
    }

    #[test]
    fn completed_matches_hide_run_continuation_notices_but_live_matches_keep_them() {
        use crate::game::{Match, RunResult};
        for (result, notice) in [
            (
                RunResult::SuccessfulAccess,
                "Last run completed: successful access. Continue the turn below.",
            ),
            (RunResult::EndedByIce, "ICE ended the run."),
            (
                RunResult::EndedByManegarm,
                "Manegarm ended the run because payment was declined. No Manegarm credits or clicks were paid.",
            ),
        ] {
            for complete in [false, true] {
                // The presentation consumes public observations. Vary only the
                // prior run result and terminal status at that boundary.
                let mut view = Match::introductory(42).observe(Player::Runner);
                view.turn_number = 1;
                view.active_player = Some(Player::Corp);
                view.decision = None;
                view.run_result = Some(result);
                view.phase = if complete {
                    "Match complete"
                } else {
                    "Action phase"
                };
                view.match_winner = complete.then_some(Player::Corp);
                view.terminal_reason = complete.then_some("Agenda victory");
                let art = interface_app()
                    .world_mut()
                    .remove_resource::<Artwork>()
                    .unwrap();
                let mut app = App::new();
                app.add_plugins(MinimalPlugins)
                    .insert_resource(art)
                    .add_systems(Startup, move |mut commands: Commands, art: Res<Artwork>| {
                        turns::draw_dock(
                            &mut commands,
                            &art,
                            &view,
                            &Interface::default(),
                            Player::Runner,
                            (0, None),
                            complete,
                            None,
                        );
                    });
                app.update();
                let world = app.world_mut();
                let text: Vec<_> = world
                    .query::<&Text>()
                    .iter(world)
                    .map(|text| text.0.as_str())
                    .collect();
                assert_eq!(
                    text.contains(&notice),
                    !complete,
                    "{result:?}, complete={complete}"
                );
                assert_eq!(text.contains(&"Corp wins · Agenda victory"), complete);
            }
        }
    }

    #[test]
    fn verbal_plasticity_uses_the_resource_area_without_a_credit_counter() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Zone};
        let mut position = EngineeringPosition::economy(20);
        let mut card = EngineeringCard::new(CardKind::VerbalPlasticity, Zone::Rig);
        card.installed_turn = Some(1);
        position.cards = vec![card];
        position.active = Player::Runner;
        let mut play = Play::engineering(position, None, Player::Runner);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == "Resources")
        );
        assert_eq!(
            world
                .query::<&Name>()
                .iter(world)
                .filter(|name| name.as_str() == "Verbal Plasticity")
                .count(),
            1,
            "the resource is not also drawn among programs"
        );
        assert!(
            !world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Verbal Plasticity credits")
        );
    }

    #[test]
    fn ordinary_piles_are_aligned_thin_and_grow_with_the_observed_count() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Zone};

        for seat in [Player::Corp, Player::Runner] {
            for (count, height) in [(0, 0.), (8, 0.052), (1, 0.0065), (2, 0.013), (34, 0.221)] {
                for (name, zone, kind, texture) in [
                    (
                        "Archives pile",
                        Zone::Archives,
                        CardKind::Bran,
                        "backs/corp.png",
                    ),
                    (
                        "Heap pile",
                        Zone::Heap,
                        CardKind::SureGamble,
                        "cards/30030.png",
                    ),
                    ("R&D pile", Zone::Rd, CardKind::Bran, "backs/corp.png"),
                    (
                        "Stack pile",
                        Zone::Stack,
                        CardKind::SureGamble,
                        "backs/runner.png",
                    ),
                ] {
                    let mut position = EngineeringPosition::economy(3);
                    position.cards = (0..count)
                        .map(|_| EngineeringCard::new(kind, zone))
                        .collect();
                    let mut play = Play::engineering(position, None, seat);
                    play.admit();
                    assert_eq!(play.admission(), &Admission::Ready);
                    let before = play.observation().unwrap();
                    let mut app = interface_app();
                    app.insert_resource(Controller(play));
                    app.update();
                    let world = app.world();
                    let piles: Vec<_> = world
                        .iter_entities()
                        .filter(|entity| {
                            entity.get::<Name>().is_some_and(|actual| {
                                [name, "Facedown card", "Brân 1.0", "Sure Gamble"]
                                    .contains(&actual.as_str())
                            })
                        })
                        .collect();
                    if count == 0 {
                        assert!(piles.is_empty(), "{name}: empty pile draws nothing");
                    } else {
                        let (min, max) = piles
                            .iter()
                            .flat_map(|pile| {
                                let mesh = world
                                    .resource::<Assets<Mesh>>()
                                    .get(&pile.get::<Mesh3d>().unwrap().0)
                                    .unwrap();
                                let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                                else {
                                    panic!("mesh positions")
                                };
                                let pose = pile.get::<Transform>().unwrap();
                                vertices
                                    .iter()
                                    .map(|p| pose.transform_point(Vec3::from_array(*p)))
                            })
                            .fold(
                                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                                |(min, max), p| (min.min(p), max.max(p)),
                            );
                        assert!(
                            (max.y - min.y - height).abs() < 0.00001,
                            "{name}: count {count} height {}",
                            max.y - min.y
                        );
                        assert!((max.x - min.x - 1.8).abs() < 0.00001, "{name}: no fan");
                        assert!(
                            (max.z - min.z - 2.5).abs() < 0.00001,
                            "{name}: aligned footprint"
                        );
                        assert!(
                            (min.y - 0.05).abs() < 0.00001,
                            "{name}: preserve table clearance"
                        );
                        assert_eq!(piles.len(), 1, "{name}: one compact body");
                        let pile = &piles[0];
                        let pose = pile.get::<Transform>().unwrap();
                        let children = pile.get::<Children>().unwrap();
                        assert_eq!(children.len(), 1, "one visible top");
                        let top = children[0];
                        let material = world
                            .resource::<Assets<StandardMaterial>>()
                            .get(
                                &world
                                    .get::<MeshMaterial3d<StandardMaterial>>(top)
                                    .unwrap()
                                    .0,
                            )
                            .unwrap();
                        assert_eq!(
                            material.base_color_texture,
                            Some(world.resource::<Artwork>().image(texture))
                        );
                        let top_y =
                            pose.translation.y + world.get::<Transform>(top).unwrap().translation.y;
                        assert!((top_y - max.y - 0.002).abs() < 0.00001, "face above body");
                    }
                    assert_eq!(world.resource::<Controller>().0.observation(), Some(before));
                    if count > 0 && matches!(zone, Zone::Rd | Zone::Stack) {
                        click_piece(&mut app, name);
                        assert!(app.world().resource::<Interface>().inspected.is_none());
                    }
                }
            }
        }
    }

    fn click_piece(app: &mut App, name: &str) {
        use bevy::{
            camera::RenderTarget,
            picking::{
                backend::HitData,
                events::Pointer,
                pointer::{Location, PointerButton, PointerId},
            },
            window::WindowRef,
        };
        let world = app.world_mut();
        let entity = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, actual)| actual.as_str() == name)
            .unwrap()
            .0;
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
        app.update();
    }

    fn inspection_button(app: &mut App, label: &str) -> Option<Entity> {
        let world = app.world_mut();
        world
            .query::<(Entity, &ButtonCommand, &Children)>()
            .iter(world)
            .find_map(|(entity, _, children)| {
                children
                    .iter()
                    .any(|child| world.get::<Text>(child).is_some_and(|text| text.0 == label))
                    .then_some(entity)
            })
    }

    #[test]
    fn compact_discard_inspection_browses_every_lawful_card_without_changing_the_game() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Zone};
        for seat in [Player::Corp, Player::Runner] {
            for top_faceup in [false, true] {
                let mut position = EngineeringPosition::economy(3);
                position.cards.clear();
                for _ in 0..3 {
                    position
                        .cards
                        .push(EngineeringCard::new(CardKind::Bran, Zone::Archives));
                    position
                        .cards
                        .push(EngineeringCard::new(CardKind::SureGamble, Zone::Heap));
                }
                // Exercise admission of a coherent saved Archives mixture.
                // Rezzed is an installed-card property, not a discard's face.
                let game = crate::game::Match::engineering_turns(position).unwrap();
                let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
                checkpoint["state"]["cards"][0]["faceup"] = true.into();
                checkpoint["state"]["cards"][4]["faceup"] = top_faceup.into();
                let game = crate::game::Match::resume(serde_json::from_value(checkpoint).unwrap())
                    .unwrap();
                let directory = tempfile::tempdir().unwrap();
                let save = SaveOptions {
                    path: directory.path().join("mixture.json"),
                    recover_from: None,
                };
                drop(crate::saved_state::Record::open(&save, game).unwrap());
                let mut play = Play::load_turns(save, seat);
                play.admit();
                let before = play.observation().unwrap();
                let mut app = interface_app();
                app.insert_resource(Controller(play));
                app.update();
                for (name, cards) in [("Archives", &before.archives), ("Heap", &before.heap)] {
                    // Actual pile click, then the displayed native navigation buttons.
                    click_piece(&mut app, &format!("{name} pile"));
                    for (index, expected) in cards.iter().rev().enumerate() {
                        assert_eq!(
                            app.world().resource::<Interface>().inspected.as_ref(),
                            Some(expected)
                        );
                        assert!(
                            app.world_mut()
                                .query::<&Text>()
                                .iter(app.world())
                                .any(|text| text.0 == format!("{name} · card {} of 3", index + 1))
                        );
                        assert_eq!(
                            inspection_button(&mut app, "Previous card").is_some(),
                            index > 0
                        );
                        let next = inspection_button(&mut app, "Next card");
                        assert_eq!(next.is_some(), index < 2);
                        if let Some(entity) = next {
                            app.world_mut().trigger(Activate { entity });
                            app.update();
                        }
                    }
                    for expected in cards.iter().skip(1) {
                        let entity = inspection_button(&mut app, "Previous card").unwrap();
                        app.world_mut().trigger(Activate { entity });
                        app.update();
                        assert_eq!(
                            app.world().resource::<Interface>().inspected.as_ref(),
                            Some(expected)
                        );
                    }
                    assert!(inspection_button(&mut app, "Previous card").is_none());
                    let entity = inspection_button(&mut app, "Close inspection").unwrap();
                    app.world_mut().trigger(Activate { entity });
                    app.update();
                    assert!(app.world().resource::<Interface>().inspected.is_none());
                    assert_eq!(
                        app.world().resource::<Controller>().0.observation(),
                        Some(before.clone())
                    );
                }
                if seat == Player::Runner {
                    let hidden = &before.archives[1];
                    assert!(
                        hidden.printing.is_none()
                            && hidden.instance.is_none()
                            && hidden.title.is_none()
                            && hidden.rules_text.is_none()
                    );
                }
                let world = app.world();
                let pile = world
                    .iter_entities()
                    .find(|entity| {
                        entity
                            .get::<Name>()
                            .is_some_and(|name| name.as_str() == "Archives pile")
                    })
                    .unwrap();
                let top = pile.get::<Children>().unwrap()[0];
                let material = world
                    .resource::<Assets<StandardMaterial>>()
                    .get(
                        &world
                            .get::<MeshMaterial3d<StandardMaterial>>(top)
                            .unwrap()
                            .0,
                    )
                    .unwrap();
                assert_eq!(
                    material.base_color_texture,
                    Some(world.resource::<Artwork>().image(if top_faceup {
                        "cards/30039.png"
                    } else {
                        "backs/corp.png"
                    }))
                );
            }
        }
    }

    #[test]
    fn ordinary_zone_counts_are_grouped_on_fixed_non_pickable_mat_ink() {
        use crate::game::EngineeringPosition;
        for seat in [Player::Corp, Player::Runner] {
            let mut app = interface_app();
            let mut play = Play::engineering(EngineeringPosition::economy(3), None, seat);
            play.admit();
            app.insert_resource(Controller(play));
            app.update();
            let world = app.world_mut();
            let names: Vec<_> = world
                .query::<&Name>()
                .iter(world)
                .map(|n| n.as_str())
                .collect();
            assert!(names.contains(&"Printed mat label: HQ\n2"));
            assert!(names.contains(&"Printed mat label: R&D\n3"));
            assert!(names.contains(&"Printed mat label: Archives\n0 faceup\n0 facedown"));
            assert!(names.contains(&"Printed mat label: Stack\n1"));
            assert!(names.contains(&"Printed mat label: Heap\n0"));
            for entity in world.iter_entities().filter(|e| {
                e.get::<Name>()
                    .is_some_and(|n| n.as_str().starts_with("Printed mat label: "))
            }) {
                let pose = entity.get::<Transform>().unwrap();
                assert_eq!(
                    pose.rotation,
                    Quat::IDENTITY,
                    "all mat text faces the seated human"
                );
                assert!(pose.translation.y > -0.025 && pose.translation.y < 0.05);
                assert_eq!(entity.get::<Pickable>(), Some(&Pickable::IGNORE));
                assert!(entity.contains::<Mesh3d>());
            }
            assert!(
                !world
                    .query::<&Text>()
                    .iter(world)
                    .any(|text| matches!(text.0.as_str(), "Credits" | "Clicks"))
            );
        }
    }

    #[test]
    fn remote_growth_replaces_the_printed_atlas_and_rebuilds_remove_previous_rows() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        use bevy::camera::RenderTarget;
        let mut app = interface_app();
        let mut previous_image = None;
        let mut previous_entities = Vec::new();
        for remote_count in [1, 1, 6, 1] {
            let mut position = EngineeringPosition::economy(3);
            for remote in 1..=remote_count {
                let mut card = EngineeringCard::new(
                    CardKind::RegolithMiningLicense,
                    Zone::Root(Server::Remote(remote)),
                );
                card.installed_turn = Some(1);
                position.cards.push(card);
            }
            let mut play = Play::engineering(position, None, Player::Runner);
            play.admit();
            assert_eq!(play.admission(), &Admission::Ready);
            app.insert_resource(Controller(play));
            // A real UI redraw with unchanged zone count must reuse the image.
            let open = app.world().resource::<Interface>().history_open;
            app.world_mut().resource_mut::<Interface>().history_open = !open;
            app.update();
            let world = app.world_mut();
            for entity in previous_entities.drain(..) {
                assert!(
                    world.get_entity(entity).is_err(),
                    "old scene entity survived rebuild"
                );
            }
            let cameras: Vec<_> = world
                .query_filtered::<(Entity, &RenderTarget), (With<Camera2d>, With<BoardPiece>)>()
                .iter(world)
                .collect();
            assert_eq!(cameras.len(), 1, "one printed atlas camera");
            let RenderTarget::Image(target) = cameras[0].1 else {
                panic!("printed target")
            };
            let image = target.handle.clone();
            let rows = world
                .query_filtered::<Entity, With<turns::PrintedLabel>>()
                .iter(world)
                .count();
            assert_eq!(
                world
                    .resource::<Assets<Image>>()
                    .get(&image)
                    .unwrap()
                    .height(),
                rows as u32 * 224
            );
            if let Some((old_count, old_image)) = previous_image {
                assert_eq!(
                    image == old_image,
                    old_count == remote_count,
                    "reuse only matching row dimensions"
                );
            }
            previous_image = Some((remote_count, image));
            previous_entities = world
                .query_filtered::<Entity, With<BoardPiece>>()
                .iter(world)
                .collect();
            assert_eq!(
                world.query::<&UiTargetCamera>().iter(world).count(),
                1,
                "one text root after redraw"
            );
        }
    }

    #[test]
    fn status_hud_uses_original_tokens_with_accessible_counts() {
        let mut app = interface_app();
        let mut play = Play::engineering(
            crate::game::EngineeringPosition::economy(3),
            None,
            Player::Corp,
        );
        play.admit();
        app.insert_resource(Controller(play));
        app.update();
        for (meaning, key) in [
            ("Corp credits: 5", "tokens/credit.png"),
            ("Corp clicks: 3", "tokens/click-blue.png"),
            ("Runner credits: 5", "tokens/credit.png"),
            ("Runner clicks: 0", "tokens/click-red.png"),
        ] {
            let world = app.world_mut();
            let expected = world.resource::<Artwork>().image(key);
            let matches: Vec<_> = world
                .query::<(&ImageNode, &AccessibleLabel, &Node)>()
                .iter(world)
                .filter(|(_, label, _)| label.0 == meaning)
                .collect();
            assert_eq!(matches.len(), 1, "{meaning}");
            assert_eq!(matches[0].0.image, expected);
            assert_eq!(matches[0].2.width, px(24.));
            assert_eq!(matches[0].2.height, px(24.));
        }
    }

    #[test]
    fn ordinary_table_uses_one_token_with_exact_counts_including_zero_and_updates() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};

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
                    .query::<(&LabelAnchor, &Text)>()
                    .iter(world)
                    .filter(|(label, _)| {
                        counter_anchor(label).is_some_and(|(at, _)| at == tokens[0])
                    })
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
    fn remote_root_bodies_do_not_intersect_each_other_or_adjacent_servers() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        for seat in [Player::Runner, Player::Corp] {
            for depth in [0, 4, 5] {
                let mut position = EngineeringPosition::economy(3);
                for (kind, server) in [
                    (CardKind::RegolithMiningLicense, Server::Remote(1)),
                    (CardKind::ManegarmSkunkworks, Server::Remote(1)),
                    (CardKind::NicoCampaign, Server::Remote(2)),
                ] {
                    let mut card = EngineeringCard::new(kind, Zone::Root(server));
                    card.rezzed = true;
                    card.installed_turn = Some(1);
                    position.cards.push(card);
                }
                for _ in 0..depth {
                    let mut card = EngineeringCard::new(
                        CardKind::Palisade,
                        Zone::Protecting(Server::Remote(1)),
                    );
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
                let cards: Vec<_> = world
                    .iter_entities()
                    .filter(|entity| {
                        entity.get::<Name>().is_some_and(|name| {
                            matches!(
                                name.as_str(),
                                "Regolith Mining License"
                                    | "Manegarm Skunkworks"
                                    | "Nico Campaign"
                                    | "Palisade"
                            )
                        })
                    })
                    .map(|entity| {
                        let mesh = meshes.get(&entity.get::<Mesh3d>().unwrap().0).unwrap();
                        let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                        else {
                            panic!("mesh positions")
                        };
                        let transform = entity.get::<Transform>().unwrap();
                        let bounds = vertices
                            .iter()
                            .map(|p| transform.transform_point(Vec3::from_array(*p)))
                            .fold(
                                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                                |(min, max), p| (min.min(p), max.max(p)),
                            );
                        (entity.get::<Name>().unwrap().as_str(), bounds)
                    })
                    .collect();
                assert_eq!(
                    cards.len(),
                    3 + depth,
                    "one physical body per root or ICE card"
                );
                for (i, (a, (amin, amax))) in cards.iter().enumerate() {
                    for (b, (bmin, bmax)) in &cards[i + 1..] {
                        let gap = (amin.x - bmax.x)
                            .max(bmin.x - amax.x)
                            .max(amin.z - bmax.z)
                            .max(bmin.z - amax.z);
                        assert!(
                            gap >= 0.2,
                            "{seat:?}: {a} and {b} need physical clearance, found {gap}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn corp_mat_contains_full_ice_columns_from_either_seat() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        for seat in [Player::Runner, Player::Corp] {
            let mut initial_mat = None;
            for depth in [0, 1, 2, 4, 5, 6] {
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
                let mat_depth = max.z - min.z;
                let initial = *initial_mat.get_or_insert(mat_depth);
                if depth <= 4 {
                    assert_eq!(
                        mat_depth, initial,
                        "four ICE must fit the initially reserved mat"
                    );
                } else {
                    assert!(mat_depth > initial, "a fifth ICE expands the mat");
                }
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
        whitespace_native_choice(true);
        whitespace_native_choice(false);
    }

    #[test]
    fn native_tread_lightly_corp_can_install_for_free_then_pay_six_to_rez() {
        use crate::game::{EngineeringPosition, Server};
        let mut play = Play::engineering(
            EngineeringPosition::run_income(18, true),
            None,
            Player::Corp,
        );
        play.admit();
        for _ in 0..20 {
            let view = [Player::Corp, Player::Runner]
                .into_iter()
                .find_map(|p| play.observe(p).filter(|v| v.decision.is_some()))
                .unwrap();
            if view.phase.starts_with("Brân ·") {
                break;
            }
            let d = view.decision.unwrap();
            let action = if view.phase == "Action phase" {
                Action::PlayCard(view.hand[0].instance.unwrap())
            } else if d.choices.contains(&Action::RunServer(Server::Hq)) {
                Action::RunServer(Server::Hq)
            } else {
                Action::PassPriority
            };
            play.choose_for(d.actor, d.revision, action).unwrap();
        }
        assert!(play.observation().unwrap().phase.starts_with("Brân ·"));
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let view = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        let palisade = view.hand[0].instance.unwrap();
        let bran = view
            .servers
            .iter()
            .flat_map(|s| &s.ice)
            .find(|c| c.printing == Some("30039"))
            .unwrap()
            .instance
            .unwrap();
        activate(&mut app, Action::InstallInward(palisade));
        activate(&mut app, Action::TrashInstalled(bran));
        activate(&mut app, Action::FinishInstall);
        for _ in 0..16 {
            let view = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if let Some(d) = view.decision {
                if d.choices.contains(&Action::Rez(palisade)) {
                    assert_eq!(view.corp_credits, 6);
                    let world = app.world_mut();
                    assert!(
                        world
                            .query::<&Text>()
                            .iter(world)
                            .any(|t| t.0 == "Rez Palisade · 6 credits")
                    );
                    assert!(
                        world
                            .query::<&Text>()
                            .iter(world)
                            .any(|t| t.0 == "Tread Lightly: ICE rez costs +3 during this run.")
                    );
                    activate(&mut app, Action::Rez(palisade));
                    assert_eq!(
                        app.world()
                            .resource::<Controller>()
                            .0
                            .observation()
                            .unwrap()
                            .corp_credits,
                        0
                    );
                    return;
                }
                activate(&mut app, Action::PassPriority);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("native Corp did not reach paid rez");
    }

    #[test]
    fn native_run_income_buttons_show_hosted_counts_and_resume_collection() {
        use crate::game::{EngineeringPosition, Server};
        let directory = tempfile::tempdir().unwrap();
        let options = SaveOptions {
            path: directory.path().join("income.json"),
            recover_from: None,
        };
        let mut play = Play::engineering(
            EngineeringPosition::run_income(18, false),
            Some(options.clone()),
            Player::Runner,
        );
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let mut ran = false;
        for _ in 0..40 {
            let view = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if view.phase == "Action phase" {
                let world = app.world_mut();
                assert!(
                    world
                        .query::<&Name>()
                        .iter(world)
                        .any(|n| n.as_str() == "Runner Red Team credits")
                );
                assert!(
                    world
                        .query::<&Name>()
                        .iter(world)
                        .any(|n| n.as_str() == "Runner Pennyshaver credits")
                );
                if ran {
                    assert!(
                        world
                            .query::<&Text>()
                            .iter(world)
                            .any(|t| t.0 == "Central runs this turn: R&D")
                    );
                    assert!(
                        world
                            .query::<&Text>()
                            .iter(world)
                            .any(|t| t.0 == "Pennyshaver · 1 click: place 1, then take 2 credits")
                    );
                    let penny = view
                        .rig
                        .iter()
                        .find(|c| c.printing == Some("30014"))
                        .unwrap()
                        .instance
                        .unwrap();
                    let old = app.world_mut().remove_resource::<Controller>().unwrap();
                    drop(old);
                    let mut reopened = Play::load_turns(options.clone(), Player::Runner);
                    reopened.admit();
                    assert_eq!(reopened.observation(), Some(view));
                    app.insert_resource(Controller(reopened));
                    app.update();
                    activate(&mut app, Action::TakePennyshaver(penny));
                    let after = app
                        .world()
                        .resource::<Controller>()
                        .0
                        .observation()
                        .unwrap();
                    assert_eq!(after.runner_credits, 13);
                    assert_eq!(
                        after
                            .rig
                            .iter()
                            .find(|c| c.printing == Some("30018"))
                            .unwrap()
                            .hosted_credits,
                        9
                    );
                    assert_eq!(
                        after
                            .rig
                            .iter()
                            .find(|c| c.printing == Some("30014"))
                            .unwrap()
                            .hosted_credits,
                        0
                    );
                    return;
                }
                let red = view
                    .rig
                    .iter()
                    .find(|c| c.printing == Some("30018"))
                    .unwrap()
                    .instance
                    .unwrap();
                assert_eq!(
                    turns::action_card(Action::RunRedTeam {
                        card: red,
                        server: Server::Rd
                    }),
                    Some(red)
                );
                activate(
                    &mut app,
                    Action::RunRedTeam {
                        card: red,
                        server: Server::Rd,
                    },
                );
                ran = true;
            } else if let Some(d) = view.decision {
                let action = [
                    Action::PassPriority,
                    Action::Continue,
                    Action::AccessRd,
                    Action::FinishAccess,
                ]
                .into_iter()
                .find(|a| d.choices.contains(a))
                .unwrap();
                activate(&mut app, action);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("native income route did not finish");
    }

    #[test]
    fn native_mayfly_buttons_preserve_payment_warning_resume_and_run_end_trash() {
        native_mayfly_copy(false);
        native_mayfly_copy(true);
    }

    fn native_mayfly_copy(two_copies: bool) {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        let directory = tempfile::tempdir().unwrap();
        let options = SaveOptions {
            path: directory.path().join("mayfly.json"),
            recover_from: None,
        };
        let mut position = EngineeringPosition::economy(17);
        position.active = Player::Runner;
        position.runner_credits = 8;
        position.corp_credits = 0;
        position.cards = vec![EngineeringCard::new(CardKind::Overclock, Zone::Grip)];
        for (kind, zone) in [
            (CardKind::Carmen, Zone::Rig),
            (CardKind::Mayfly, Zone::Rig),
            (CardKind::Unity, Zone::Rig),
            (CardKind::Tithe, Zone::Protecting(Server::Hq)),
        ] {
            position.cards.push(EngineeringCard {
                kind,
                zone,
                rezzed: kind == CardKind::Tithe,
                advancement: 0,
                installed_turn: Some(1),
            });
        }
        if two_copies {
            position
                .cards
                .retain(|c| !matches!(c.kind, CardKind::Carmen | CardKind::Unity));
            let second = position
                .cards
                .iter()
                .find(|c| c.kind == CardKind::Mayfly)
                .unwrap()
                .clone();
            position.cards.push(second);
        }
        let mut play = Play::engineering(position, Some(options.clone()), Player::Runner);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        let mut broke = false;
        let mut restored = false;
        for _ in 0..50 {
            let v = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if v.run_result.is_some() {
                assert!(broke && restored);
                assert_eq!(v.memory_used, 2);
                assert!(v.heap.iter().any(|c| c.printing == Some("30032")));
                return;
            }
            if v.run
                .as_ref()
                .is_some_and(|r| !r.trash_at_run_end.is_empty())
                && !restored
            {
                let world = app.world_mut();
                assert!(world.query::<&Text>().iter(world).any(|t| t.0
                    == if two_copies {
                        "Mayfly at rig 1 will be trashed when this run ends."
                    } else {
                        "Mayfly: 1 will be trashed when this run ends."
                    }));
                let old = app.world_mut().remove_resource::<Controller>().unwrap();
                drop(old);
                let mut reopened = Play::load_turns(options.clone(), Player::Runner);
                reopened.admit();
                assert_eq!(reopened.observation(), Some(v.clone()));
                app.insert_resource(Controller(reopened));
                app.update();
                restored = true;
            }
            if let Some(d) = &v.decision {
                let action = if v.phase == "Action phase" {
                    Action::PlayCard(v.hand[0].instance.unwrap())
                } else if d.choices.contains(&Action::RunServer(Server::Hq)) {
                    Action::RunServer(Server::Hq)
                } else if d.choices.contains(&Action::PayRunCredits(1)) {
                    if two_copies {
                        let world = app.world_mut();
                        assert!(
                            world
                                .query::<&Text>()
                                .iter(world)
                                .any(|t| t.0 == "Pay for Mayfly · rig 1: break subroutine 1")
                        );
                    }
                    Action::PayRunCredits(1)
                } else if !broke && v.run.as_ref().is_some_and(|r| r.encounter_paid) {
                    let mayfly = v
                        .rig
                        .iter()
                        .find(|c| c.printing == Some("30032"))
                        .unwrap()
                        .instance
                        .unwrap();
                    let world = app.world_mut();
                    if !two_copies {
                        assert!(world.query::<&Text>().iter(world).any(|t| t.0 == "Carmen · +3 strength · 2 credits · this encounter"));
                    }
                    assert!(world.query::<&Text>().iter(world).any(|t| t.0
                        == if two_copies {
                            "Mayfly · rig 1 · break sub 1 · 1 credit · trash at run end"
                        } else {
                            "Mayfly · break sub 1 · 1 credit · trash at run end"
                        }));
                    broke = true;
                    Action::Break {
                        breaker: mayfly,
                        subroutines: 1,
                    }
                } else if d.choices.contains(&Action::JackOut) {
                    Action::JackOut
                } else {
                    Action::PassPriority
                };
                activate(&mut app, action);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("native Mayfly route did not finish");
    }

    fn whitespace_native_choice(break_second: bool) {
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
            if break_second {
                Action::Break {
                    breaker: unity,
                    subroutines: 2,
                }
            } else {
                Action::Boost(unity)
            },
        );
        {
            let world = app.world_mut();
            let purpose = if break_second {
                "Pay for Unity: break subroutine 2"
            } else {
                "Pay for Unity: boost strength"
            };
            assert!(
                world.query::<&Text>().iter(world).any(|t| t.0 == purpose),
                "payment must identify its ability"
            );
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
        if !break_second {
            assert_eq!(view.run_result, Some(crate::game::RunResult::EndedByIce));
            assert!(view.run.is_none());
            assert!(view.heap.iter().any(|c| c.printing == Some("30029")));
            let mut resumed = Play::load_turns(options, Player::Runner);
            resumed.admit();
            assert_eq!(resumed.observation(), Some(view));
            return;
        }
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
                .query::<(&LabelAnchor, &Text)>()
                .iter(world)
                .any(|(l, t)| counter_anchor(l).is_some_and(|(at, _)| at == token) && t.0 == "4")
        );
        let mut resumed = Play::load_turns(options, Player::Runner);
        resumed.admit();
        assert_eq!(resumed.observation(), Some(view));
        for _ in 0..20 {
            let view = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if view.run.is_none()
                && view.run_result == Some(crate::game::RunResult::SuccessfulAccess)
            {
                assert_eq!(view.runner_credits, 4);
                return;
            }
            if let Some(decision) = view.decision {
                activate(
                    &mut app,
                    if decision.choices.contains(&Action::Continue) {
                        Action::Continue
                    } else {
                        Action::PassPriority
                    },
                );
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("native controls did not finish the empty HQ breach");
    }

    #[test]
    fn native_manegarm_access_buttons_preserve_cost_purpose_and_mandatory_steal() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        let mut p = EngineeringPosition::economy(42);
        p.active = Player::Runner;
        p.cards = vec![
            EngineeringCard::new(CardKind::Overclock, Zone::Grip),
            EngineeringCard::new(CardKind::OffworldOffice, Zone::Hq),
            EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
        ];
        for (kind, zone, rezzed) in [
            (CardKind::ManegarmSkunkworks, Zone::Root(Server::Hq), true),
            (CardKind::DocklandsPass, Zone::Rig, false),
        ] {
            p.cards.push(EngineeringCard {
                kind,
                zone,
                rezzed,
                installed_turn: Some(1),
                advancement: 0,
            });
        }
        let mut play = Play::engineering(p, None, Player::Runner);
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        for _ in 0..20 {
            let v = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if v.phase.starts_with("Manegarm") {
                break;
            }
            if let Some(d) = v.decision {
                let action = if v.phase == "Action phase" {
                    Action::PlayCard(v.hand[0].instance.unwrap())
                } else if d.choices.contains(&Action::RunServer(Server::Hq)) {
                    Action::RunServer(Server::Hq)
                } else if d.choices.contains(&Action::Continue) {
                    Action::Continue
                } else {
                    Action::PassPriority
                };
                activate(&mut app, action);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        assert!(
            app.world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap()
                .phase
                .starts_with("Manegarm")
        );
        activate(&mut app, Action::SpendManegarmClicks);
        activate(&mut app, Action::AccessRoot(0));
        activate(&mut app, Action::TrashAccessed);
        let world = app.world_mut();
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|t| t.0 == "Pay for trash Manegarm Skunkworks")
        );
        activate(&mut app, Action::PayRunCredits(3));
        for _ in 0..2 {
            activate(&mut app, Action::AccessHq);
            let v = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            let action = if v.accessed.unwrap().printing == Some("30067") {
                assert_eq!(v.decision.unwrap().choices, vec![Action::StealAgenda]);
                Action::StealAgenda
            } else {
                Action::FinishAccess
            };
            activate(&mut app, action);
        }
        let v = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        assert!(v.run.is_none());
        assert_eq!(v.agenda_points, [0, 2]);
        assert_eq!(v.clicks[1], 1);
        assert_eq!(v.runner_credits, 4);
    }

    #[test]
    fn native_jailbreak_access_displays_exact_steal_and_checkpoint_victory() {
        use crate::game::{EngineeringPosition, Server};
        let mut play = Play::engineering(
            EngineeringPosition::central_access(42, true),
            None,
            Player::Runner,
        );
        play.admit();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        for _ in 0..30 {
            let v = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if v.match_winner.is_some() {
                assert_eq!(v.match_winner, Some(Player::Runner));
                assert_eq!(v.agenda_points, [0, 6]);
                let world = app.world_mut();
                assert!(
                    !world
                        .query::<&Text>()
                        .iter(world)
                        .any(|t| t.0 == "Accessed card")
                );
                let agendas = world
                    .query::<&Name>()
                    .iter(world)
                    .filter(|name| name.as_str() == "Send a Message")
                    .count();
                assert_eq!(
                    agendas, 2,
                    "only the two scored agendas remain, without a stale preview"
                );
                return;
            }
            if let Some(d) = v.decision {
                let action = if v.phase == "Action phase" {
                    Action::PlayCard(
                        v.hand
                            .iter()
                            .find(|c| c.printing == Some("30028"))
                            .unwrap()
                            .instance
                            .unwrap(),
                    )
                } else if d.choices.contains(&Action::RunServer(Server::Rd)) {
                    Action::RunServer(Server::Rd)
                } else if d.choices.contains(&Action::StealAgenda) {
                    let world = app.world_mut();
                    assert!(
                        world
                            .query::<&Text>()
                            .iter(world)
                            .any(|t| t.0 == "Steal Send a Message · 3 agenda points")
                    );
                    Action::StealAgenda
                } else if d.choices.contains(&Action::AccessRd) {
                    Action::AccessRd
                } else if d.choices.contains(&Action::Continue) {
                    Action::Continue
                } else {
                    Action::PassPriority
                };
                activate(&mut app, action);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("native central route did not reach the winning checkpoint");
    }

    #[test]
    fn native_damage_buttons_preserve_karuna_choice_trap_payment_and_flatline() {
        use crate::game::{EngineeringPosition, Server, baseline_response};
        for (karuna, lethal_encounter) in [(true, false), (false, false), (true, true)] {
            let mut position = EngineeringPosition::damage_run(42, karuna);
            if lethal_encounter {
                for card in position
                    .cards
                    .iter_mut()
                    .filter(|c| c.zone == crate::game::Zone::Grip)
                    .take(2)
                {
                    card.zone = crate::game::Zone::Stack;
                }
            }
            let mut play = Play::engineering(position, None, Player::Runner);
            play.admit();
            let mut app = interface_app();
            app.insert_resource(Controller(play));
            app.update();
            let mut saw_choice = false;
            for _ in 0..80 {
                let v = app
                    .world()
                    .resource::<Controller>()
                    .0
                    .observation()
                    .unwrap();
                if v.match_winner.is_some() || v.run_result.is_some() {
                    assert!(saw_choice);
                    assert_eq!(v.match_winner, karuna.then_some(Player::Corp));
                    assert_eq!(
                        v.heap.len(),
                        if lethal_encounter {
                            2
                        } else if karuna {
                            4
                        } else {
                            5
                        }
                    );
                    if karuna {
                        let world = app.world_mut();
                        assert!(
                            !world
                                .query::<&Text>()
                                .iter(world)
                                .any(|t| t.0.contains("pass to advance"))
                        );
                        assert!(
                            !world
                                .query::<&ButtonCommand>()
                                .iter(world)
                                .any(|b| matches!(b, ButtonCommand::Choose(..)))
                        );
                    }
                    break;
                }
                if let Some(d) = &v.decision {
                    let action = if v.phase == "Action phase" && !karuna {
                        Action::PlayCard(
                            v.hand
                                .iter()
                                .find(|c| c.printing == Some("30029"))
                                .unwrap()
                                .instance
                                .unwrap(),
                        )
                    } else if d.choices.contains(&Action::RunServer(Server::Remote(1))) {
                        Action::RunServer(Server::Remote(1))
                    } else if v.phase.starts_with("Karunā") {
                        saw_choice = true;
                        let world = app.world_mut();
                        assert!(world.query::<&Text>().iter(world).any(|t| t.0 == "Continue · resolve remaining unbroken subroutines"));
                        assert!(
                            !world
                                .query::<&Text>()
                                .iter(world)
                                .any(|t| t.0.contains("No encounter"))
                        );
                        assert_eq!(d.choices, vec![Action::Continue, Action::JackOut]);
                        Action::Continue
                    } else if d.choices.contains(&Action::TrashAccessed) {
                        saw_choice = true;
                        let world = app.world_mut();
                        assert!(
                            world
                                .query::<&Text>()
                                .iter(world)
                                .any(|t| t.0 == "Trash accessed card · pay 2 credits")
                        );
                        Action::TrashAccessed
                    } else {
                        baseline_response(&v).unwrap()
                    };
                    activate(&mut app, action);
                } else {
                    app.world_mut()
                        .resource_mut::<Controller>()
                        .0
                        .respond()
                        .unwrap();
                    app.update();
                }
            }
            let v = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            assert!(v.match_winner.is_some() || v.run_result.is_some());
        }
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

    /// Read the real laid-out glyph rectangles through each rendered plane's
    /// UV mapping. Transparent row padding is deliberately not treated as ink.
    fn printed_glyph_corners(world: &World) -> Vec<(Entity, String, Vec<Vec3>)> {
        use bevy::{camera::RenderTarget, mesh::VertexAttributeValues};
        let meshes = world.resource::<Assets<Mesh>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        let images = world.resource::<Assets<Image>>();
        world.iter_entities().filter(|entity| entity.contains::<turns::PrintedLabel>()).map(|plane| {
            let text = plane.get::<Name>().unwrap().as_str().strip_prefix("Printed mat label: ").unwrap();
            let material = materials.get(&plane.get::<MeshMaterial3d<StandardMaterial>>().unwrap().0).unwrap();
            assert!(material.unlit);
            assert_eq!(material.alpha_mode, AlphaMode::Blend);
            let image = material.base_color_texture.as_ref().unwrap();
            let size = images.get(image).unwrap().size().as_vec2();
            let root = world.iter_entities().find(|entity| entity.get::<UiTargetCamera>().is_some_and(|target| {
                matches!(world.get::<RenderTarget>(target.0), Some(RenderTarget::Image(target)) if target.handle == *image)
            })).unwrap();
            let text_entity = root.get::<Children>().unwrap().iter().flat_map(|row| world.get::<Children>(row).unwrap().iter()).find(|entity| world.get::<Text>(*entity).is_some_and(|value| value.0 == text)).unwrap();
            let layout = world.get::<bevy::text::TextLayoutInfo>(text_entity).unwrap();
            assert!(!layout.glyphs.is_empty(), "{text} needs real glyphs");
            let node = world.get::<ComputedNode>(text_entity).unwrap();
            let ui = world.get::<UiGlobalTransform>(text_entity).unwrap();
            let mesh = meshes.get(&plane.get::<Mesh3d>().unwrap().0).unwrap();
            let Some(VertexAttributeValues::Float32x3(vertices)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!("plane positions") };
            let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else { panic!("plane UVs") };
            let origin = Vec3::from_array(vertices[0]);
            let origin_uv = Vec2::from_array(uvs[0]);
            let x = uvs.iter().position(|uv| uv[0] != origin_uv.x && uv[1] == origin_uv.y).unwrap();
            let z = uvs.iter().position(|uv| uv[1] != origin_uv.y && uv[0] == origin_uv.x).unwrap();
            let dx = (Vec3::from_array(vertices[x]) - origin) / (uvs[x][0] - origin_uv.x);
            let dz = (Vec3::from_array(vertices[z]) - origin) / (uvs[z][1] - origin_uv.y);
            let pose = plane.get::<Transform>().unwrap();
            let corners = layout.glyphs.iter().flat_map(|glyph| {
                let center = node.content_box().min + glyph.position;
                let half = glyph.atlas_info.rect.size() / 2.;
                [Vec2::new(-1., -1.), Vec2::new(1., -1.), Vec2::new(1., 1.), Vec2::new(-1., 1.)].map(|direction| {
                    let uv = ui.transform_point2(center + half * direction) / size;
                    pose.transform_point(origin + dx * (uv.x - origin_uv.x) + dz * (uv.y - origin_uv.y))
                })
            }).collect();
            (plane.id(), text.to_owned(), corners)
        }).collect()
    }

    #[test]
    fn fitted_table_labels_have_real_text_clearance_in_both_seats() {
        use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone};
        use bevy::{
            camera::{ComputedCameraValues, RenderTargetInfo},
            input::mouse::AccumulatedMouseScroll,
        };
        let mut defects = Vec::new();
        for seat in [Player::Corp, Player::Runner] {
            for (hq_size, grip_size) in [(2, 4), (6, 6), (7, 7), (20, 4), (2, 20)] {
                for upgrade_server in [
                    None,
                    Some(Server::Remote(1)),
                    Some(Server::Hq),
                    Some(Server::Rd),
                    Some(Server::Archives),
                ] {
                    for size in [UVec2::new(1600, 1000), UVec2::new(1280, 800)] {
                        let scene_height = size.y as f32 - turns::dock_height(size.y as f32);
                        let mut app = interface_app_with_text(true);
                        // The bundled, monospaced font gives portable real glyph metrics;
                        // the actual admitted DejaVu font is additionally checked natively.
                        let mut position = EngineeringPosition::economy(26);
                        position.active = seat;
                        position
                            .cards
                            .retain(|card| !matches!(card.zone, Zone::Hq | Zone::Grip));
                        position.cards.extend(
                            (0..hq_size)
                                .map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Hq)),
                        );
                        position.cards.extend(
                            (0..grip_size)
                                .map(|_| EngineeringCard::new(CardKind::SureGamble, Zone::Grip)),
                        );
                        // A populated pile exercises the full two-digit caption
                        // beside the central root, not just the empty-pile label.
                        position
                            .cards
                            .extend((0..10).map(|_| {
                                EngineeringCard::new(CardKind::HedgeFund, Zone::Archives)
                            }));
                        position.cards.extend(
                            (0..7).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Rd)),
                        );
                        for (kind, zone) in [
                            (
                                CardKind::RegolithMiningLicense,
                                Zone::Root(Server::Remote(1)),
                            ),
                            (CardKind::NicoCampaign, Zone::Root(Server::Remote(2))),
                            (CardKind::Palisade, Zone::Protecting(Server::Hq)),
                            (CardKind::Palisade, Zone::Protecting(Server::Rd)),
                            (CardKind::Palisade, Zone::Protecting(Server::Archives)),
                        ]
                        .into_iter()
                        .chain(
                            upgrade_server
                                .map(|server| (CardKind::ManegarmSkunkworks, Zone::Root(server))),
                        )
                        .chain(
                            (0..5)
                                .map(|_| (CardKind::Palisade, Zone::Protecting(Server::Remote(1)))),
                        ) {
                            let mut card = EngineeringCard::new(kind, zone);
                            card.installed_turn = Some(1);
                            position.cards.push(card);
                        }
                        let mut play = Play::engineering(position, None, seat);
                        play.admit();
                        assert_eq!(
                            play.admission(),
                            &Admission::Ready,
                            "hand sizes {hq_size}/{grip_size}"
                        );
                        app.insert_resource(Controller(play));
                        app.insert_resource(unfolded_tabletop::camera::Orbit::new((
                            Vec3::ZERO,
                            27.,
                        )));
                        app.init_resource::<AccumulatedMouseScroll>();
                        app.world_mut().spawn((
                            Window {
                                focused: true,
                                resolution: (size.x, size.y).into(),
                                ..default()
                            },
                            PrimaryWindow,
                        ));
                        let camera = app
                            .world_mut()
                            .spawn((
                                Camera3d::default(),
                                unfolded_tabletop::camera::TableCamera,
                                Transform::default(),
                            ))
                            .id();
                        app.world_mut().spawn((Camera2d, IsDefaultUiCamera));
                        turns::table_camera(&mut app);
                        app.add_systems(
                            Update,
                            (
                                camera_viewport,
                                turns::camera_facts_and_framing,
                                move |mut cameras: Query<(
                                    &mut Camera,
                                    &mut Projection,
                                    Has<Camera3d>,
                                    Option<&bevy::camera::RenderTarget>,
                                )>,
                                      images: Res<Assets<Image>>| {
                                    for (mut camera, mut projection, three_d, target) in
                                        &mut cameras
                                    {
                                        let target_size = match target {
                                            Some(bevy::camera::RenderTarget::Image(target)) => {
                                                images.get(&target.handle).unwrap().size()
                                            }
                                            _ => size,
                                        };
                                        projection.update(
                                            target_size.x as f32,
                                            if three_d {
                                                scene_height
                                            } else {
                                                target_size.y as f32
                                            },
                                        );
                                        camera.computed = ComputedCameraValues {
                                            clip_from_view: projection.get_clip_from_view(),
                                            target_info: Some(RenderTargetInfo {
                                                physical_size: target_size,
                                                scale_factor: 1.,
                                            }),
                                            ..default()
                                        };
                                    }
                                },
                            )
                                .chain()
                                .after(draw_interface),
                        );
                        for _ in 0..6 {
                            app.update();
                        }
                        for home in [false, true] {
                            if home {
                                app.world_mut()
                                    .resource_mut::<unfolded_tabletop::camera::Orbit>()
                                    .focus((Vec3::splat(90.), 10.));
                                app.world_mut()
                                    .resource_mut::<ButtonInput<KeyCode>>()
                                    .press(KeyCode::Home);
                                for _ in 0..3 {
                                    app.world_mut().run_schedule(Update);
                                    app.world_mut().run_schedule(PostUpdate);
                                }
                            }
                            let world = app.world_mut();
                            let camera_at = world.get::<GlobalTransform>(camera).unwrap();
                            let view_camera = world.get::<Camera>(camera).unwrap();
                            for entity in world.iter_entities().filter(|entity| {
                                entity.contains::<ImageNode>()
                                    && entity.contains::<AccessibleLabel>()
                            }) {
                                let parent = entity.get::<ChildOf>().unwrap().parent();
                                let node = world.get::<ComputedNode>(parent).unwrap();
                                let at =
                                    world.get::<UiGlobalTransform>(parent).unwrap().translation;
                                assert!(
                                    (at - node.size() / 2.).cmpge(Vec2::ZERO).all()
                                        && (at + node.size() / 2.).cmple(size.as_vec2()).all(),
                                    "HUD side group must fit at {size:?}"
                                );
                            }
                            let labels: Vec<_> = printed_glyph_corners(world).into_iter()
                        .map(|(entity, text, corners)| {
                            let mut bounds = Rect {
                                min: Vec2::splat(f32::INFINITY),
                                max: Vec2::splat(f32::NEG_INFINITY),
                            };
                            for corner in corners {
                                let point = view_camera.world_to_viewport(camera_at, corner).unwrap();
                                bounds.min = bounds.min.min(point);
                                bounds.max = bounds.max.max(point);
                            }
                            assert!(
                                bounds.min.cmpge(Vec2::ZERO).all()
                                    && bounds
                                        .max
                                        .cmple(Vec2::new(size.x as f32, scene_height))
                                        .all(),
                                "{seat:?}, {size:?}: {} must fit the table viewport: {bounds:?}",
                                text
                            );
                            (entity, text, bounds)
                        })
                        .collect();
                            assert!(labels.len() >= 12);
                            for (index, (_, a, bounds_a)) in labels.iter().enumerate() {
                                for (_, b, bounds_b) in &labels[index + 1..] {
                                    if !bounds_a.intersect(*bounds_b).is_empty() {
                                        defects.push(format!("{seat:?}, {size:?}, Home={home}: {a:?} {bounds_a:?} overlaps {b:?} {bounds_b:?}"));
                                    }
                                }
                            }
                            let camera_at = world.get::<GlobalTransform>(camera).unwrap();
                            let camera = world.get::<Camera>(camera).unwrap();
                            let meshes = world.resource::<Assets<Mesh>>();
                            for entity in world.iter_entities().filter(|entity| {
                                entity.contains::<BoardPiece>()
                                    && !entity.contains::<turns::PrintedLabel>()
                                    && entity.get::<Name>().is_some_and(|name| {
                                        !name.as_str().ends_with("playmat")
                                            && !name.as_str().ends_with("pool")
                                    })
                            }) {
                                let Some(mesh) =
                                    entity.get::<Mesh3d>().and_then(|mesh| meshes.get(&mesh.0))
                                else {
                                    continue;
                                };
                                let Some(bevy::mesh::VertexAttributeValues::Float32x3(vertices)) =
                                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                                else {
                                    continue;
                                };
                                let transform = entity.get::<Transform>().unwrap();
                                let mut bounds = Rect {
                                    min: Vec2::splat(f32::INFINITY),
                                    max: Vec2::splat(f32::NEG_INFINITY),
                                };
                                for vertex in vertices {
                                    let point = camera
                                        .world_to_viewport(
                                            camera_at,
                                            transform.transform_point(Vec3::from_array(*vertex)),
                                        )
                                        .unwrap();
                                    bounds.min = bounds.min.min(point);
                                    bounds.max = bounds.max.max(point);
                                }
                                for (_, text, label) in &labels {
                                    if !label.intersect(bounds).is_empty() {
                                        defects.push(format!("{seat:?}, {size:?}, upgrade in {upgrade_server:?}, Home={home}: {text:?} {label:?} covers {} {bounds:?}", entity.get::<Name>().unwrap()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(defects.is_empty(), "{}", defects.join("\n"));
    }

    #[test]
    fn initial_and_home_camera_frame_the_rendered_mats_in_both_seats() {
        use bevy::input::mouse::AccumulatedMouseScroll;
        use unfolded_tabletop::camera::Orbit;
        for seat in [Player::Corp, Player::Runner] {
            let mut app = interface_app();
            let mut play = Play::introductory_seeded(42, None, seat);
            play.admit();
            app.insert_resource(Controller(play));
            app.insert_resource(Orbit::new((Vec3::ZERO, 27.)));
            app.init_resource::<ButtonInput<KeyCode>>();
            app.init_resource::<ButtonInput<MouseButton>>();
            app.init_resource::<AccumulatedMouseScroll>();
            app.world_mut().spawn((
                Window {
                    focused: true,
                    resolution: (1600, 1000).into(),
                    ..default()
                },
                PrimaryWindow,
            ));
            let camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    unfolded_tabletop::camera::TableCamera,
                    Transform::default(),
                ))
                .id();
            turns::table_camera(&mut app);
            app.add_systems(
                Update,
                turns::camera_facts_and_framing.after(draw_interface),
            );
            app.update();
            for home in [false, true] {
                if home {
                    app.world_mut()
                        .resource_mut::<Orbit>()
                        .focus((Vec3::new(90., 0., 90.), 10.));
                    app.world_mut()
                        .resource_mut::<ButtonInput<KeyCode>>()
                        .press(KeyCode::Home);
                    app.update();
                }
                let world = app.world();
                let camera = world.get::<Transform>(camera).unwrap();
                let projection =
                    bevy::camera::CameraProjection::get_clip_from_view(&PerspectiveProjection {
                        aspect_ratio: 1600. / 760.,
                        ..default()
                    });
                let clip_from_world = projection * camera.to_matrix().inverse();
                let meshes = world.resource::<Assets<Mesh>>();
                for entity in world.iter_entities().filter(|entity| {
                    entity
                        .get::<Name>()
                        .is_some_and(|name| name.as_str().ends_with("playmat"))
                }) {
                    let mesh = meshes.get(&entity.get::<Mesh3d>().unwrap().0).unwrap();
                    let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
                    else {
                        panic!("mesh positions")
                    };
                    for vertex in vertices {
                        let point = entity
                            .get::<Transform>()
                            .unwrap()
                            .transform_point(Vec3::from_array(*vertex));
                        let projected = clip_from_world.project_point3(point);
                        assert!(
                            projected.x.abs() < 0.98 && projected.y.abs() < 0.98,
                            "{seat:?}, Home={home}: rendered mat corner {point:?} is clipped at {projected:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn native_camera_captures_modals_and_dock_wheel_but_navigates_the_table() {
        use crate::game::EngineeringPosition;
        use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
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
        app.world_mut().spawn((
            Camera3d::default(),
            unfolded_tabletop::camera::TableCamera,
            Transform::default(),
        ));
        turns::table_camera(&mut app);
        app.add_systems(
            Update,
            turns::camera_facts_and_framing.after(draw_interface),
        );
        app.update();
        let printed = |world: &mut World| {
            let mut labels: Vec<_> = world
                .query_filtered::<(&Name, &Transform), With<turns::PrintedLabel>>()
                .iter(world)
                .map(|(name, at)| (name.as_str().to_owned(), *at))
                .collect();
            labels.sort_by(|a, b| a.0.cmp(&b.0));
            labels
        };
        let before_labels = printed(app.world_mut());
        assert!(!before_labels.is_empty());
        let initial = app.world().resource::<Orbit>().pose();
        app.update();
        assert_eq!(app.world().resource::<Orbit>().pose(), initial);
        app.world_mut()
            .get_mut::<Window>(window_id)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(300., 300.)));
        app.world_mut().resource_mut::<Interface>().history_open = true;
        app.update();
        assert_eq!(app.world().resource::<Orbit>().pose(), initial);
        app.world_mut().resource_mut::<Interface>().history_open = false;
        app.update();
        assert!(app.world().resource::<Orbit>().pose().distance < initial.distance);
        assert_eq!(
            printed(app.world_mut()),
            before_labels,
            "camera movement and modal redraws never billboard or relocate mat ink"
        );
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

    fn bran_install(app: &mut App) {
        let action = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap()
            .decision
            .unwrap()
            .choices
            .into_iter()
            .find(|a| matches!(a, Action::InstallInward(_)))
            .unwrap();
        activate(app, action);
    }
    fn bran_trash_and_finish(app: &mut App) {
        let action = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap()
            .decision
            .unwrap()
            .choices
            .into_iter()
            .find(|a| matches!(a, Action::TrashInstalled(_)))
            .unwrap();
        activate(app, action);
        activate(app, Action::FinishInstall);
    }
    fn bran_rez_and_finish(app: &mut App) {
        for _ in 0..16 {
            let view = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap();
            if view.run_result.is_some() {
                return;
            }
            if let Some(action) = view
                .decision
                .and_then(|d| d.choices.into_iter().find(|a| matches!(a, Action::Rez(_))))
            {
                activate(app, action);
            } else {
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .respond()
                    .unwrap();
                app.update();
            }
        }
        panic!("ordinary Brân run did not finish");
    }

    #[test]
    fn undo_button_restores_resources_and_refused_correction_does_not_stop_the_opponent() {
        use crate::game::EngineeringPosition;
        let mut position = EngineeringPosition::economy(42);
        position.active = Player::Runner;
        let mut play = Play::engineering(position, None, Player::Runner);
        play.admit();
        play.respond().unwrap();
        play.respond().unwrap();
        let mut app = interface_app();
        app.insert_resource(Controller(play));
        app.update();
        activate(&mut app, Action::GainCredit);
        let undo = |app: &mut App| {
            let world = app.world_mut();
            let entity = world
                .query::<(Entity, &ButtonCommand)>()
                .iter(world)
                .find_map(|(entity, command)| {
                    matches!(command, ButtonCommand::Undo(_)).then_some(entity)
                })
                .expect("visible Undo control");
            world.trigger(Activate { entity });
            app.update();
        };
        undo(&mut app);
        let view = app
            .world()
            .resource::<Controller>()
            .0
            .observation()
            .unwrap();
        assert_eq!((view.runner_credits, view.clicks[1]), (5, 4));
        activate(&mut app, Action::Draw);
        let after = app.world().resource::<Controller>().0.observation();
        undo(&mut app);
        assert_eq!(app.world().resource::<Controller>().0.observation(), after);
        assert!(app.world().resource::<Interface>().error.is_empty());
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains(" · Undo unavailable: new information")),
            "a refused click must explain the barrier in the visible phase/status line"
        );
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("new information"))
        );
    }

    #[test]
    fn native_buttons_finish_the_run_and_modal_state_blocks_underlying_choices() {
        let mut app = interface_app();
        let before = app.world().resource::<Controller>().0.observation();
        app.world_mut().resource_mut::<Interface>().history_open = true;
        bran_install(&mut app);
        assert_eq!(app.world().resource::<Controller>().0.observation(), before);
        app.world_mut().resource_mut::<Interface>().history_open = false;
        app.update();
        bran_install(&mut app);
        bran_trash_and_finish(&mut app);
        app.world_mut()
            .resource_mut::<Controller>()
            .0
            .respond()
            .unwrap();
        app.update();
        bran_rez_and_finish(&mut app);
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
            .filter_map(|(entity, name)| name.as_str().ends_with("Credit pool").then_some(entity))
            .collect();
        assert_eq!(token_roots.len(), 2);
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
        bran_install(&mut app);
        bran_trash_and_finish(&mut app);
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
        bran_install(&mut app);
        bran_trash_and_finish(&mut app);
        let before = app.world().resource::<Controller>().0.observation();
        let bytes = std::fs::read(&save).unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(unfolded_record_replacement::lock_path(
                &root.path().join("match.json"),
            ))
            .unwrap();
        lock.lock().unwrap();
        app.world_mut().resource_mut::<Interface>().respond_after = 0.;
        app.add_systems(Update, fixed_opponent.before(draw_interface));
        // The worker must return on a later frame; keep rendering while waiting
        // for the same observable save failure, rather than assuming sync work.
        let started = std::time::Instant::now();
        while app.world().resource::<Interface>().error.is_empty()
            && started.elapsed() < std::time::Duration::from_secs(2)
        {
            app.update();
            std::thread::yield_now();
        }
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
        let pile_probe = std::env::var_os("NETRUNNER_PILE_PROBE").is_some();
        let bran_probe = turns_save.is_none() && !pile_probe;
        let verified = assets::inspect_turns(Path::new(&root)).unwrap();
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
        if !bran_probe {
            let seat = if std::env::var("NETRUNNER_SEAT").as_deref() == Ok("corp") {
                Player::Corp
            } else {
                Player::Runner
            };
            let play = if pile_probe {
                use crate::game::{CardKind, EngineeringCard, EngineeringPosition, Zone};
                let mut position = EngineeringPosition::economy(3);
                position.active = seat;
                position.cards.clear();
                for (count, kind, zone) in [
                    (12, CardKind::Bran, Zone::Archives),
                    (22, CardKind::Bran, Zone::Rd),
                    (12, CardKind::SureGamble, Zone::Heap),
                    (18, CardKind::SureGamble, Zone::Stack),
                ] {
                    position
                        .cards
                        .extend((0..count).map(|_| EngineeringCard::new(kind, zone)));
                }
                Play::engineering(position, None, seat)
            } else {
                Play::load_turns(
                    SaveOptions {
                        path: turns_save.expect("ordinary save").into(),
                        recover_from: None,
                    },
                    seat,
                )
            };
            app.insert_resource(Controller(play));
        }
        app.world_mut().spawn((
            Window {
                resolution: (1600, 1000).into(),
                ..default()
            },
            PrimaryWindow,
        ));
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
                                physical_size: UVec2::new(1600, 760),
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
        if let Ok(pile) = std::env::var("NETRUNNER_INSPECT_PILE") {
            click_piece(&mut app, &format!("{pile} pile"));
            assert!(app.world().resource::<Interface>().inspected_pile.is_some());
        } else {
            let identity = app
                .world()
                .resource::<Controller>()
                .0
                .observation()
                .unwrap()
                .corp_identity;
            app.world_mut().resource_mut::<Interface>().inspected = Some(identity);
        }
        for _ in 0..30 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        capture(
            &mut app,
            target.clone(),
            output.with_extension("inspection.png"),
        );
        if let Some(entity) = inspection_button(&mut app, "Next card") {
            let before = app.world().resource::<Controller>().0.observation();
            app.world_mut().trigger(Activate { entity });
            for _ in 0..8 {
                app.update();
            }
            assert_eq!(
                app.world()
                    .resource::<Interface>()
                    .inspected_pile
                    .as_ref()
                    .unwrap()
                    .index,
                1
            );
            assert_eq!(app.world().resource::<Controller>().0.observation(), before);
            capture(
                &mut app,
                target.clone(),
                output.with_extension("inspection-next.png"),
            );
        }
        {
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
                    .query_filtered::<&mut RenderTarget, Or<(With<Camera3d>, With<IsDefaultUiCamera>)>>()
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
                    .query_filtered::<(&ComputedNode, &UiGlobalTransform, &ChildOf), With<InspectionImage>>()
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
                .query::<&LabelAnchor>()
                .iter(world)
                .find_map(counter_anchor)
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
                .query_filtered::<&bevy::ui::ComputedStackIndex, With<ActionDock>>()
                .single(world)
                .unwrap()
                .0;
            let (label_node, label_stack) = world
                .query::<(&LabelAnchor, &Node, &bevy::ui::ComputedStackIndex)>()
                .iter(world)
                .find(|(label, _, _)| counter_anchor(label).is_some_and(|(at, _)| at == token))
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
        if bran_probe {
            bran_install(&mut app);
            bran_trash_and_finish(&mut app);
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
            bran_rez_and_finish(&mut app);
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
            assert!(
                app.world_mut()
                    .resource_mut::<Controller>()
                    .0
                    .restart()
                    .is_err()
            );
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
