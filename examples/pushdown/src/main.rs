//! State machine — pushdown example
//!
//! Demonstrates push/pop transitions: a `Patrol` state is interrupted by
//! `Stunned` (pushed onto the stack), and after the stun duration the previous
//! state is restored via pop. Press Space to manually stun the agent. The HUD
//! shows stack depth, state history, and recent transitions.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, SignalId, StateEntered, StateExited, StateMachineBuilder,
    StateMachineInstance, StateMachineLibrary, StateMachineSignal, TransitionDefinition,
    TransitionTrigger, TransitionTriggered,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine — Pushdown")]
struct PushdownPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(monitor)]
    active_state: String,
    #[pane(monitor)]
    stack_depth: String,
}

impl Default for PushdownPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            active_state: "Patrol".into(),
            stack_depth: "0".into(),
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

const SIGNAL_STUN: SignalId = SignalId(1);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ai_state_machine / pushdown".into(),
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
        .register_pane::<PushdownPane>()
        .add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
        .add_systems(
            Update,
            (
                sync_pane_time_scale,
                handle_keyboard,
                auto_stun_cycle,
                update_sphere_color,
                update_hud,
                update_pane_monitors,
                update_transition_log,
            ),
        )
        .run();
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
    commands.spawn((
        Name::new("Arena Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(22.0, 22.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.11, 0.13, 0.16),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));
}

// ---------------------------------------------------------------------------
// State machine — push/pop transitions
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("pushdown");
    let root = builder.root_region("root");
    let patrol = builder.atomic_state("Patrol");
    let chase = builder.atomic_state("Chase");
    let stunned = builder.atomic_state("Stunned");

    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(chase, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, patrol)
        // Patrol → Chase after 5s
        .add_transition(
            TransitionDefinition::replace(patrol, chase)
                .with_trigger(TransitionTrigger::after_seconds(5.0)),
        )
        // Chase → Patrol after 5s
        .add_transition(
            TransitionDefinition::replace(chase, patrol)
                .with_trigger(TransitionTrigger::after_seconds(5.0)),
        )
        // Any state: push Stunned via signal (interrupt)
        .add_transition(
            TransitionDefinition::push(
                saddle_ai_state_machine::TransitionSource::AnyState,
                stunned,
            )
            .with_signal(SIGNAL_STUN),
        )
        // Pop back after 3.0s of stun
        .add_transition(
            TransitionDefinition::pop(stunned).with_trigger(TransitionTrigger::after_seconds(3.0)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("PushdownAgent"),
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
// HUD
// ---------------------------------------------------------------------------

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16.0),
                left: px(16.0),
                width: px(380.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.88)),
        ))
        .with_child((
            Text::new("State: Patrol"),
            TextFont::from_font_size(18.0),
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
                width: px(280.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.82)),
        ))
        .with_child((
            Text::new(
                "Controls:\n\
                 [Space] Stun the agent\n\n\
                 Pushdown automaton:\n\
                 Patrol <-> Chase (cycling)\n\n\
                 When stunned, the current\n\
                 state is pushed to a stack.\n\
                 After stun ends, it pops\n\
                 back to the previous state.\n\n\
                 Auto-stun every ~12s.\n\
                 Press Space for manual stun.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

fn sync_pane_time_scale(pane: Res<PushdownPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut signals: MessageWriter<StateMachineSignal>,
    agents: Query<Entity, With<Agent>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        for entity in &agents {
            signals.write(StateMachineSignal::new(entity, SIGNAL_STUN));
        }
    }
}

fn auto_stun_cycle(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut signals: MessageWriter<StateMachineSignal>,
    agents: Query<(Entity, &StateMachineInstance), With<Agent>>,
) {
    *timer += time.delta_secs();
    if *timer >= 12.0 {
        *timer = 0.0;
        for (entity, instance) in &agents {
            // Only auto-stun if not already stunned
            if instance.stack.is_empty() {
                signals.write(StateMachineSignal::new(entity, SIGNAL_STUN));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Visual feedback
// ---------------------------------------------------------------------------

fn update_sphere_color(
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
            .unwrap_or("Patrol");

        let elapsed = time.elapsed_secs();

        let (base, emissive, y_offset, scale) = match active_name {
            "Chase" => (
                Color::srgb(0.92, 0.68, 0.22),
                Color::srgb(0.14, 0.08, 0.02),
                0.55 + (elapsed * 6.0).sin().abs() * 0.1,
                1.0,
            ),
            "Stunned" => (
                Color::srgb(0.92, 0.28, 0.28),
                Color::srgb(0.20, 0.04, 0.04),
                0.55,
                0.8 + (elapsed * 15.0).sin().abs() * 0.15,
            ),
            _ => (
                Color::srgb(0.30, 0.60, 0.92),
                Color::srgb(0.02, 0.05, 0.10),
                0.55 + (elapsed * 1.5).sin().abs() * 0.03,
                1.0,
            ),
        };
        material.base_color = base;
        material.emissive = emissive.into();
        transform.translation.y = y_offset;
        transform.scale = Vec3::splat(scale);
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

    let stack_depth = instance.stack.len();

    let stack_desc = if stack_depth == 0 {
        "  (empty)".to_string()
    } else {
        format!("  {} frame(s) saved", stack_depth)
    };

    **text = format!(
        "State: {state_name}\n\
         Stack depth: {stack_depth}\n\
         Stack: \n{stack_desc}\n\
         Revision: {}",
        instance.runtime_revision,
    );
}

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<Agent>>,
    mut pane: ResMut<PushdownPane>,
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
    pane.stack_depth = format!("{}", instance.stack.len());
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
        let op = match event.operation {
            saddle_ai_state_machine::TransitionOperation::Push => "PUSH",
            saddle_ai_state_machine::TransitionOperation::Pop => "POP",
            saddle_ai_state_machine::TransitionOperation::Replace => "->",
        };
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
            .unwrap_or_else(|| "(resume)".into());
        history.push(format!("  {source} {op} {target}"));
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
