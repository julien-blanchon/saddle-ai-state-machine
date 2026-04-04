//! State machine — orthogonal regions example
//!
//! Demonstrates a parallel (orthogonal) state with two independent regions
//! running simultaneously: a **locomotion** region (Grounded / Jump) and an
//! **action** region (IdleAction / Attack). Each region transitions on its own
//! timer, and the sphere color blends both to show the combination.

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
#[pane(title = "State Machine — Orthogonal Regions")]
struct OrthogonalPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(monitor)]
    locomotion_state: String,
    #[pane(monitor)]
    action_state: String,
}

impl Default for OrthogonalPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            locomotion_state: "Grounded".into(),
            action_state: "IdleAction".into(),
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
                title: "ai_state_machine / orthogonal_regions".into(),
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
        .register_pane::<OrthogonalPane>()
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
// State machine — parallel state with two regions
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("orthogonal");
    let root = builder.root_region("root");

    // Parallel wrapper
    let controller = builder.parallel_state("Controller");

    // Region 1: locomotion
    let locomotion = builder.region("locomotion", controller);
    let grounded = builder.atomic_state("Grounded");
    let jump = builder.atomic_state("Jump");

    // Region 2: action
    let action = builder.region("action", controller);
    let idle_action = builder.atomic_state("IdleAction");
    let attack = builder.atomic_state("Attack");

    builder
        .add_state_to_region(controller, root)
        .set_region_initial(root, controller)
        // Locomotion region
        .add_state_to_region(grounded, locomotion)
        .add_state_to_region(jump, locomotion)
        .set_region_initial(locomotion, grounded)
        .add_transition(
            TransitionDefinition::replace(grounded, jump)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        .add_transition(
            TransitionDefinition::replace(jump, grounded)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        // Action region
        .add_state_to_region(idle_action, action)
        .add_state_to_region(attack, action)
        .set_region_initial(action, idle_action)
        .add_transition(
            TransitionDefinition::replace(idle_action, attack)
                .with_trigger(TransitionTrigger::after_seconds(1.5)),
        )
        .add_transition(
            TransitionDefinition::replace(attack, idle_action)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("OrthogonalMachine"),
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

fn sync_pane_time_scale(pane: Res<OrthogonalPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

// ---------------------------------------------------------------------------
// Sphere color — blends locomotion + action state
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

        // Collect all active leaf state names (orthogonal regions produce
        // multiple simultaneous leaf states)
        let active_names: Vec<&str> = instance
            .active_leaf_states
            .iter()
            .filter_map(|&sid| definition.state(sid).map(|s| s.name.as_str()))
            .collect();

        let is_jumping = active_names.contains(&"Jump");
        let is_attacking = active_names.contains(&"Attack");

        let (base, emissive) = match (is_jumping, is_attacking) {
            (true, true) => (
                Color::srgb(0.92, 0.28, 0.48),
                Color::srgb(0.18, 0.04, 0.08),
            ),
            (true, false) => (
                Color::srgb(0.85, 0.78, 0.24),
                Color::srgb(0.10, 0.08, 0.01),
            ),
            (false, true) => (
                Color::srgb(0.92, 0.48, 0.22),
                Color::srgb(0.18, 0.07, 0.02),
            ),
            (false, false) => (
                Color::srgb(0.30, 0.60, 0.92),
                Color::srgb(0.02, 0.05, 0.10),
            ),
        };
        material.base_color = base;
        material.emissive = emissive.into();
    }
}

// ---------------------------------------------------------------------------
// Pane monitors — show both region states
// ---------------------------------------------------------------------------

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance>,
    mut pane: ResMut<OrthogonalPane>,
) {
    for instance in &machines {
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        let active_names: Vec<String> = instance
            .active_leaf_states
            .iter()
            .filter_map(|&sid| definition.state(sid).map(|s| s.name.clone()))
            .collect();

        // Heuristic: first name is locomotion, second is action
        pane.locomotion_state = active_names.first().cloned().unwrap_or_else(|| "?".into());
        pane.action_state = active_names.get(1).cloned().unwrap_or_else(|| "?".into());
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
