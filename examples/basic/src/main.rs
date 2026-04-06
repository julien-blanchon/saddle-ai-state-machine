//! State machine — basic example
//!
//! Demonstrates the simplest possible state machine: three states (`Idle`,
//! `Run`, `Jump`) cycling via timed transitions. A 3D sphere changes color and
//! bounces to reflect the active state. An on-screen HUD shows the current
//! state, recent transitions, and keyboard controls.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, Blackboard, BlackboardValueType, GuardId, SignalId, StateEntered,
    StateExited, StateMachineBuilder, StateMachineCallbacks, StateMachineInstance,
    StateMachineLibrary, StateMachineSignal, TransitionDefinition, TransitionTrigger,
    TransitionTriggered,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine — Basic")]
struct BasicPane {
    #[pane(slider, min = 0.1, max = 3.0, step = 0.05)]
    time_scale: f32,
    #[pane(monitor)]
    active_state: String,
    #[pane(monitor)]
    time_in_state: String,
}

impl Default for BasicPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            active_state: "Idle".into(),
            time_in_state: "0.0s".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Agent;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct TransitionLog;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SIGNAL_RESET: SignalId = SignalId(1);
const GUARD_AUTO: GuardId = GuardId(1);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "ai_state_machine / basic".into(),
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
    .register_pane::<BasicPane>()
    .add_plugins(AiStateMachinePlugin::always_on(Update))
    .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
    .add_systems(
        Update,
        (
            sync_pane_time_scale,
            handle_keyboard_input,
            update_agent_visual,
            update_hud,
            update_pane_monitors,
            update_transition_log,
        ),
    );

    // Guard that always passes — used for the cycling auto-transitions
    app.world_mut()
        .resource_mut::<StateMachineCallbacks>()
        .register_guard(GUARD_AUTO, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("auto_cycle").unwrap())
                .unwrap()
                .unwrap_or(true)
        });

    app.run();
}

// ---------------------------------------------------------------------------
// Scene — camera, lights, floor, pillars
// ---------------------------------------------------------------------------

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.5, 10.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
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
    // Floor
    commands.spawn((
        Name::new("Arena Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(22.0, 22.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.11, 0.13, 0.16),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));
    // State indicator pillars
    for (name, x, color) in [
        ("Idle Pillar", -3.0, Color::srgb(0.20, 0.40, 0.60)),
        ("Run Pillar", 0.0, Color::srgb(0.60, 0.35, 0.15)),
        ("Jump Pillar", 3.0, Color::srgb(0.15, 0.55, 0.30)),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.15, 0.3))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.7,
                ..default()
            })),
            Transform::from_xyz(x, 0.075, -2.0),
        ));
    }
}

// ---------------------------------------------------------------------------
// State machine definition: Idle → Run → Jump → Idle (cycling)
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("basic");
    builder.blackboard_key(
        "auto_cycle",
        BlackboardValueType::Bool,
        false,
        Some(true.into()),
    );

    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    let jump = builder.atomic_state("Jump");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .add_state_to_region(jump, root)
        .set_region_initial(root, idle)
        // Idle → Run after 5.0s
        .add_transition(
            TransitionDefinition::replace(idle, run)
                .with_trigger(TransitionTrigger::after_seconds(5.0))
                .with_guard(GUARD_AUTO),
        )
        // Run → Jump after 4.0s
        .add_transition(
            TransitionDefinition::replace(run, jump)
                .with_trigger(TransitionTrigger::after_seconds(4.0))
                .with_guard(GUARD_AUTO),
        )
        // Jump → Idle after 3.0s (cycle restarts)
        .add_transition(
            TransitionDefinition::replace(jump, idle)
                .with_trigger(TransitionTrigger::after_seconds(3.0))
                .with_guard(GUARD_AUTO),
        )
        // Manual reset via signal from any state
        .add_transition(TransitionDefinition::replace(run, idle).with_signal(SIGNAL_RESET))
        .add_transition(TransitionDefinition::replace(jump, idle).with_signal(SIGNAL_RESET));

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    // Spawn the agent entity with a visible sphere
    commands.spawn((
        Name::new("BasicAgent"),
        Agent,
        StateMachineInstance::new(definition_id),
        Mesh3d(meshes.add(Sphere::new(0.55).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.60, 0.92),
            emissive: Color::BLACK.into(),
            metallic: 0.08,
            perceptual_roughness: 0.38,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.55, 0.0),
    ));
}

