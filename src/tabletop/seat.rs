//! Netrunner's lawful observation and native controls on the shared agent seat.
//! Transport, pointer input, freshness guards and capture remain shared. This
//! adapter never submits a choice or triggers a button observer itself.
use super::*;
use bevy::{camera::RenderTarget, time::TimeUpdateStrategy};
use serde_json::{Value, json};
use std::time::Duration;
use unfolded_tabletop::{
    capture,
    seat::{
        self as shared, Focus, Game, Hit, Name, Observed, SeatKey,
        start::{self, Start},
    },
};

/// Netrunner's seat start: its marker, the shared limits and its words for an
/// admission that ran out of time. The order of the steps and the refusals are shared.
const START: Start = Start::new("NETRUNNER_AGENT_SEAT_DETACHED", |_, limit| {
    format!(
        "Required artwork and renderer did not admit the agent seat within {} seconds.",
        limit.as_secs()
    )
});

/// The ordinary table without a desktop window, on JSON lines or a held socket.
pub fn run(
    root: &Path,
    save: Option<SaveOptions>,
    turns: bool,
    human: Option<Player>,
    socket_path: Option<&Path>,
) -> Result<(), String> {
    START.run(socket_path, || {
        table(root, save, turns, human, UVec2::new(1600, 1000))
    })
}

/// The table, admitted, with an agent seated at it in this process.
#[cfg(test)]
fn open(
    root: &Path,
    save: Option<SaveOptions>,
    turns: bool,
    human: Option<Player>,
    size: UVec2,
) -> Result<(App, shared::Seat<Table>), String> {
    let seated = START.admit(table(root, save, turns, human, size)?)?;
    Ok((seated.app, seated.seat))
}

/// The windowless table with the seat's surface installed, before the seat opens it.
fn table(
    root: &Path,
    save: Option<SaveOptions>,
    turns: bool,
    human: Option<Player>,
    size: UVec2,
) -> Result<start::Table<Table>, String> {
    let verified = assets::inspect_turns(root).map_err(|error| error.to_string())?;
    let play = if turns {
        Play::load_turns(
            save.ok_or("Turn inspection requires an existing save.")?,
            human.unwrap_or(Player::Corp),
        )
    } else {
        Play::introductory(save, human)?
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: verified.root.to_string_lossy().into_owned(),
                ..default()
            })
            .set(capture::no_window())
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    )
    .insert_resource(Controller(play))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    install(&mut app, verified);
    // The UI camera covers the full screen; the 3D camera retains its native
    // partial viewport and renders beneath that UI onto the same image.
    capture::install::<Camera2d>(&mut app, size);
    app.add_systems(
        PostStartup,
        |mut commands: Commands,
         surface: Res<capture::Surface>,
         camera: Single<Entity, With<Camera3d>>| {
            commands
                .entity(*camera)
                .insert(RenderTarget::Image(surface.image().into()));
        },
    );
    Ok(start::Table::new(app, Table))
}

struct Table;

