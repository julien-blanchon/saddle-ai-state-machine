//! State machine — delayed transitions example
//!
//! Models an interactive door cycle: Closed → Opening → Open → Closing → Closed.
//! Each transition fires after a timer expires. Press Space to toggle the door.
//! The sphere scales and colors to represent the four door phases. An on-screen
//! HUD shows the current phase, timer progress, and controls.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, Blackboard, BlackboardValueType, GuardId, StateEntered, StateExited,
    StateMachineBuilder, StateMachineCallbacks, StateMachineInstance, StateMachineLibrary,
    TransitionDefinition, TransitionTrigger, TransitionTriggered,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine — Delayed Transitions")]
struct DelayedPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(monitor)]
    active_state: String,
    #[pane(monitor)]
    time_in_state: String,
}

impl Default for DelayedPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            active_state: "Closed".into(),
            time_in_state: "0.0s".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct DoorAgent;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct TransitionLog;

const GUARD_WANTS_OPEN: GuardId = GuardId(1);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "ai_state_machine / delayed_transitions".into(),
            resolution: (1280, 720).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.45, 0.48, 0.52),
        brightness: 200.0,
        ..default()
    })
    .add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<DelayedPane>()
    .add_plugins(AiStateMachinePlugin::always_on(Update))
    .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
    .add_systems(
        Update,
        (
            sync_pane_time_scale,
            handle_keyboard,
            update_door_visual,
            update_hud,
            update_pane_monitors,
            update_transition_log,
        ),
    );

    app.world_mut()
        .resource_mut::<StateMachineCallbacks>()
        .register_guard(GUARD_WANTS_OPEN, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("wants_open").unwrap())
                .unwrap()
                .unwrap_or(false)
        });

    app.run();
}

// ---------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Key Light"),
        DirectionalLight {
            illuminance: 15_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.95, -0.55, 0.0)),
    ));
    commands.spawn((
        Name::new("Fill Light"),
        PointLight {
            intensity: 400_000.0,
            range: 60.0,
            ..default()
        },
        Transform::from_xyz(-4.0, 6.0, -3.0),
    ));
    commands.spawn((
        Name::new("Arena Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(22.0, 22.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.11, 0.13, 0.16),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));
    // Door frame
    for (name, pos) in [
        ("Left Frame", Vec3::new(-1.2, 1.5, 0.0)),
        ("Right Frame", Vec3::new(1.2, 1.5, 0.0)),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(meshes.add(Cuboid::new(0.2, 3.0, 0.3))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.25, 0.2),
                perceptual_roughness: 0.85,
                ..default()
            })),
            Transform::from_translation(pos),
        ));
    }
    // Top beam
    commands.spawn((
        Name::new("Top Frame"),
        Mesh3d(meshes.add(Cuboid::new(2.6, 0.2, 0.3))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.25, 0.2),
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::from_xyz(0.0, 3.1, 0.0),
    ));
}

