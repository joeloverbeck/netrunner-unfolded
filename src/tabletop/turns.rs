//! Netrunner-owned placement and input policy for the selected open table.
//! Geometry and camera movement come from the pinned shared renderer.
use super::*;
use crate::game::{CardId, Server};
use bevy::input::mouse::AccumulatedMouseScroll;
use unfolded_tabletop::camera::{Capture, FrameInput, Orbit, wheel_steps};

pub(super) fn is_ordinary(controller: &Controller) -> bool {
    controller.0.admission() == &Admission::Ready
        && controller
            .0
            .observation()
            .is_some_and(|view| view.active_player.is_some())
}

pub(super) fn back(owner: Player) -> &'static str {
    match owner {
        Player::Corp => "backs/corp.png",
        Player::Runner => "backs/runner.png",
    }
}

pub(super) fn action_card(action: Action) -> Option<CardId> {
    match action {
        Action::PlayCard(id)
        | Action::InstallCard(id)
        | Action::TrashInstalled(id)
        | Action::Advance(id)
        | Action::Score(id)
        | Action::Rez(id)
        | Action::SelectTarget(id)
        | Action::ToggleDiscard(id)
        | Action::InstallInward(id)
        | Action::Boost(id)
        | Action::Break { breaker: id, .. } => Some(id),
        _ => None,
    }
}

fn color(player: Player) -> Color {
    match player {
        Player::Corp => Color::srgb_u8(175, 199, 255),
        Player::Runner => Color::srgb_u8(255, 192, 170),
    }
}

fn mat_color(player: Player) -> Color {
    match player {
        Player::Corp => Color::srgb_u8(36, 61, 120),
        Player::Runner => Color::srgb_u8(139, 48, 40),
    }
}

fn table_point(seat: Player, x: f32, y: f32, z: f32) -> Vec3 {
    if seat == Player::Corp {
        Vec3::new(-x, y, -z)
    } else {
        Vec3::new(x, y, z)
    }
}

fn card_transform(seat: Player, owner: Player, x: f32, z: f32, ice: bool) -> Transform {
    let rotation = if seat == owner {
        0.
    } else {
        std::f32::consts::PI
    } + if ice { std::f32::consts::FRAC_PI_2 } else { 0. };
    Transform::from_translation(table_point(seat, x, 0.08, z))
        .with_rotation(Quat::from_rotation_y(rotation))
}

#[derive(Component)]
pub(super) struct BoardLabel(Vec3);

