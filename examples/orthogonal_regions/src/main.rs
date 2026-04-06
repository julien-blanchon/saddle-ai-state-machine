//! State machine — orthogonal regions example
//!
//! Demonstrates a parallel (orthogonal) state with two independent regions
//! running simultaneously: a **locomotion** region (Grounded / Jump) and an
//! **action** region (IdleAction / Attack). Each region transitions on its own
//! timer. The HUD shows both region states and a transition log.
//! Press Space to force a jump, press A to force an attack.

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
// Markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Agent;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct TransitionLog;

const SIGNAL_JUMP: SignalId = SignalId(1);
const SIGNAL_ATTACK: SignalId = SignalId(2);

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
        .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
        .add_systems(
            Update,
            (
                sync_pane_time_scale,
                handle_keyboard,
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
                .with_trigger(TransitionTrigger::after_seconds(6.0)),
        )
        .add_transition(
            TransitionDefinition::replace(jump, grounded)
                .with_trigger(TransitionTrigger::after_seconds(3.0)),
        )
        // Manual jump signal
        .add_transition(TransitionDefinition::replace(grounded, jump).with_signal(SIGNAL_JUMP))
        // Action region
        .add_state_to_region(idle_action, action)
        .add_state_to_region(attack, action)
        .set_region_initial(action, idle_action)
        .add_transition(
            TransitionDefinition::replace(idle_action, attack)
                .with_trigger(TransitionTrigger::after_seconds(7.0)),
        )
        .add_transition(
            TransitionDefinition::replace(attack, idle_action)
                .with_trigger(TransitionTrigger::after_seconds(3.0)),
        )
        // Manual attack signal
        .add_transition(
            TransitionDefinition::replace(idle_action, attack).with_signal(SIGNAL_ATTACK),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("OrthogonalAgent"),
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
            Text::new("Regions: loading..."),
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
                 [Space] Force jump\n\
                 [A] Force attack\n\n\
                 Two parallel regions:\n\n\
                 Locomotion:\n\
                   Grounded <-> Jump\n\n\
                 Action:\n\
                   IdleAction <-> Attack\n\n\
                 Both run independently!\n\
                 Color blends both states.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

fn sync_pane_time_scale(pane: Res<OrthogonalPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut signals: MessageWriter<StateMachineSignal>,
    agents: Query<Entity, With<Agent>>,
) {
    for entity in &agents {
        if keyboard.just_pressed(KeyCode::Space) {
            signals.write(StateMachineSignal::new(entity, SIGNAL_JUMP));
        }
        if keyboard.just_pressed(KeyCode::KeyA) {
            signals.write(StateMachineSignal::new(entity, SIGNAL_ATTACK));
        }
    }
}

// ---------------------------------------------------------------------------
// Visual — color blend both regions + bounce
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

        let active_names: Vec<&str> = instance
            .active_leaf_states
            .iter()
            .filter_map(|&sid| definition.state(sid).map(|s| s.name.as_str()))
            .collect();

        let is_jumping = active_names.contains(&"Jump");
        let is_attacking = active_names.contains(&"Attack");
        let elapsed = time.elapsed_secs();

        let (base, emissive) = match (is_jumping, is_attacking) {
            (true, true) => (Color::srgb(0.92, 0.28, 0.48), Color::srgb(0.18, 0.04, 0.08)),
            (true, false) => (Color::srgb(0.85, 0.78, 0.24), Color::srgb(0.10, 0.08, 0.01)),
            (false, true) => (Color::srgb(0.92, 0.48, 0.22), Color::srgb(0.18, 0.07, 0.02)),
            (false, false) => (Color::srgb(0.30, 0.60, 0.92), Color::srgb(0.02, 0.05, 0.10)),
        };
        material.base_color = base;
        material.emissive = emissive.into();

        // Bounce when jumping
        let y_offset = if is_jumping {
            0.55 + (elapsed * 3.0).sin().abs() * 0.6
        } else {
            0.55 + (elapsed * 1.5).sin().abs() * 0.03
        };
        transform.translation.y = y_offset;
    }
}

// ---------------------------------------------------------------------------
// HUD update — show both region states
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

    let active_names: Vec<String> = instance
        .active_leaf_states
        .iter()
        .filter_map(|&sid| definition.state(sid).map(|s| s.name.clone()))
        .collect();

    let locomotion = active_names.first().cloned().unwrap_or_else(|| "?".into());
    let action = active_names.get(1).cloned().unwrap_or_else(|| "?".into());

    **text = format!(
        "Locomotion: {locomotion}\n\
         Action: {action}\n\
         Active regions: {}\n\
         Revision: {}",
        instance.active_regions.len(),
        instance.runtime_revision,
    );
}

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<Agent>>,
    mut pane: ResMut<OrthogonalPane>,
) {
    let Ok(instance) = machines.single() else {
        return;
    };
    let Some(definition) = library.definition(instance.definition_id) else {
        return;
    };
    let active_names: Vec<String> = instance
        .active_leaf_states
        .iter()
        .filter_map(|&sid| definition.state(sid).map(|s| s.name.clone()))
        .collect();
    pane.locomotion_state = active_names.first().cloned().unwrap_or_else(|| "?".into());
    pane.action_state = active_names.get(1).cloned().unwrap_or_else(|| "?".into());
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
