//! State machine — hierarchical example
//!
//! Demonstrates compound (hierarchical) states: the root has `Idle` and a
//! compound `Combat` state. `Combat` contains its own sub-region with `Windup`
//! and `Strike`. A blackboard guard controls the Idle → Combat transition,
//! togglable via keyboard or the pane. An on-screen HUD shows the full state
//! path, hierarchy, and recent transitions.

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
    #[pane(monitor)]
    state_path: String,
}

impl Default for HierarchicalPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            enter_combat: false,
            active_state: "Idle".into(),
            state_path: "Idle".into(),
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

const GUARD_ENTER: GuardId = GuardId(1);
const GUARD_EXIT: GuardId = GuardId(2);

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
    .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
    .add_systems(
        Update,
        (
            sync_pane_to_runtime,
            handle_keyboard,
            update_sphere_color,
            update_hud,
            update_pane_monitors,
            update_transition_log,
        ),
    );

    // Register guard callbacks
    {
        let mut callbacks = app.world_mut().resource_mut::<StateMachineCallbacks>();
        callbacks.register_guard(GUARD_ENTER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("enter").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_EXIT, |_, _, definition, _, blackboard, _| {
            !blackboard
                .get_bool(definition.find_blackboard_key("enter").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
    }

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
    builder.blackboard_key(
        "enter",
        BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );

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
        // Combat → Idle: when `enter` becomes false
        .add_transition(TransitionDefinition::replace(combat, idle).with_guard(GUARD_EXIT))
        // Inside Combat: Windup ↔ Strike cycling
        .add_transition(
            TransitionDefinition::replace(windup, strike)
                .with_trigger(TransitionTrigger::after_seconds(2.5)),
        )
        .add_transition(
            TransitionDefinition::replace(strike, windup)
                .with_trigger(TransitionTrigger::after_seconds(2.0)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("HierarchicalAgent"),
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
    // State display (top-left)
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
            Text::new("State: Idle"),
            TextFont::from_font_size(18.0),
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
                 [C] Toggle combat mode\n\n\
                 Hierarchy:\n\
                 Root\n\
                   Idle (leaf)\n\
                   Combat (compound)\n\
                     Windup (leaf)\n\
                     Strike (leaf)\n\n\
                 Press C to enter Combat.\n\
                 Inside Combat, Windup and\n\
                 Strike cycle on timers.\n\
                 Press C again to exit.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Pane → runtime (time scale + blackboard)
// ---------------------------------------------------------------------------

fn sync_pane_to_runtime(
    pane: Res<HierarchicalPane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    library: Res<StateMachineLibrary>,
    mut machines: Query<(&StateMachineInstance, &mut Blackboard), With<Agent>>,
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
// Keyboard
// ---------------------------------------------------------------------------

fn handle_keyboard(keyboard: Res<ButtonInput<KeyCode>>, mut pane: ResMut<HierarchicalPane>) {
    if keyboard.just_pressed(KeyCode::KeyC) {
        pane.enter_combat = !pane.enter_combat;
    }
}

// ---------------------------------------------------------------------------
// Sphere color
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
            .unwrap_or("Idle");

        let elapsed = time.elapsed_secs();

        let (base, emissive, y_offset) = match active_name {
            "Windup" => (
                Color::srgb(0.85, 0.78, 0.24),
                Color::srgb(0.10, 0.08, 0.01),
                0.55 + (elapsed * 4.0).sin().abs() * 0.15,
            ),
            "Strike" => (
                Color::srgb(0.92, 0.38, 0.22),
                Color::srgb(0.20, 0.06, 0.02),
                0.55 + (elapsed * 12.0).sin().abs() * 0.05,
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
    pane: Res<HierarchicalPane>,
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

    let leaf_name = instance
        .active_leaf()
        .and_then(|sid| definition.state(sid))
        .map(|s| s.name.as_str())
        .unwrap_or("None");

    let path: Vec<&str> = instance
        .active_path
        .iter()
        .filter_map(|sid| definition.state(*sid).map(|s| s.name.as_str()))
        .collect();
    let path_str = if path.is_empty() {
        leaf_name.to_string()
    } else {
        let mut p = path.join(" > ");
        p.push_str(&format!(" > {leaf_name}"));
        p
    };

    let combat_status = if pane.enter_combat { "ON" } else { "OFF" };

    **text = format!(
        "Leaf state: {leaf_name}\n\
         Full path: {path_str}\n\
         Combat toggle: {combat_status}\n\
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
    mut pane: ResMut<HierarchicalPane>,
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

    let path: Vec<String> = instance
        .active_path
        .iter()
        .filter_map(|sid| definition.state(*sid).map(|s| s.name.clone()))
        .collect();
    pane.state_path = if path.is_empty() {
        pane.active_state.clone()
    } else {
        path.join(" > ")
    };
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
