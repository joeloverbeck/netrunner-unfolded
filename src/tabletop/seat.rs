//! Netrunner's lawful observation and native controls on the shared agent seat.
//! Transport, pointer input, freshness guards and capture remain shared. This
//! adapter never submits a choice or triggers a button observer itself.
use super::*;
use bevy::{camera::RenderTarget, time::TimeUpdateStrategy};
use serde_json::{Value, json};
use std::time::Duration;
use unfolded_tabletop::{
    capture,
    seat::{self as shared, Focus, Game, Hit, Name, Observed, Seat, SeatKey, socket},
};

const DETACHED: &str = "NETRUNNER_AGENT_SEAT_DETACHED";

/// The ordinary table without a desktop window, on JSON lines or a held socket.
pub fn run(
    root: &Path,
    save: Option<SaveOptions>,
    turns: bool,
    human: Player,
    socket_path: Option<&Path>,
) -> Result<(), String> {
    let listener = if let Some(path) = socket_path {
        if !socket::detached(DETACHED) {
            return socket::launch(path, DETACHED).or_else(shared::refuse);
        }
        Some(socket::detach(path, DETACHED).or_else(shared::refuse)?)
    } else {
        None
    };
    let (mut app, mut seat) =
        open(root, save, turns, human, UVec2::new(1600, 1000)).or_else(shared::refuse)?;
    if let Some(listener) = listener {
        socket::announce(&seat.ready(&mut app))?;
        listener.serve(&mut seat, &mut app, Duration::from_secs(30 * 60))
    } else {
        seat.serve(&mut app, std::io::stdin().lock(), std::io::stdout().lock())
    }
}

fn open(
    root: &Path,
    save: Option<SaveOptions>,
    turns: bool,
    human: Player,
    size: UVec2,
) -> Result<(App, Seat<Table>), String> {
    let verified = if turns {
        assets::inspect_turns(root)
    } else {
        assets::inspect(root)
    }
    .map_err(|error| error.to_string())?;
    let play = if turns {
        Play::load_turns(
            save.ok_or("Turn inspection requires an existing save.")?,
            human,
        )
    } else {
        Play::new(3, save)
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
    let mut seat = Seat::open(Table, &mut app)?;
    seat.admit(&mut app, Duration::from_secs(120))
        .map_err(|error| match error {
            shared::Unadmitted::Refused(reason) => reason,
            shared::Unadmitted::Unfinished => {
                "Required artwork and renderer did not admit the agent seat within 120 seconds."
                    .into()
            }
        })?;
    Ok((app, seat))
}

struct Table;

impl Game for Table {
    fn observe(&self, world: &mut World) -> Observed {
        let pending = self.unsettled(world);
        let controls = controls(world);
        let viewport = viewport(world);
        let play = &world.resource::<Controller>().0;
        let ui = world.resource::<Interface>();
        Observed {
            observation: serde_json::to_value(play.observation())
                .expect("player observation serializes"),
            table: json!({
                "revision": play.revision().unwrap_or(0), "seat": play.human(),
                "settled": pending.is_empty(), "pending": pending,
                "viewport": viewport.map(|size| [size.x, size.y]),
                "modal": if ui.history_open { Some("history") } else if ui.inspected.is_some() { Some("inspection") } else { None },
                "inspected": ui.inspected, "error": ui.error,
                "opponent_pending": play.opponent_pending(),
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
        if matches!(control.command, ButtonCommand::Choose(shown, _) | ButtonCommand::ChooseInspected(shown, _) if shown != revision)
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
        let modal = ui.history_open || ui.inspected.is_some();
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
            || matches!(command, ButtonCommand::Choose(shown, _) | ButtonCommand::ChooseInspected(shown, _) if *shown != revision))
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
                ButtonCommand::Restart => "restart",
                ButtonCommand::Quit => "quit",
                _ => unreachable!(),
            }
            .into(),
        ),
    }
}

fn viewport(world: &mut World) -> Option<Vec2> {
    world
        .query_filtered::<&Camera, With<Camera2d>>()
        .single(world)
        .ok()
        .and_then(Camera::logical_viewport_size)
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
            ButtonCommand::Choose(..) | ButtonCommand::History | ButtonCommand::Restart => {
                ui.inspected.is_none() && !ui.history_open
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
                Player::Corp,
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