// ---------------------------------------------------------------------------
// HUD setup
// ---------------------------------------------------------------------------

fn setup_hud(mut commands: Commands) {
    // State display (top-left)
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
            Text::new("State: Idle"),
            TextFont::from_font_size(20.0),
            TextColor(Color::WHITE),
            HudText,
        ));

    // Transition log (bottom-left)
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

    // Instructions (top-right)
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
                 [Space] Reset to Idle\n\
                 [P] Toggle auto-cycle\n\n\
                 States cycle:\n\
                 Idle -> Run -> Jump -> Idle\n\n\
                 Each transition fires\n\
                 after a timer expires.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Pane → runtime
// ---------------------------------------------------------------------------

fn sync_pane_time_scale(pane: Res<BasicPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

// ---------------------------------------------------------------------------
// Keyboard input
// ---------------------------------------------------------------------------

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    library: Res<StateMachineLibrary>,
    mut signals: MessageWriter<StateMachineSignal>,
    mut agents: Query<(Entity, &StateMachineInstance, &mut Blackboard), With<Agent>>,
) {
    for (entity, instance, mut blackboard) in &mut agents {
        // Space = reset to Idle
        if keyboard.just_pressed(KeyCode::Space) {
            signals.write(StateMachineSignal::new(entity, SIGNAL_RESET));
        }
        // P = toggle auto-cycle
        if keyboard.just_pressed(KeyCode::KeyP) {
            let Some(definition) = library.definition(instance.definition_id) else {
                continue;
            };
            if let Some(key_id) = definition.find_blackboard_key("auto_cycle") {
                let current = blackboard.get_bool(key_id).unwrap().unwrap_or(true);
                let _ = blackboard.set(key_id, !current);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Visual feedback — color + bounce based on active state
// ---------------------------------------------------------------------------

fn update_agent_visual(
    time: Res<Time>,
    library: Res<StateMachineLibrary>,
    mut machines: Query<
        (
            &StateMachineInstance,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        With<Agent>,
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
            .unwrap_or("Idle");

        let elapsed = time.elapsed_secs();

        let (base, emissive, y_offset) = match active_name {
            "Run" => (
                Color::srgb(0.92, 0.58, 0.22),
                Color::srgb(0.18, 0.09, 0.02),
                0.55 + (elapsed * 8.0).sin().abs() * 0.08,
            ),
            "Jump" => (
                Color::srgb(0.24, 0.82, 0.44),
                Color::srgb(0.02, 0.12, 0.04),
                0.55 + (elapsed * 3.0).sin().abs() * 0.6,
            ),
            _ => (
                Color::srgb(0.30, 0.60, 0.92),
                Color::srgb(0.02, 0.05, 0.10),
                0.55 + (elapsed * 1.5).sin().abs() * 0.03,
            ),
        };
        material.base_color = base;
        material.emissive = emissive.into();
        transform.translation.y = y_offset;
    }
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

fn update_hud(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<Agent>>,
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

    let (icon, color_hint) = match state_name {
        "Run" => (">>", "orange"),
        "Jump" => ("^^", "green"),
        _ => ("--", "blue"),
    };

    **text = format!(
        "State: {state_name} {icon}\n\
         Color: {color_hint}\n\
         Time in state: {elapsed:.1}s\n\
         Revision: {}",
        instance.runtime_revision,
    );
}

// ---------------------------------------------------------------------------
// Pane monitors
// ---------------------------------------------------------------------------

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<Agent>>,
    mut pane: ResMut<BasicPane>,
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

    // Keep last 12 entries
    while history.len() > 12 {
        history.remove(0);
    }

    let Ok(mut text) = log_text.single_mut() else {
        return;
    };
    **text = format!("Transition log:\n{}", history.join("\n"));
}