// ---------------------------------------------------------------------------
// State machine — four-state door cycle
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("door");
    builder.blackboard_key(
        "wants_open",
        BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );

    let root = builder.root_region("root");
    let closed = builder.atomic_state("Closed");
    let opening = builder.atomic_state("Opening");
    let open = builder.atomic_state("Open");
    let closing = builder.atomic_state("Closing");

    builder
        .add_state_to_region(closed, root)
        .add_state_to_region(opening, root)
        .add_state_to_region(open, root)
        .add_state_to_region(closing, root)
        .set_region_initial(root, closed)
        // Closed → Opening when user wants open
        .add_transition(TransitionDefinition::replace(closed, opening).with_guard(GUARD_WANTS_OPEN))
        // Opening → Open after animation time
        .add_transition(
            TransitionDefinition::replace(opening, open)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        // Open → Closing after hold time
        .add_transition(
            TransitionDefinition::replace(open, closing)
                .with_trigger(TransitionTrigger::after_seconds(2.0)),
        )
        // Closing → Closed after animation time
        .add_transition(
            TransitionDefinition::replace(closing, closed)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    // Door panel (the moving part)
    commands.spawn((
        Name::new("DoorPanel"),
        DoorAgent,
        StateMachineInstance::new(definition_id),
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.8, 0.12))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.35, 0.25),
            metallic: 0.05,
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.4, 0.0),
    ));
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16.0),
                left: px(16.0),
                width: px(340.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.88)),
        ))
        .with_child((
            Text::new("Door: Closed"),
            TextFont::from_font_size(20.0),
            TextColor(Color::WHITE),
            HudText,
        ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(16.0),
                left: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.82)),
        ))
        .with_child((
            Text::new("Transition log:\n  (waiting...)"),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.7, 0.75, 0.8)),
            TransitionLog,
        ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16.0),
                right: px(16.0),
                width: px(260.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.82)),
        ))
        .with_child((
            Text::new(
                "Controls:\n\
                 [Space] Open the door\n\n\
                 Door cycle:\n\
                 Closed -> Opening (1s)\n\
                 Opening -> Open\n\
                 Open -> Closing (2s)\n\
                 Closing -> Closed (1s)\n\n\
                 Each transition fires\n\
                 after a timer expires.\n\
                 The door panel slides up\n\
                 to show the state visually.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

fn sync_pane_time_scale(pane: Res<DelayedPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    library: Res<StateMachineLibrary>,
    mut agents: Query<(&StateMachineInstance, &mut Blackboard), With<DoorAgent>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        for (instance, mut blackboard) in &mut agents {
            let Some(definition) = library.definition(instance.definition_id) else {
                continue;
            };
            if let Some(key_id) = definition.find_blackboard_key("wants_open") {
                let _ = blackboard.set(key_id, true);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Door visual — slide panel up based on state
// ---------------------------------------------------------------------------

fn update_door_visual(
    library: Res<StateMachineLibrary>,
    mut machines: Query<
        (
            &StateMachineInstance,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        With<DoorAgent>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (instance, material_handle, mut transform) in &mut machines {
        let Some(material) = materials.get_mut(material_handle.id()) else {
            continue;
        };
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        let active_name = instance
            .active_leaf()
            .and_then(|sid| definition.state(sid))
            .map(|s| s.name.as_str())
            .unwrap_or("Closed");

        let elapsed = instance
            .active_leaf()
            .and_then(|sid| instance.state_elapsed_seconds.get(sid.0 as usize).copied())
            .unwrap_or(0.0);

        let (y_pos, color) = match active_name {
            "Opening" => {
                let t = (elapsed / 1.0).clamp(0.0, 1.0);
                (1.4 + t * 2.8, Color::srgb(0.85, 0.78, 0.24))
            }
            "Open" => (1.4 + 2.8, Color::srgb(0.24, 0.84, 0.44)),
            "Closing" => {
                let t = (elapsed / 1.0).clamp(0.0, 1.0);
                (1.4 + (1.0 - t) * 2.8, Color::srgb(0.92, 0.48, 0.22))
            }
            _ => (1.4, Color::srgb(0.45, 0.35, 0.25)),
        };

        transform.translation.y = y_pos;
        material.base_color = color;
    }
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

fn update_hud(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<DoorAgent>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok(instance) = machines.single() else {
        return;
    };
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let Some(definition) = library.definition(instance.definition_id) else {
        return;
    };

    let state_name = instance
        .active_leaf()
        .and_then(|sid| definition.state(sid))
        .map(|s| s.name.as_str())
        .unwrap_or("None");

    let elapsed = instance
        .active_leaf()
        .and_then(|sid| instance.state_elapsed_seconds.get(sid.0 as usize).copied())
        .unwrap_or(0.0);

    **text = format!(
        "Door: {state_name}\n\
         Time in phase: {elapsed:.1}s\n\
         Revision: {}",
        instance.runtime_revision,
    );
}

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<DoorAgent>>,
    mut pane: ResMut<DelayedPane>,
) {
    let Ok(instance) = machines.single() else {
        return;
    };
    let Some(definition) = library.definition(instance.definition_id) else {
        return;
    };
    pane.active_state = instance
        .active_leaf()
        .and_then(|sid| definition.state(sid))
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "None".into());
    let elapsed = instance
        .active_leaf()
        .and_then(|sid| instance.state_elapsed_seconds.get(sid.0 as usize).copied())
        .unwrap_or(0.0);
    pane.time_in_state = format!("{elapsed:.1}s");
}

// ---------------------------------------------------------------------------
// Transition log
// ---------------------------------------------------------------------------

fn update_transition_log(
    library: Res<StateMachineLibrary>,
    mut entered: MessageReader<StateEntered>,
    mut exited: MessageReader<StateExited>,
    mut triggered: MessageReader<TransitionTriggered>,
    mut history: Local<Vec<String>>,
    mut log_text: Query<&mut Text, With<TransitionLog>>,
) {
    let mut changed = false;
    for event in exited.read() {
        let name = library
            .definition(event.definition_id)
            .and_then(|d| d.state(event.state_id))
            .map(|s| s.name.as_str())
            .unwrap_or("?");
        history.push(format!("  EXIT  {name}"));
        changed = true;
    }
    for event in entered.read() {
        let name = library
            .definition(event.definition_id)
            .and_then(|d| d.state(event.state_id))
            .map(|s| s.name.as_str())
            .unwrap_or("?");
        history.push(format!("  ENTER {name}"));
        changed = true;
    }
    for event in triggered.read() {
        let source = event
            .source
            .map(|sid| {
                library
                    .definition(event.definition_id)
                    .and_then(|d| d.state(sid))
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| format!("{:?}", sid))
            })
            .unwrap_or_else(|| "Any".into());
        let target = event
            .target
            .map(|sid| {
                library
                    .definition(event.definition_id)
                    .and_then(|d| d.state(sid))
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| format!("{:?}", sid))
            })
            .unwrap_or_else(|| "Pop".into());
        history.push(format!("  {source} -> {target}"));
        changed = true;
    }
    if !changed {
        return;
    }
    while history.len() > 12 {
        history.remove(0);
    }
    let Ok(mut text) = log_text.single_mut() else {
        return;
    };
    **text = format!("Transition log:\n{}", history.join("\n"));
}