impl Game for Table {
    fn observe(&self, world: &mut World) -> Observed {
        let pending = self.unsettled(world);
        let controls = controls(world);
        let dock = action_dock_positions(world).ok();
        let viewport = viewport(world);
        let play = &world.resource::<Controller>().0;
        let ui = world.resource::<Interface>();
        Observed {
            observation: serde_json::to_value(play.observation())
                .expect("player observation serializes"),
            table: json!({
                "revision": play.revision().unwrap_or(0), "seat": play.human(),
                "needs_seat": play.needs_seat(),
                "settled": pending.is_empty(), "pending": pending,
                "viewport": viewport.map(|size| [size.x, size.y]),
                "modal": if ui.new_match_confirmation.is_some() { Some("new-match") } else if ui.history_open { Some("history") } else if ui.inspected.is_some() { Some("inspection") } else { None },
                "archived_match": play.archived_match(),
                "inspected": ui.inspected, "error": ui.error,
                "opponent_pending": play.opponent_pending(),
                "opponent_policy": play.opponent_policy().identity(),
                // The number of opponent legal alternatives depends on its
                // hidden hand, so keep that diagnostic field out of this seat.
                "opponent_report": play.opponent_report().map(|r| json!({
                    "policy": r.policy, "revision": r.revision,
                    "elapsed_nanoseconds": r.elapsed_nanoseconds,
                    "selector_invocations": r.selector_invocations,
                    "outcome": r.outcome, "committed": r.committed,
                })),
                "undo_unavailable": play.undo_reason(),
                "undo_feedback": ui.undo_feedback.then(|| play.undo_reason()).flatten(),
                "components": dock.map(|points| vec![json!({
                    "component":"action-dock", "label":"Action dock",
                    "position":[points[0].x, points[0].y], "on_screen":true
                })]).unwrap_or_default(),
                "controls": controls.iter().map(|control| {
                    let (choice, component) = match &control.name {
                        Name::Choice(name) => (Some(name), None),
                        Name::Component(name) => (None, Some(name)),
                    };
                    json!({"choice":choice,"component":component,"label":control.label,
                        "position":[control.target.position.x, control.target.position.y],
                        "on_screen":control.target.on_screen})
                }).collect::<Vec<_>>()
            }),
        }
    }

    fn revision(&self, world: &mut World) -> u64 {
        world.resource::<Controller>().0.revision().unwrap_or(0)
    }

    fn resolve(&self, world: &mut World, name: &Name) -> Result<Vec<Vec2>, String> {
        if name == &Name::Component("action-dock".into()) {
            return action_dock_positions(world);
        }
        let revision = self.revision(world);
        let mut found = controls(world)
            .into_iter()
            .filter(|control| &control.name == name);
        let control = found
            .next()
            .ok_or_else(|| format!("No shown control is named {name:?}."))?;
        if found.next().is_some() {
            return Err(format!("More than one shown control is named {name:?}."));
        }
        if matches!(control.command, ButtonCommand::Choose(shown, _) | ButtonCommand::ChooseInspected(shown, _) | ButtonCommand::Undo(shown) | ButtonCommand::NewMatch(shown) | ButtonCommand::ConfirmNewMatch(shown) if shown != revision)
        {
            return Err(
                "The control belongs to an earlier decision. Settle and observe again.".into(),
            );
        }
        if !control.target.on_screen {
            return Err(format!(
                "{name:?} is clipped or off screen; bring it into view first."
            ));
        }
        Ok(vec![control.target.position])
    }

    fn identify(&self, world: &mut World, entity: Entity) -> Hit {
        let mut current = Some(entity);
        while let Some(ancestor) = current {
            if let Some(command) = world.get::<ButtonCommand>(ancestor) {
                let name = command_name(*command);
                return Hit {
                    description: json!({"control":format!("{name:?}"),"label":first_text(world, ancestor)}),
                    name: Some(name),
                };
            }
            if world.get::<ActionDock>(ancestor).is_some() {
                let ui = world.resource::<Interface>();
                if ui.inspected.is_none() && !ui.history_open {
                    return Hit {
                        name: Some(Name::Component("action-dock".into())),
                        description: json!({"component":"action-dock","label":"Action dock"}),
                    };
                }
            }
            current = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
        }
        Hit {
            name: None,
            description: json!({"surface":"table or overlay","text":first_text(world, entity)}),
        }
    }

    fn focus(&self, _world: &mut World) -> Focus {
        Focus::Nothing
    }

