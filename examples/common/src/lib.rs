use bevy::prelude::*;
use saddle_ai_state_machine::{
    Blackboard, StateEntered, StateExited, StateMachineEvaluationMode, StateMachineInstance,
    StateMachineLibrary, TransitionTriggered,
};
use saddle_pane::prelude::*;

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine Demo")]
pub struct StateMachineExamplePane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    pub time_scale: f32,
    pub event_driven_evaluation: bool,
    pub go: bool,
    pub alt: bool,
    pub enter: bool,
    pub interrupt: bool,
    pub swap: bool,
    #[pane(slider, min = 0.0, max = 1.0, step = 0.01)]
    pub low_score: f32,
    #[pane(slider, min = 0.0, max = 1.0, step = 0.01)]
    pub high_score: f32,
}

impl Default for StateMachineExamplePane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            event_driven_evaluation: false,
            go: false,
            alt: false,
            enter: true,
            interrupt: false,
            swap: false,
            low_score: 0.25,
            high_score: 0.85,
        }
    }
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn base_app(title: &str) -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.to_string(),
            resolution: (1280, 720).into(),
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.45, 0.48, 0.52),
        brightness: 200.0,
        ..default()
    });
    app.add_plugins(pane_plugins());
    app.register_pane::<StateMachineExamplePane>();
    app.add_systems(Startup, setup_scene);
    app.add_systems(
        Update,
        (
            decorate_machine_entities,
            sync_pane_to_runtime,
            update_machine_visuals,
            log_messages,
        ),
    );
    app
}

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
            metallic: 0.02,
            ..default()
        })),
    ));
    for (name, position, color) in [
        (
            "North Pillar",
            Vec3::new(-4.5, 1.0, -3.5),
            Color::srgb(0.29, 0.34, 0.39),
        ),
        (
            "East Pillar",
            Vec3::new(4.5, 1.0, -2.0),
            Color::srgb(0.24, 0.27, 0.33),
        ),
        (
            "Signal Beacon",
            Vec3::new(0.0, 0.5, -5.2),
            Color::srgb(0.68, 0.32, 0.20),
        ),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(meshes.add(Cuboid::new(1.2, 2.0, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.88,
                ..default()
            })),
            Transform::from_translation(position),
        ));
    }
}

fn decorate_machine_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    machines: Query<(Entity, Option<&Mesh3d>), Added<StateMachineInstance>>,
) {
    for (entity, mesh) in &machines {
        if mesh.is_some() {
            continue;
        }

        commands.entity(entity).insert((
            Mesh3d(meshes.add(Sphere::new(0.55).mesh().uv(32, 18))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.30, 0.60, 0.92),
                emissive: Color::BLACK.into(),
                metallic: 0.08,
                perceptual_roughness: 0.38,
                ..default()
            })),
        ));
    }
}

fn sync_pane_to_runtime(
    pane: Res<StateMachineExamplePane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    library: Res<StateMachineLibrary>,
    mut machines: Query<(&mut StateMachineInstance, &mut Blackboard)>,
) {
    if !pane.is_changed() {
        return;
    }

    virtual_time.set_relative_speed(pane.time_scale.max(0.1));

    for (mut instance, mut blackboard) in &mut machines {
        instance.config.evaluation_mode = if pane.event_driven_evaluation {
            StateMachineEvaluationMode::OnSignalOrBlackboardChange
        } else {
            StateMachineEvaluationMode::EveryFrame
        };

        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };

        for (key, value) in [
            ("go", pane.go),
            ("alt", pane.alt),
            ("enter", pane.enter),
            ("interrupt", pane.interrupt),
            ("swap", pane.swap),
        ] {
            if let Some(id) = definition.find_blackboard_key(key) {
                let _ = blackboard.set(id, value);
            }
        }

        for (key, value) in [("low", pane.low_score), ("high", pane.high_score)] {
            if let Some(id) = definition.find_blackboard_key(key) {
                let _ = blackboard.set(id, value);
            }
        }
    }
}

fn update_machine_visuals(
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
            .and_then(|state_id| definition.state(state_id))
            .map(|state| state.name.as_str())
            .unwrap_or("Idle");

        let (base_color, emissive) = match active_name {
            "Run" | "Strike" | "Alert" => (
                Color::srgb(0.92, 0.48, 0.22),
                Color::srgb(0.18, 0.07, 0.02),
            ),
            "Move" | "Windup" | "Pursue" => (
                Color::srgb(0.85, 0.78, 0.24),
                Color::srgb(0.10, 0.08, 0.01),
            ),
            _ => (
                Color::srgb(0.30, 0.60, 0.92),
                Color::srgb(0.02, 0.05, 0.10),
            ),
        };

        material.base_color = base_color;
        material.emissive = emissive.into();
    }
}

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
            "Transition {:?} {:?} -> {:?}",
            event.transition_id, event.source, event.target
        );
    }
}
