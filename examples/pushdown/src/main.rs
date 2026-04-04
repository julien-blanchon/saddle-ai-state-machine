//! State machine — pushdown example
//!
//! Demonstrates push/pop transitions: a `Patrol` state is interrupted by
//! `Stunned` (pushed onto the stack), and after the stun duration the previous
//! state is restored via pop. The sphere flashes red while stunned.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, StateEntered, StateExited, StateMachineBuilder, StateMachineInstance,
    StateMachineLibrary, TransitionDefinition, TransitionTrigger, TransitionTriggered,
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
            stack_depth: "1".into(),
        }
    }
}

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
        .add_systems(Startup, (setup_scene, setup_machine))
        .add_systems(
            Update,
            (
                sync_pane_time_scale,
                update_sphere_color,
                update_pane_monitors,
                log_messages,
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
        Transform::from_xyz(0.0, 7.5, 14.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
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
    let stunned = builder.atomic_state("Stunned");
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, patrol)
        // After 1.2s of patrol, push Stunned onto the stack
        .add_transition(
            TransitionDefinition::push(patrol, stunned)
                .with_trigger(TransitionTrigger::after_seconds(1.2)),
        )
        // After 0.8s of stun, pop back to Patrol
        .add_transition(
            TransitionDefinition::pop(stunned)
                .with_trigger(TransitionTrigger::after_seconds(0.8)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("PushdownMachine"),
        StateMachineInstance::new(definition_id),
        Mesh3d(meshes.add(Sphere::new(0.55).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.30, 0.60, 0.92),
            emissive: Color::BLACK.into(),
            metallic: 0.08,
            perceptual_roughness: 0.38,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}

// ---------------------------------------------------------------------------
// Pane → runtime
// ---------------------------------------------------------------------------

fn sync_pane_time_scale(pane: Res<PushdownPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

// ---------------------------------------------------------------------------
// Sphere color reflects active state
// ---------------------------------------------------------------------------

fn update_sphere_color(
    library: Res<StateMachineLibrary>,
    machines: Query<(&StateMachineInstance, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (instance, material_handle) in &machines {
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

        let (base, emissive) = match active_name {
            "Stunned" => (
                Color::srgb(0.92, 0.28, 0.28),
                Color::srgb(0.20, 0.04, 0.04),
            ),
            _ => (
                Color::srgb(0.30, 0.60, 0.92),
                Color::srgb(0.02, 0.05, 0.10),
            ),
        };
        material.base_color = base;
        material.emissive = emissive.into();
    }
}

// ---------------------------------------------------------------------------
// Pane monitors
// ---------------------------------------------------------------------------

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance>,
    mut pane: ResMut<PushdownPane>,
) {
    for instance in &machines {
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        pane.active_state = instance
            .active_leaf()
            .and_then(|sid| definition.state(sid))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "None".into());
        pane.stack_depth = format!("{}", instance.stack.len());
    }
}

// ---------------------------------------------------------------------------
// Log lifecycle messages
// ---------------------------------------------------------------------------

fn log_messages(
    mut entered: MessageReader<StateEntered>,
    mut exited: MessageReader<StateExited>,
    mut triggered: MessageReader<TransitionTriggered>,
) {
    for event in exited.read() {
        info!("Exited state {:?} on {:?}", event.state_id, event.entity);
    }
    for event in entered.read() {
        info!("Entered state {:?} on {:?}", event.state_id, event.entity);
    }
    for event in triggered.read() {
        info!(
            "Transition {:?}: {:?} -> {:?}",
            event.transition_id, event.source, event.target,
        );
    }
}