    fn unsettled(&self, world: &mut World) -> Vec<String> {
        let play = &world.resource::<Controller>().0;
        if play.admission() != &Admission::Ready {
            return vec!["artwork or save admission".into()];
        }
        let ui = world.resource::<Interface>();
        let modal =
            ui.history_open || ui.inspected.is_some() || ui.new_match_confirmation.is_some();
        if !modal && ui.error.is_empty() && play.opponent_pending() {
            return vec!["native opponent response (including its presentation delay)".into()];
        }
        let revision = play.revision().unwrap_or(0);
        let expected = (!modal)
            .then(|| play.observation().and_then(|view| view.decision))
            .flatten();
        let viewport = viewport(world);
        let shown: Vec<_> = world
            .query::<(Entity, &ButtonCommand)>()
            .iter(world)
            .map(|(entity, command)| (entity, *command))
            .collect();
        if shown.iter().any(|(entity, command)| shared::ui_target(world, *entity, viewport).is_none()
            || matches!(command, ButtonCommand::Choose(shown, _) | ButtonCommand::ChooseInspected(shown, _) | ButtonCommand::Undo(shown) | ButtonCommand::NewMatch(shown) | ButtonCommand::ConfirmNewMatch(shown) if *shown != revision))
            || expected.is_some_and(|decision| decision.choices.iter().any(|action| !shown.iter().any(|(_, command)| matches!(command, ButtonCommand::Choose(shown, offered) if *shown == revision && offered == action)))) {
            return vec!["native control redraw or layout".into()];
        }
        Vec::new()
    }

    fn advance(&self, world: &mut World, by: Duration) {
        *world.resource_mut::<TimeUpdateStrategy>() = TimeUpdateStrategy::ManualDuration(by);
    }
    fn admission(&self, world: &World) -> shared::Admission {
        match world.resource::<Controller>().0.admission() {
            Admission::Ready => shared::Admission::Admitted,
            Admission::Loading => shared::Admission::Checking,
            Admission::Refused(reason) => shared::Admission::Refused(reason.clone()),
        }
    }
    fn pipelines_ready(&self, world: &World) -> bool {
        world
            .resource::<Uploaded>()
            .pipelines_ready
            .load(Ordering::Acquire)
    }
    fn keys(&self) -> &[SeatKey] {
        &[]
    }
}

fn action_name(action: Action) -> String {
    match serde_json::to_value(action).expect("action serializes") {
        Value::String(name) => name,
        value => value.to_string(),
    }
}

fn command_name(command: ButtonCommand) -> Name {
    match command {
        ButtonCommand::Choose(_, action) | ButtonCommand::ChooseInspected(_, action) => {
            Name::Choice(action_name(action))
        }
        command => Name::Component(
            match command {
                ButtonCommand::CloseInspection => "close-inspection",
                ButtonCommand::PreviousCard => "previous-card",
                ButtonCommand::NextCard => "next-card",
                ButtonCommand::History => "history",
                ButtonCommand::Undo(_) => "undo",
                ButtonCommand::NewMatch(_) => "new-match",
                ButtonCommand::ConfirmNewMatch(_) => "confirm-new-match",
                ButtonCommand::CancelNewMatch => "cancel-new-match",
                ButtonCommand::SelectSeat(Player::Corp) => "play-corp",
                ButtonCommand::SelectSeat(Player::Runner) => "play-runner",
                ButtonCommand::Quit => "quit",
                _ => unreachable!(),
            }
            .into(),
        ),
    }
}

fn viewport(world: &mut World) -> Option<Vec2> {
    world
        .query_filtered::<&Camera, With<IsDefaultUiCamera>>()
        .single(world)
        .ok()
        .and_then(Camera::logical_viewport_size)
}

