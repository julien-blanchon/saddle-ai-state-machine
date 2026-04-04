//! State machine — hierarchical example
//!
//! Demonstrates compound (hierarchical) states: the root has `Idle` and a
//! compound `Combat` state. `Combat` contains its own sub-region with `Windup`
//! and `Strike`. A blackboard guard controls the Idle → Combat transition,
//! togglable from the pane.

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
#[pane(title = "State Machine — Hierarchical")]
struct HierarchicalPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    /// Toggle to allow the Idle → Combat transition
    pub enter_combat: bool,
    #[pane(monitor)]
    active_state: String,
}

impl Default for HierarchicalPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            enter_combat: true,
            active_state: "Idle".into(),
        }
    }
}

const GUARD_ENTER: GuardId = GuardId(1);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "ai_state_machine / hierarchical".into(),
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
    .register_pane::<HierarchicalPane>()
    .add_plugins(AiStateMachinePlugin::always_on(Update))
    .add_systems(Startup, (setup_scene, setup_machine))
    .add_systems(
        Update,
        (
            sync_pane_to_runtime,
            update_sphere_color,
            update_pane_monitors,
            log_messages,
        ),
    );

    // Register guard callback — reads the `enter` blackboard key
    app.world_mut()
        .resource_mut::<StateMachineCallbacks>()
        .register_guard(GUARD_ENTER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("enter").unwrap())
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
// State machine — hierarchical with compound Combat state
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("hierarchical");

    // Blackboard key: pane-driven toggle to enter combat
    builder.blackboard_key("enter", BlackboardValueType::Bool, false, Some(true.into()));

    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let combat = builder.compound_state("Combat");
    let combat_region = builder.region("combat_region", combat);
    let windup = builder.atomic_state("Windup");
    let strike = builder.atomic_state("Strike");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(combat, root)
        .set_region_initial(root, idle)
        .add_state_to_region(windup, combat_region)
        .add_state_to_region(strike, combat_region)
        .set_region_initial(combat_region, windup)
        // Idle → Combat: gated by the `enter` guard
        .add_transition(TransitionDefinition::replace(idle, combat).with_guard(GUARD_ENTER))
        // Inside Combat: Windup → Strike after 0.8s
        .add_transition(
            TransitionDefinition::replace(windup, strike)
                .with_trigger(TransitionTrigger::after_seconds(0.8)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("HierarchicalMachine"),
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
// Pane → runtime (time scale + blackboard)
// ---------------------------------------------------------------------------

fn sync_pane_to_runtime(
    pane: Res<HierarchicalPane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    library: Res<StateMachineLibrary>,
    mut machines: Query<(&StateMachineInstance, &mut Blackboard)>,
) {
    if !pane.is_changed() {
        return;
    }
    virtual_time.set_relative_speed(pane.time_scale.max(0.1));

    for (instance, mut blackboard) in &mut machines {
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        if let Some(id) = definition.find_blackboard_key("enter") {
            let _ = blackboard.set(id, pane.enter_combat);
        }
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
            "Windup" => (
                Color::srgb(0.85, 0.78, 0.24),
                Color::srgb(0.10, 0.08, 0.01),
            ),
            "Strike" => (
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
    mut pane: ResMut<HierarchicalPane>,
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