fn board_label(commands: &mut Commands, text: String, position: Vec3, tint: Color) {
    commands.spawn((
        BoardPiece,
        BoardLabel(position),
        GlobalZIndex(-1),
        Text::new(text),
        TextFont {
            font_size: 16.0.into(),
            ..default()
        },
        TextColor(tint),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

pub(super) fn position_labels(
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&BoardLabel, &mut Node, &mut Visibility)>,
) {
    let (camera, transform) = *camera;
    for (anchor, mut node, mut visible) in &mut labels {
        if let Ok(point) = camera.world_to_viewport(transform, anchor.0)
            && camera
                .logical_viewport_rect()
                .is_some_and(|rect| rect.contains(point))
        {
            node.left = px(point.x);
            node.top = px(point.y);
            *visible = Visibility::Visible;
        } else {
            *visible = Visibility::Hidden;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window, With<PrimaryWindow>>,
    controller: Res<Controller>,
    ui: Res<Interface>,
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    nodes: Query<(), With<Node>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    buttons_ui: Query<(), With<UiButton>>,
) {
    if !is_ordinary(&controller) {
        return;
    }
    let blocked =
        controller.0.admission() != &Admission::Ready || ui.inspected.is_some() || ui.history_open;
    let over_ui = window
        .cursor_position()
        .is_some_and(|p| p.y >= window.height() * 0.76)
        || hover
            .as_ref()
            .and_then(|h| h.get(&bevy::picking::pointer::PointerId::Mouse))
            .is_some_and(|hits| hits.keys().any(|entity| nodes.contains(*entity)));
    let step = orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: wheel_steps(&scroll),
        dt: time.delta_secs(),
        capture: Capture {
            pointer_blocked: blocked,
            over_ui,
            keyboard: blocked
                || focus
                    .as_ref()
                    .and_then(|f| f.get())
                    .is_some_and(|entity| buttons_ui.contains(entity)),
            pan: Some(blocked),
            wheel: blocked || over_ui,
        },
    });
    for mut camera in &mut cameras {
        *camera = step.transform;
    }
}

pub(super) fn draw_dock(commands: &mut Commands, view: &Observation, ui: &Interface, seat: Player) {
    commands.spawn((InterfaceRoot, Node { position_type: PositionType::Absolute, left: px(16.), top: px(10.), right: px(16.), justify_content: JustifyContent::SpaceBetween, ..default() }, Pickable::IGNORE))
        .with_children(|row| {
            label(row, &format!("You: {seat:?} · Turn {} · {:?} active", view.turn_number, view.active_player.unwrap()), 21., color(seat));
            label(row, &format!("Corp {} credits · {} clicks · {} / 6 points    Runner {} credits · {} clicks · {} / 6 points", view.corp_credits, view.clicks[0], view.agenda_points[0], view.runner_credits, view.clicks[1], view.agenda_points[1]), 18., Color::WHITE);
        });
    commands
        .spawn((
            InterfaceRoot,
            ScrollArea,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                top: percent(76.),
                width: percent(100.),
                height: percent(24.),
                padding: UiRect::all(px(16.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.065, 0.09, 0.12)),
        ))
        .with_children(|dock| {
            label(dock, view.phase, 22., color(seat));
            if view.run.is_none() && view.run_result == Some(crate::game::RunResult::EndedByIce) {
                label(dock, "ICE ended the run. Unspent Overclock credits returned to the bank.", 18., color(Player::Runner));
            }
            if view.run.is_none() && view.run_result == Some(crate::game::RunResult::SuccessfulAccess) {
                label(dock, "Last run completed: successful HQ access. Continue the turn below.", 18., color(Player::Runner));
            }
            if view.run_result == Some(crate::game::RunResult::EndedByManegarm) {
                label(dock, "Manegarm ended the run because payment was declined. No Manegarm credits or clicks were paid.", 18., color(Player::Runner));
            }
            if let Some(card) = &view.accessed
                && !matches!(card.printing, Some("30042" | "30067"))
            {
                label(dock, card.title.as_deref().unwrap_or("Accessed card"), 18., Color::WHITE);
            }
            if let Some(run) = &view.run {
                if !run.known_hq.is_empty() {
                    label(dock, &format!("Already accessed, still in HQ: {}", run.known_hq.iter().filter_map(|c| c.title.as_deref()).collect::<Vec<_>>().join(", ")), 16., Color::WHITE);
                }
                if let Some(remaining) = run.hq_access_remaining {
                    label(dock, &format!("HQ cards still permitted: {remaining}. Root access does not use this limit."), 16., Color::WHITE);
                }
                if let Some(purpose) = &run.payment_for {
                    label(dock, &format!("Pay for {purpose}"), 18., Color::WHITE);
                }
                let title = run.ice.as_ref().and_then(|c| c.title.as_deref()).unwrap_or(
                    if run.ice.is_some() {
                        "facedown ICE"
                    } else {
                        "vacant position"
                    },
                );
                let strength = run
                    .ice_strength
                    .map(|n| format!(" · strength {n}"))
                    .unwrap_or_default();
                label(
                    dock,
                    &format!(
                        "{} · {}{} · Pool {} · Overclock {} (spend only)",
                        run.server.label(),
                        if run.position.is_some() {
                            title
                        } else {
                            "server"
                        },
                        strength,
                        view.runner_credits,
                        run.hosted_credits
                    ),
                    18.,
                    Color::WHITE,
                );
                if !run.subroutines.is_empty() {
                    label(
                        dock,
                        &run.subroutines
                            .iter()
                            .map(|s| {
                                format!(
                                    "{} [{}]: {}",
                                    s.number,
                                    if s.broken { "broken" } else { "unbroken" },
                                    s.text
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("   |   "),
                        17.,
                        Color::WHITE,
                    );
                    label(
                        dock,
                        &format!("{} · {}", run.breakers
                            .iter()
                            .map(|(_, name, strength)| format!("{name}: strength {strength}"))
                            .collect::<Vec<_>>()
                            .join(" · "), if run.encounter_paid {
                                "Boosts do not break. Both players passing resolves unbroken subroutines."
                            } else {
                                "No encounter: boosts expire immediately; pass to advance without spending."
                            }),
                        16.,
                        color(Player::Runner),
                    );
                }
            }

            if let Some(reason) = view.terminal_reason {
                label(
                    dock,
                    &format!("{:?} wins · {reason}", view.match_winner.unwrap()),
                    24.,
                    Color::WHITE,
                );
            }
            if ui.inspected.is_none() && !ui.history_open {
                dock.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(10.),
                    row_gap: px(8.),
                    ..default()
                })
                .with_children(|actions| {
                    if let Some(decision) = &view.decision {
                        for action in &decision.choices {
                            actions
                                .spawn(Node {
                                    min_width: px(210.),
                                    max_width: px(440.),
                                    ..default()
                                })
                                .with_children(|slot| {
                                    themed_button(
                                        slot,
                                        &action.description(view),
                                        ButtonCommand::Choose(decision.revision, *action),
                                        mat_color(seat),
                                    );
                                });
                        }
                    } else if view.match_winner.is_none()
                        && !view.run.as_ref().is_some_and(|r| r.server_approach)
                    {
                        label(actions, "Opponent is deciding…", 18., Color::WHITE);
                    }
                    actions
                        .spawn(Node {
                            width: px(160.),
                            ..default()
                        })
                        .with_children(|slot| button(slot, "History", ButtonCommand::History));
                    actions
                        .spawn(Node {
                            width: px(120.),
                            ..default()
                        })
                        .with_children(|slot| button(slot, "Quit", ButtonCommand::Quit));
                });
            }
            if !ui.error.is_empty() {
                label(dock, &ui.error, 18., Color::srgb(1., 0.5, 0.4));
            }
            if let Some(line) = view.history.last() {
                label(dock, line, 18., Color::WHITE);
            }
            label(
                dock,
                "Click a card to inspect · WASD pan · right-drag orbit · wheel zoom · Home reset",
                15.,
                Color::srgb(0.65, 0.73, 0.79),
            );
        });
}

fn hidden() -> CardView {
    CardView {
        instance: None,
        title: None,
        rezzed: false,
        printing: None,
        rules_text: None,
        advancement: 0,
        hosted_credits: 0,
        selected: false,
        face_up: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn piece(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    view: CardView,
    seat: Player,
    owner: Player,
    x: f32,
    z: f32,
    ice: bool,
    face: bool,
) {
    let transform = card_transform(seat, owner, x, z, ice);
    if view.selected {
        commands.spawn((
            BoardPiece,
            Mesh3d(meshes.add(Cuboid::new(2., 0.025, 2.7))),
            MeshMaterial3d(materials.add(color(owner))),
            transform.with_translation(transform.translation - Vec3::Y * 0.05),
        ));
    }
    let counters = view.advancement;
    spawn_card_for(
        commands, meshes, materials, art, view, transform, face, owner,
    );
    if counters > 0 {
        counter(
            commands,
            meshes,
            materials,
            art,
            "Advancement",
            "tokens/advancement.png",
            table_point(seat, x + 0.5, 0.15, z + 0.5),
            owner,
            counters,
        );
        board_label(
            commands,
            "Advancement".into(),
            table_point(seat, x + 0.5, 0.1, z + 0.5) + Vec3::new(-0.8, 0., 1.25),
            Color::WHITE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn counter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    name: &str,
    key: &str,
    position: Vec3,
    owner: Player,
    amount: u32,
) {
    let root = token::spawn_body(
        commands,
        meshes,
        materials,
        token::Body {
            name: format!("{owner:?} {name}"),
            mesh: Cylinder::new(0.28, 0.13).into(),
            color: color(owner),
            transform: Transform::from_translation(position),
        },
    );
    commands.entity(root).insert((BoardPiece, Pickable::IGNORE));
    token::add_face(
        commands,
        meshes,
        materials,
        root,
        token::Face {
            mesh: Circle::new(0.28).into(),
            image: art.image(key),
            alpha: AlphaMode::Opaque,
            transform: Transform::from_xyz(0., 0.067, 0.)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        },
    );
    let label = count_label::spawn(commands, root, amount, Vec3::new(0., 0.04, 0.55));
    commands.entity(label).insert((
        BoardPiece,
        GlobalZIndex(-1),
        TextFont {
            font_size: 18.0.into(),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(3.), px(1.)),
            border_radius: BorderRadius::all(px(2.)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.015, 0.02, 0.02)),
    ));
}

pub(super) fn draw_board(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Artwork,
    view: &Observation,
    seat: Player,
) {
    let max_ice = view.servers.iter().map(|s| s.ice.len()).max().unwrap_or(0) as f32;
    // Reserve the full row spacing on the Corp side. Splitting this growth
    // between players lets the outer ICE spill into the gap and Runner mat.
    // A central root is a separate public zone, never the central pile.
    // Reserve a row for all columns when any central has a root, independent
    // of the concealed card's identity.
    let root_row = if view.servers.iter().any(|s| {
        matches!(s.server, Server::Hq | Server::Rd | Server::Archives) && !s.root.is_empty()
    }) {
        3.2
    } else {
        0.
    };
    let corp_z = -3. - 2.4 * max_ice - root_row;
    let runner_z = 3.;
    let min_x =
        (-12.0_f32).min(-13. - view.corp_score.len().max(view.runner_score.len()) as f32 * 2.1);
    let max_x = (view.servers.len() as f32 * 3.6)
        .max(view.rig.len() as f32 * 2.4 + 18.)
        .max(view.hand.len().max(view.grip_count) as f32 * 2.1)
        .max(view.heap.len() as f32 * 0.3 + 18.);
    let width = max_x - min_x + 4.;
    let center_x = (max_x + min_x) / 2.;
    // Leave room behind each identity for counters and the owner's hand.
    // The Corp mat grows away from the central gap as ICE columns deepen.
    let corp_edge = corp_z - 5.6;
    let runner_edge = runner_z + 5.6;
    let depth = runner_edge - corp_edge + 0.8;
    let center_z = (runner_edge + corp_edge) / 2.;
    commands.spawn((
        BoardPiece,
        Mesh3d(meshes.add(Cuboid::new(width, 0.3, depth))),
        MeshMaterial3d(materials.add(Color::srgb(0.08, 0.13, 0.16))),
        Transform::from_translation(table_point(seat, center_x, -0.2, center_z)),
        Pickable::IGNORE,
    ));
    for (owner, z) in [(Player::Corp, corp_z), (Player::Runner, runner_z)] {
        let (near, far) = if owner == Player::Corp {
            (corp_edge, -0.7)
        } else {
            (0.7, runner_edge)
        };
        commands.spawn((
            BoardPiece,
            Name::new(format!("{owner:?} playmat")),
            Mesh3d(meshes.add(Cuboid::new(width - 0.8, 0.02, far - near))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: mat_color(owner),
                // Preserve the authored fabric colour under the table light.
                unlit: true,
                ..default()
            })),
            Transform::from_translation(table_point(seat, center_x, -0.035, (near + far) / 2.)),
            Pickable::IGNORE,
        ));
        let identity = if owner == Player::Corp {
            view.corp_identity.clone()
        } else {
            view.runner_identity.clone()
        };
        piece(
            commands, meshes, materials, art, identity, seat, owner, -8., z, false, true,
        );
        let credits = if owner == Player::Corp {
            view.corp_credits
        } else {
            view.runner_credits
        };
        let toward_owner = if owner == Player::Corp { -1. } else { 1. };
        board_label(
            commands,
            format!("{owner:?}"),
            table_point(seat, -13., 0.1, z + toward_owner * 1.3),
            color(owner),
        );
        for (name, caption, key, amount, x) in [
            ("Credit pool", "Credits", "tokens/credit.png", credits, -9.4),
            (
                "Click pool",
                "Clicks",
                if owner == Player::Corp {
                    "tokens/click-blue.png"
                } else {
                    "tokens/click-red.png"
                },
                u32::from(view.clicks[owner.index()]),
                -6.6,
            ),
        ] {
            let position = table_point(seat, x, 0.1, z + toward_owner * 2.7);
            counter(
                commands, meshes, materials, art, name, key, position, owner, amount,
            );
            board_label(
                commands,
                caption.into(),
                position + Vec3::new(-0.7, 0.02, -1.4),
                color(owner),
            );
        }
        let scores = if owner == Player::Corp {
            &view.corp_score
        } else {
            &view.runner_score
        };
        for (i, card) in scores.iter().enumerate() {
            piece(
                commands,
                meshes,
                materials,
                art,
                card.clone(),
                seat,
                owner,
                -11. - i as f32 * 2.1,
                z,
                false,
                true,
            );
        }
    }
    for (column, server) in view.servers.iter().enumerate() {
        let x = -3.5 + column as f32 * 3.6;
        board_label(
            commands,
            server.server.label(),
            table_point(seat, x - 0.8, 0.1, corp_z - 1.7),
            color(Player::Corp),
        );
        let central = match server.server {
            Server::Hq => Some((view.hq_count, "HQ")),
            Server::Rd => Some((view.rd_count, "R&D")),
            _ => None,
        };
        if let Some((count, title)) = central {
            if count > 0 {
                piece(
                    commands,
                    meshes,
                    materials,
                    art,
                    hidden(),
                    seat,
                    Player::Corp,
                    x,
                    corp_z,
                    false,
                    false,
                );
            }
            board_label(
                commands,
                format!("{title}: {count}"),
                table_point(seat, x - 0.7, 0.1, corp_z + 1.3),
                Color::WHITE,
            );
        }
        if server.server == Server::Archives {
            for (i, card) in view.archives.iter().enumerate() {
                piece(
                    commands,
                    meshes,
                    materials,
                    art,
                    card.clone(),
                    seat,
                    Player::Corp,
                    x + i as f32 * 0.3,
                    corp_z,
                    false,
                    card.face_up,
                );
            }
        }
        for (i, card) in server.root.iter().enumerate() {
            piece(
                commands,
                meshes,
                materials,
                art,
                card.clone(),
                seat,
                Player::Corp,
                x + i as f32 * 0.3,
                corp_z + root_row,
                false,
                card.rezzed,
            );
        }
        for (i, card) in server.ice.iter().enumerate() {
            piece(
                commands,
                meshes,
                materials,
                art,
                card.clone(),
                seat,
                Player::Corp,
                x,
                corp_z + root_row + 2.6 + i as f32 * 2.4,
                true,
                card.rezzed,
            );
        }
    }
    let mut next_x = -3.5;
    for (title, hardware) in [("Programs", false), ("Hardware", true)] {
        let cards: Vec<_> = view
            .rig
            .iter()
            .filter(|c| (c.printing == Some("30013")) == hardware)
            .collect();
        board_label(
            commands,
            if hardware {
                title.into()
            } else {
                format!("Programs · {} / 4 MU", view.memory_used)
            },
            table_point(seat, next_x - 0.8, 0.1, runner_z - 1.7),
            color(Player::Runner),
        );
        for (i, card) in cards.iter().enumerate() {
            piece(
                commands,
                meshes,
                materials,
                art,
                (*card).clone(),
                seat,
                Player::Runner,
                next_x + i as f32 * 2.2,
                runner_z,
                false,
                true,
            );
        }
        next_x += 2.2 * cards.len().max(2) as f32 + 1.8;
    }
    for (i, card) in view
        .play_area
        .iter()
        .filter(|c| c.printing == Some("30029"))
        .enumerate()
    {
        let x = next_x + i as f32 * 2.4;
        piece(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            seat,
            if card.printing == Some("30029") {
                Player::Runner
            } else {
                Player::Corp
            },
            x,
            runner_z,
            false,
            true,
        );
        if card.printing == Some("30029") {
            counter(
                commands,
                meshes,
                materials,
                art,
                "Overclock credits",
                "tokens/credit.png",
                table_point(seat, x + 0.5, 0.15, runner_z + 0.5),
                Player::Runner,
                u32::from(card.hosted_credits),
            );
            board_label(
                commands,
                "Overclock · hosted".into(),
                table_point(seat, x - 0.8, 0.1, runner_z - 1.7),
                color(Player::Runner),
            );
        }
    }
    if !view.play_area.iter().any(|c| c.printing == Some("30029")) {
        board_label(
            commands,
            "Resources".into(),
            table_point(seat, next_x, 0.1, runner_z - 1.7),
            color(Player::Runner),
        );
    }

    let hand_z = if seat == Player::Corp {
        corp_z - 3.8
    } else {
        runner_z + 3.8
    };
    for (i, card) in view.hand.iter().enumerate() {
        piece(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            seat,
            seat,
            -3.5 + i as f32 * 2.1,
            hand_z,
            false,
            true,
        );
    }
    board_label(
        commands,
        if seat == Player::Corp {
            "HQ · your hand"
        } else {
            "Grip · your hand"
        }
        .into(),
        table_point(
            seat,
            -3.5 + if seat == Player::Corp {
                view.hand.len().saturating_sub(1) as f32 * 2.1
            } else {
                0.
            },
            0.1,
            hand_z,
        ) + Vec3::new(-0.8, 0., 1.5),
        color(seat),
    );
    if seat == Player::Corp {
        for i in 0..view.grip_count {
            piece(
                commands,
                meshes,
                materials,
                art,
                hidden(),
                seat,
                Player::Runner,
                -3.5 + i as f32 * 2.1,
                runner_z + 3.8,
                false,
                false,
            );
        }
    }
    if view.stack_count > 0 {
        piece(
            commands,
            meshes,
            materials,
            art,
            hidden(),
            seat,
            Player::Runner,
            next_x + 3.,
            runner_z,
            false,
            false,
        );
    }
    board_label(
        commands,
        format!("Stack: {}", view.stack_count),
        table_point(seat, next_x + 2.2, 0.1, runner_z + 1.5),
        color(Player::Runner),
    );
    for (i, card) in view.heap.iter().enumerate() {
        piece(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            seat,
            Player::Runner,
            next_x + 5.5 + i as f32 * 0.3,
            runner_z,
            false,
            true,
        );
    }
    board_label(
        commands,
        format!("Heap: {}", view.heap.len()),
        table_point(seat, next_x + 4.8, 0.1, runner_z + 1.5),
        color(Player::Runner),
    );
    if let Some(card) = &view.installing {
        piece(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            seat,
            view.active_player.unwrap(),
            -7.,
            0.,
            false,
            true,
        );
    }
    if let Some(card) = &view.accessed {
        piece(
            commands,
            meshes,
            materials,
            art,
            card.clone(),
            seat,
            Player::Corp,
            0.,
            0.,
            false,
            true,
        );
        board_label(
            commands,
            "Accessed card".into(),
            table_point(seat, -1., 0.1, -1.7),
            Color::WHITE,
        );
    }
}