/// Points on the real scroll panel, including its padding when a button covers
/// the center. The shared seat must still verify the actual picked identity.
fn action_dock_positions(world: &mut World) -> Result<Vec<Vec2>, String> {
    let ui = world.resource::<Interface>();
    if ui.inspected.is_some() || ui.history_open {
        return Err("The action dock is covered by an inspection or history.".into());
    }
    let viewport = viewport(world).ok_or("The action dock has no viewport.")?;
    let entity = world
        .query_filtered::<Entity, With<ActionDock>>()
        .single(world)
        .map_err(|_| "There is no single shown action dock.")?;
    let target = shared::ui_target(world, entity, Some(viewport))
        .filter(|target| target.on_screen)
        .ok_or("The action dock has no visible layout.")?;
    let node = world.get::<ComputedNode>(entity).expect("visible layout");
    let transform = world
        .get::<UiGlobalTransform>(entity)
        .expect("visible layout");
    let half = node.size() / 2.;
    let clip = world.get::<bevy::ui::CalculatedClip>(entity);
    let mut points = vec![target.position];
    for offset in [
        Vec2::new(-half.x + node.padding.min_inset.x / 2., 0.),
        Vec2::new(half.x - node.padding.max_inset.x / 2., 0.),
        Vec2::new(0., -half.y + node.padding.min_inset.y / 2.),
        Vec2::new(0., half.y - node.padding.max_inset.y / 2.),
    ] {
        let physical = transform.transform_point2(offset);
        let point = physical * node.inverse_scale_factor;
        if point.is_finite()
            && point.cmpge(Vec2::ZERO).all()
            && point.cmplt(viewport).all()
            && clip.is_none_or(|clip| clip.contains_point(physical))
        {
            points.push(point);
        }
    }
    Ok(points)
}

fn first_text(world: &World, entity: Entity) -> Option<String> {
    if let Some(text) = world.get::<Text>(entity) {
        return Some(text.0.clone());
    }
    world
        .get::<Children>(entity)?
        .iter()
        .find_map(|child| first_text(world, child))
}

struct Control {
    command: ButtonCommand,
    name: Name,
    label: String,
    target: shared::UiTarget,
}

