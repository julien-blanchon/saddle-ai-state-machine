//! State machine — debug gizmos example
//!
//! Demonstrates `AiDebugAnnotations`: attaching debug circles, lines, and paths
//! to a state-machine entity so they render as gizmos in the 3D viewport. The
//! sphere toggles between `Idle` and `Alert` on a timer, and gizmo colors shift
//! to reflect the active state. An on-screen HUD shows the current state and
//! controls.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiDebugAnnotations, AiDebugCircle, AiDebugLine, AiDebugPath, AiStateMachinePlugin, SignalId,
    StateEntered, StateExited, StateMachineBuilder, StateMachineInstance, StateMachineLibrary,
    StateMachineSignal, TransitionDefinition, TransitionTrigger, TransitionTriggered,
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
    #[pane(slider, min = 1.0, max = 8.0, step = 0.1)]
    gizmo_radius: f32,
    #[pane(monitor)]
    active_state: String,
}

impl Default for GizmoPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            gizmo_radius: 3.0,
            active_state: "Idle".into(),
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

const SIGNAL_TOGGLE: SignalId = SignalId(1);

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
        .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
        .add_systems(
            Update,
            (
                sync_pane,
                handle_keyboard,
                update_sphere_color,
                update_gizmo_annotations,
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
        // Auto-cycle
        .add_transition(
            TransitionDefinition::replace(idle, alert)
                .with_trigger(TransitionTrigger::after_seconds(2.0)),
        )
        .add_transition(
            TransitionDefinition::replace(alert, idle)
                .with_trigger(TransitionTrigger::after_seconds(2.0)),
        )
        // Manual toggle
        .add_transition(TransitionDefinition::replace(idle, alert).with_signal(SIGNAL_TOGGLE))
        .add_transition(TransitionDefinition::replace(alert, idle).with_signal(SIGNAL_TOGGLE));

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    // Spawn with debug annotations — these appear as gizmos in the viewport
    commands.spawn((
        Name::new("DebugMachine"),
        Agent,
        StateMachineInstance::new(definition_id),
        AiDebugAnnotations {
            circles: vec![
                AiDebugCircle {
                    radius: 3.0,
                    color: Color::srgb(0.1, 0.8, 0.9),
                    offset: Vec3::ZERO,
                },
                AiDebugCircle {
                    radius: 1.5,
                    color: Color::srgb(0.9, 0.6, 0.2),
                    offset: Vec3::ZERO,
                },
            ],
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
                    Vec3::new(2.5, 0.05, -0.5),
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
            Text::new("State: Idle"),
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
                 [Space] Toggle state\n\n\
                 Debug gizmos:\n\
                 - Circles: detection ranges\n\
                 - Lines: agent direction\n\
                 - Paths: patrol route\n\n\
                 Gizmo colors change\n\
                 based on active state.\n\
                 Use the radius slider\n\
                 to resize the outer ring.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

fn sync_pane(pane: Res<GizmoPane>, mut virtual_time: ResMut<Time<Virtual>>) {
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
            signals.write(StateMachineSignal::new(entity, SIGNAL_TOGGLE));
        }
    }
}

// ---------------------------------------------------------------------------
// Sphere color
// ---------------------------------------------------------------------------

fn update_sphere_color(
    library: Res<StateMachineLibrary>,
    machines: Query<(&StateMachineInstance, &MeshMaterial3d<StandardMaterial>), With<Agent>>,
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
            "Alert" => (Color::srgb(0.92, 0.48, 0.22), Color::srgb(0.18, 0.07, 0.02)),
            _ => (Color::srgb(0.30, 0.60, 0.92), Color::srgb(0.02, 0.05, 0.10)),
        };
        material.base_color = base;
        material.emissive = emissive.into();
    }
}

// ---------------------------------------------------------------------------
// Update gizmo annotations based on state + pane settings
// ---------------------------------------------------------------------------

fn update_gizmo_annotations(
    time: Res<Time>,
    pane: Res<GizmoPane>,
    library: Res<StateMachineLibrary>,
    mut agents: Query<(&StateMachineInstance, &mut AiDebugAnnotations), With<Agent>>,
) {
    for (instance, mut annotations) in &mut agents {
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        let active_name = instance
            .active_leaf()
            .and_then(|sid| definition.state(sid))
            .map(|s| s.name.as_str())
            .unwrap_or("Idle");

        let is_alert = active_name == "Alert";

        // Update outer circle radius from pane
        if let Some(circle) = annotations.circles.first_mut() {
            circle.radius = pane.gizmo_radius;
            circle.color = if is_alert {
                Color::srgb(0.9, 0.3, 0.2)
            } else {
                Color::srgb(0.1, 0.8, 0.9)
            };
        }

        // Animate inner circle
        if let Some(circle) = annotations.circles.get_mut(1) {
            circle.color = if is_alert {
                Color::srgb(0.9, 0.6, 0.2)
            } else {
                Color::srgb(0.2, 0.6, 0.9)
            };
        }

        // Rotate the direction line
        let angle = time.elapsed_secs() * if is_alert { 2.0 } else { 0.5 };
        if let Some(line) = annotations.lines.first_mut() {
            line.end = Vec3::new(angle.cos() * 2.0, 1.0, angle.sin() * 2.0);
            line.color = if is_alert {
                Color::srgb(1.0, 0.3, 0.2)
            } else {
                Color::srgb(1.0, 0.6, 0.2)
            };
        }

        // Change path color
        if let Some(path) = annotations.paths.first_mut() {
            path.color = if is_alert {
                Color::srgb(0.9, 0.2, 0.2)
            } else {
                Color::srgb(0.9, 0.2, 0.6)
            };
        }
    }
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

fn update_hud(
    library: Res<StateMachineLibrary>,
    machines: Query<(&StateMachineInstance, &AiDebugAnnotations), With<Agent>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok((instance, annotations)) = machines.single() else {
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

    **text = format!(
        "State: {state_name}\n\
         Gizmo circles: {}\n\
         Gizmo lines: {}\n\
         Gizmo paths: {}\n\
         Revision: {}",
        annotations.circles.len(),
        annotations.lines.len(),
        annotations.paths.len(),
        instance.runtime_revision,
    );
}

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<&StateMachineInstance, With<Agent>>,
    mut pane: ResMut<GizmoPane>,
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
