//! State machine — debug gizmos example
//!
//! Demonstrates `AiDebugAnnotations`: attaching debug circles, lines, and paths
//! to a state-machine entity so they render as gizmos in the 3D viewport. The
//! sphere toggles between `Idle` and `Alert` on a timer, and gizmo colors shift
//! to reflect the active state.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiDebugAnnotations, AiDebugCircle, AiDebugLine, AiDebugPath, AiStateMachinePlugin,
    StateEntered, StateExited, StateMachineBuilder, StateMachineInstance, StateMachineLibrary,
    TransitionDefinition, TransitionTrigger, TransitionTriggered,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine — Debug Gizmos")]
struct GizmoPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(monitor)]
    active_state: String,
}

impl Default for GizmoPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            active_state: "Idle".into(),
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
                title: "ai_state_machine / debug_gizmos".into(),
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
        .register_pane::<GizmoPane>()
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
        Name::new("Rim Light"),
        PointLight {
            intensity: 650_000.0,
            range: 80.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-5.0, 8.0, -6.0),
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
// State machine — Idle ↔ Alert with debug annotations
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("debug_gizmos");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let alert = builder.atomic_state("Alert");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(alert, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, alert)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        .add_transition(
            TransitionDefinition::replace(alert, idle)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    // Spawn with debug annotations — these appear as gizmos in the viewport
    commands.spawn((
        Name::new("DebugMachine"),
        StateMachineInstance::new(definition_id),
        AiDebugAnnotations {
            circles: vec![AiDebugCircle {
                radius: 3.0,
                color: Color::srgb(0.1, 0.8, 0.9),
                offset: Vec3::ZERO,
            }],
            lines: vec![AiDebugLine {
                start: Vec3::new(0.0, 0.2, 0.0),
                end: Vec3::new(2.0, 1.0, 0.0),
                color: Color::srgb(1.0, 0.6, 0.2),
            }],
            paths: vec![AiDebugPath {
                points: vec![
                    Vec3::new(-2.0, 0.05, -1.0),
                    Vec3::new(-1.0, 0.05, 1.0),
                    Vec3::new(1.0, 0.05, 1.5),
                ],
                color: Color::srgb(0.9, 0.2, 0.6),
            }],
        },
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

fn sync_pane_time_scale(pane: Res<GizmoPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

// ---------------------------------------------------------------------------
// Sphere color
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
            .unwrap_or("Idle");

        let (base, emissive) = match active_name {
            "Alert" => (
                Color::srgb(0.92, 0.48, 0.22),
                Color::srgb(0.18, 0.07, 0.02),
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
    mut pane: ResMut<GizmoPane>,
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