fn controls(world: &mut World) -> Vec<Control> {
    let viewport = viewport(world);
    let entities: Vec<_> = world
        .query::<(Entity, &ButtonCommand)>()
        .iter(world)
        .map(|(entity, command)| (entity, *command))
        .collect();
    let ui = world.resource::<Interface>();
    entities
        .into_iter()
        .filter(|(_, command)| match command {
            ButtonCommand::Choose(..)
            | ButtonCommand::Undo(_)
            | ButtonCommand::NewMatch(_)
            | ButtonCommand::History => {
                ui.inspected.is_none() && !ui.history_open && ui.new_match_confirmation.is_none()
            }
            ButtonCommand::ChooseInspected(..) => ui.inspected.is_some() && !ui.history_open,
            _ => true,
        })
        .filter_map(|(entity, command)| {
            Some(Control {
                command,
                name: command_name(command),
                label: first_text(world, entity).unwrap_or_default(),
                target: shared::ui_target(world, entity, viewport)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::io::Cursor;

    #[test]
    #[ignore = "requires NETRUNNER_ART_ROOT, NETRUNNER_TURNS_SAVE (ordinary Runner action phase) and Vulkan; opens no desktop window"]
    fn rendered_mat_atlas_preserves_seat_actions_and_public_count_updates() {
        let root = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
        let source = std::env::var_os("NETRUNNER_TURNS_SAVE").expect("NETRUNNER_TURNS_SAVE");
        let original = std::fs::read(&source).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("printed.json");
        std::fs::write(&save, &original).unwrap();
        let size = UVec2::new(1280, 800);
        let (mut app, mut seat) = open(
            Path::new(&root),
            Some(SaveOptions {
                path: save,
                recover_from: None,
            }),
            true,
            Some(Player::Runner),
            size,
        )
        .unwrap();
        fn request(app: &mut App, seat: &mut shared::Seat<Table>, value: Value) -> Value {
            let mut output = Vec::new();
            seat.serve(app, Cursor::new(format!("{value}\n")), &mut output)
                .unwrap();
            serde_json::from_slice(
                output
                    .split(|b| *b == b'\n')
                    .rfind(|line| !line.is_empty())
                    .unwrap(),
            )
            .unwrap()
        }
        let before = request(&mut app, &mut seat, json!({"op":"observe"}));
        assert_eq!(before["table"]["viewport"], json!([1280., 800.]));
        if let Some(path) = std::env::var_os("NETRUNNER_RENDER_CAPTURE") {
            let captured = request(
                &mut app,
                &mut seat,
                json!({"op":"capture", "path":std::path::PathBuf::from(path)}),
            );
            assert_eq!(captured["status"], "captured", "{captured}");
        }
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<Camera2d>>()
                .iter(app.world())
                .count()
                >= 2,
            "exercise the main camera and printed atlas together"
        );
        let mut revision = before["table"]["revision"].as_u64().unwrap();
        for action in ["GainCredit", "Draw"] {
            let changed = request(
                &mut app,
                &mut seat,
                json!({"op":"activate", "revision":revision,"choice":action}),
            );
            assert_eq!(changed["status"], "activated", "{changed}");
            let settled = request(&mut app, &mut seat, json!({"op":"settle"}));
            assert_eq!(settled["status"], "settled", "{settled}");
            revision = settled["table"]["revision"].as_u64().unwrap();
        }
        let after = request(&mut app, &mut seat, json!({"op":"observe"}));
        assert_eq!(
            after["observation"]["runner_credits"].as_u64().unwrap(),
            before["observation"]["runner_credits"].as_u64().unwrap() + 1
        );
        let grip = before["observation"]["grip_count"].as_u64().unwrap() + 1;
        let stack = before["observation"]["stack_count"].as_u64().unwrap() - 1;
        for expected in [
            format!("Printed mat label: Grip\n{grip}"),
            format!("Printed mat label: Stack\n{stack}"),
        ] {
            assert!(
                app.world().iter_entities().any(|entity| entity
                    .get::<bevy::prelude::Name>()
                    .is_some_and(|name| name.as_str() == expected)),
                "{expected}"
            );
        }
        assert_eq!(std::fs::read(source).unwrap(), original);
    }

    #[test]
    fn dock_identity_preserves_buttons_and_refuses_modal_coverage() {
        let mut world = World::new();
        world.init_resource::<Interface>();
        let dock = world.spawn(ActionDock).id();
        let text = world
            .spawn((Text::new("Payment details"), ChildOf(dock)))
            .id();
        let button = world.spawn((ButtonCommand::Quit, ChildOf(dock))).id();
        let button_text = world.spawn((Text::new("Quit"), ChildOf(button))).id();
        assert_eq!(
            Table.identify(&mut world, text).name,
            Some(Name::Component("action-dock".into()))
        );
        assert_eq!(
            Table.identify(&mut world, button_text).name,
            Some(Name::Component("quit".into()))
        );
        let overlay = world.spawn(Text::new("History overlay")).id();
        assert!(Table.identify(&mut world, overlay).name.is_none());
        world.resource_mut::<Interface>().history_open = true;
        assert!(Table.identify(&mut world, text).name.is_none());
        assert!(
            Table
                .resolve(&mut world, &Name::Component("action-dock".into()))
                .unwrap_err()
                .contains("covered")
        );
    }

    #[test]
    #[ignore = "requires NETRUNNER_ART_ROOT, NETRUNNER_SEAT_PAYMENT_SAVE (ordinary Runner payment with all buttons clipped) and Vulkan; opens no desktop window"]
    fn rendered_payment_dock_scroll_reaches_buttons_without_changing_game_or_camera() {
        let root = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
        let source =
            std::env::var_os("NETRUNNER_SEAT_PAYMENT_SAVE").expect("NETRUNNER_SEAT_PAYMENT_SAVE");
        let original = std::fs::read(source).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("payment.json");
        std::fs::write(&save, &original).unwrap();
        let (mut app, mut seat) = open(
            Path::new(&root),
            Some(SaveOptions {
                path: save.clone(),
                recover_from: None,
            }),
            false,
            None,
            UVec2::new(1600, 1000),
        )
        .unwrap();
        let before = Table.observe(app.world_mut());
        assert_eq!(
            before.observation["phase"],
            "Choose pool / Overclock payment"
        );
        assert!(
            before.table["controls"]
                .as_array()
                .unwrap()
                .iter()
                .all(|control| control["on_screen"] == false)
        );
        let revision = before.table["revision"].as_u64().unwrap();
        let camera_before = *app
            .world_mut()
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(app.world())
            .unwrap();
        let mut output = Vec::new();
        seat.serve(
            &mut app,
            Cursor::new(format!(
                "{}\n",
                json!({
                    "op": "scroll", "revision": revision, "component": "action-dock", "y": -4
                })
            )),
            &mut output,
        )
        .unwrap();
        let scrolled: Value = serde_json::from_slice(
            output
                .split(|byte| *byte == b'\n')
                .rfind(|line| !line.is_empty())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(scrolled["status"], "scrolled", "{scrolled}");
        assert_eq!(scrolled["observation"], before.observation);
        assert!(
            scrolled["table"]["controls"]
                .as_array()
                .unwrap()
                .iter()
                .all(|control| control["on_screen"] == true),
            "{scrolled}"
        );
        let camera_after = *app
            .world_mut()
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(app.world())
            .unwrap();
        assert_eq!(camera_after, camera_before);
        assert_eq!(std::fs::read(save).unwrap(), original);
    }

    #[test]
    #[ignore = "requires NETRUNNER_ART_ROOT, NETRUNNER_SEAT_DRAW_SAVE (draw20 step 27), NETRUNNER_SEAT_CAPTURE_DIR and Vulkan; opens no desktop window"]
    fn rendered_large_hand_scroll_reaches_lower_actions_and_commits_them() {
        use bevy::{
            input::{
                mouse::{MouseScrollUnit, MouseWheel},
                touch::TouchPhase,
            },
            picking::pointer::{Location, PointerAction, PointerId, PointerInput},
        };
        fn request(app: &mut App, seat: &mut shared::Seat<Table>, request: Value) -> Value {
            let mut output = Vec::new();
            seat.serve(app, Cursor::new(format!("{request}\n")), &mut output)
                .unwrap();
            serde_json::from_slice(
                output
                    .split(|b| *b == b'\n')
                    .rfind(|line| !line.is_empty())
                    .unwrap(),
            )
            .unwrap()
        }
        let root = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
        let source =
            std::env::var_os("NETRUNNER_SEAT_DRAW_SAVE").expect("NETRUNNER_SEAT_DRAW_SAVE");
        let captures = std::path::PathBuf::from(
            std::env::var_os("NETRUNNER_SEAT_CAPTURE_DIR").expect("NETRUNNER_SEAT_CAPTURE_DIR"),
        );
        std::fs::create_dir_all(&captures).unwrap();
        let original = std::fs::read(&source).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("draw.json");
        std::fs::write(&save, &original).unwrap();
        let (mut app, mut seat) = open(
            Path::new(&root),
            Some(SaveOptions {
                path: save.clone(),
                recover_from: None,
            }),
            true,
            Some(Player::Runner),
            UVec2::new(1600, 1000),
        )
        .unwrap();
        let initial = request(&mut app, &mut seat, json!({"op":"observe"}));
        assert_eq!(initial["observation"]["grip_count"], 9);
        assert_eq!(initial["observation"]["runner_credits"], 8);
        let revision = initial["table"]["revision"].as_u64().unwrap();
        let lower = initial["table"]["controls"]
            .as_array()
            .unwrap()
            .iter()
            .find(|control| {
                control["label"]
                    .as_str()
                    .is_some_and(|label| label.starts_with("Play Overclock"))
            })
            .unwrap();
        let choice = lower["choice"].as_str().unwrap().to_owned();
        assert_eq!(lower["on_screen"], false, "witness must need scrolling");
        assert_eq!(
            request(
                &mut app,
                &mut seat,
                json!({"op":"capture", "path": captures.join("nine-card-before-scroll.png")})
            )["status"],
            "captured"
        );
        let pointed = request(
            &mut app,
            &mut seat,
            json!({"op":"point", "revision":revision, "choice":"GainCredit"}),
        );
        assert_eq!(pointed["status"], "pointed", "{pointed}");
        let position = Vec2::new(
            pointed["position"][0].as_f64().unwrap() as f32,
            pointed["position"][1].as_f64().unwrap() as f32,
        );
        let world = app.world_mut();
        let window = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
            .unwrap();
        let camera = world.resource::<capture::Surface>().camera().unwrap();
        let target = world
            .get::<RenderTarget>(camera)
            .unwrap()
            .normalize(Some(window))
            .unwrap();
        // The windowless seat's normal pointer-input seam: this is exactly the
        // scroll action produced from MouseWheel by Bevy's window input. Keep
        // the rendered image target, as the seat does for moves and clicks.
        world.write_message(MouseWheel {
            unit: MouseScrollUnit::Pixel,
            x: 0.,
            y: -800.,
            window,
            phase: TouchPhase::Moved,
        });
        world.write_message(PointerInput::new(
            PointerId::Mouse,
            Location { target, position },
            PointerAction::Scroll {
                unit: MouseScrollUnit::Pixel,
                x: 0.,
                y: -800.,
                phase: TouchPhase::Moved,
            },
        ));
        for _ in 0..6 {
            app.update();
        }
        let scrolled = request(&mut app, &mut seat, json!({"op":"observe"}));
        assert_eq!(
            scrolled["observation"], initial["observation"],
            "scrolling cannot change rules state"
        );
        assert_eq!(std::fs::read(&save).unwrap(), original);
        for name in [
            Name::Choice(choice.clone()),
            Name::Component("history".into()),
            Name::Component("quit".into()),
        ] {
            let control = controls(app.world_mut())
                .into_iter()
                .find(|control| control.name == name)
                .unwrap();
            assert!(control.target.on_screen, "{name:?} remains clipped");
            assert!(
                control.target.position.y < 977.,
                "{name:?} bottom would be clipped"
            );
        }
        assert_eq!(
            request(
                &mut app,
                &mut seat,
                json!({"op":"capture", "path": captures.join("nine-card-after-scroll.png")})
            )["status"],
            "captured"
        );
        let played = request(
            &mut app,
            &mut seat,
            json!({"op":"activate", "revision":revision, "choice":choice}),
        );
        assert_eq!(played["status"], "activated", "{played}");
        assert_eq!(played["observation"]["runner_credits"], 7);
        assert_eq!(played["observation"]["clicks"][1], 0);
        assert_eq!(played["observation"]["grip_count"], 8);
        assert_eq!(played["observation"]["phase"], "Overclock · choose server");
        assert_eq!(std::fs::read(&source).unwrap(), original);
        std::fs::write(
            captures.join("scroll-observations.json"),
            serde_json::to_vec_pretty(&json!({"before":initial,"after":scrolled,"played":played}))
                .unwrap(),
        )
        .unwrap();
    }

    #[test]
    #[ignore = "requires NETRUNNER_ART_ROOT, NETRUNNER_SEAT_INSTALLATION_SAVE, NETRUNNER_SEAT_CAPTURE_DIR and Vulkan; opens no desktop window"]
    fn rendered_installation_controls_commit_at_supported_size() {
        let root = std::env::var_os("NETRUNNER_ART_ROOT").expect("NETRUNNER_ART_ROOT");
        let source = std::env::var_os("NETRUNNER_SEAT_INSTALLATION_SAVE")
            .expect("NETRUNNER_SEAT_INSTALLATION_SAVE");
        let captures = std::path::PathBuf::from(
            std::env::var_os("NETRUNNER_SEAT_CAPTURE_DIR").expect("NETRUNNER_SEAT_CAPTURE_DIR"),
        );
        let original = std::fs::read(&source).unwrap();
        let size = match std::env::var("NETRUNNER_SEAT_TEST_WIDTH").as_deref() {
            Ok("1280") => UVec2::new(1280, 800),
            Ok("1600") | Err(_) => UVec2::new(1600, 1000),
            _ => panic!("NETRUNNER_SEAT_TEST_WIDTH must be 1280 or 1600"),
        };
        {
            let directory = tempfile::tempdir().unwrap();
            let save = directory.path().join("installation.json");
            std::fs::write(&save, &original).unwrap();
            let (mut app, mut seat) = open(
                Path::new(&root),
                Some(SaveOptions {
                    path: save.clone(),
                    recover_from: None,
                }),
                true,
                Some(Player::Corp),
                size,
            )
            .unwrap();
            let mut request = |request: Value| {
                let mut output = Vec::new();
                seat.serve(&mut app, Cursor::new(format!("{request}\n")), &mut output)
                    .unwrap();
                serde_json::from_slice::<Value>(
                    output
                        .split(|b| *b == b'\n')
                        .rfind(|line| !line.is_empty())
                        .unwrap(),
                )
                .unwrap()
            };
            let initial = request(json!({"op":"observe"}));
            assert_eq!(initial["table"]["revision"], 13);
            assert_eq!(initial["observation"]["corp_credits"], 6);
            let before = captures.join(format!("{}-before.png", size.x));
            assert_eq!(
                request(json!({"op":"capture","path":before}))["status"],
                "captured"
            );
            for action in [r#"{"TrashInstalled":2}"#, "FinishInstall"] {
                assert!(
                    initial["table"]["controls"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|control| control["choice"] == action && control["on_screen"] == true),
                    "{initial}"
                );
            }
            assert_eq!(
                request(json!({"op":"activate","revision":12,"choice":r#"{"TrashInstalled":2}"#}))
                    ["status"],
                "refused"
            );
            assert_eq!(std::fs::read(&save).unwrap(), original);
            assert_eq!(
                request(json!({"op":"activate","revision":13,"component":"history"}))["status"],
                "activated"
            );
            assert_eq!(
                request(json!({"op":"activate","revision":13,"choice":r#"{"TrashInstalled":2}"#}))
                    ["status"],
                "refused"
            );
            assert_eq!(std::fs::read(&save).unwrap(), original);
            assert_eq!(
                request(json!({"op":"activate","revision":13,"component":"close-inspection"}))["status"],
                "activated"
            );
            let trashed =
                request(json!({"op":"activate","revision":13,"choice":r#"{"TrashInstalled":2}"#}));
            assert_eq!(trashed["table"]["revision"], 14, "{trashed}");
            assert!(
                trashed["observation"]["archives"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instance"] == 2)
            );
            assert_eq!(
                request(
                    json!({"op":"activate","revision":14,"choice":action_name(Action::FinishInstall)})
                )["status"],
                "activated"
            );
            let approach = request(json!({"op":"settle"}));
            assert_eq!(approach["status"], "settled", "{approach}");
            assert_eq!(approach["table"]["revision"], 21);
            assert_eq!(approach["observation"]["corp_credits"], 6);
            assert_eq!(approach["observation"]["runner_credits"], 7);
            let rez = approach["table"]["controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|control| control["choice"] == r#"{"Rez":3}"#)
                .unwrap();
            assert!(
                rez["label"].as_str().unwrap().contains("6 credits"),
                "{rez}"
            );
            let after = captures.join(format!("{}-approach.png", size.x));
            assert_eq!(
                request(json!({"op":"capture","path":after}))["status"],
                "captured"
            );
            let rezzed = request(json!({"op":"activate","revision":21,"choice":r#"{"Rez":3}"#}));
            assert_eq!(rezzed["observation"]["corp_credits"], 0, "{rezzed}");
            assert_eq!(rezzed["observation"]["run"]["ice"]["rezzed"], true);
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}
