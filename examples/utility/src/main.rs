//! State machine — utility-scored transitions example
//!
//! Demonstrates utility-based transition selection: from `Idle`, two competing
//! transitions (to `Gather` or `Flee`) are scored by registered scorer
//! callbacks. The highest-scoring transition wins. Scores oscillate over time
//! so the winner changes periodically. An on-screen HUD shows live scores and
//! the winning state.

use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, Blackboard, BlackboardValueType, ScorerId, StateEntered, StateExited,
    StateMachineBuilder, StateMachineCallbacks, StateMachineInstance, StateMachineLibrary,
    TransitionDefinition, TransitionTrigger, TransitionTriggered, UtilityPolicy,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Pane)]
#[pane(title = "State Machine — Utility")]
struct UtilityPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(slider, min = 0.0, max = 1.0, step = 0.01)]
    gather_bias: f32,
    #[pane(monitor)]
    active_state: String,
    #[pane(monitor)]
    gather_score: String,
    #[pane(monitor)]
    flee_score: String,
}

impl Default for UtilityPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            gather_bias: 0.5,
            active_state: "Idle".into(),
            gather_score: "0.0".into(),
            flee_score: "0.0".into(),
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

const SCORE_GATHER: ScorerId = ScorerId(1);
const SCORE_FLEE: ScorerId = ScorerId(2);

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "ai_state_machine / utility".into(),
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
    .register_pane::<UtilityPane>()
    .add_plugins(AiStateMachinePlugin::always_on(Update))
    .add_systems(Startup, (setup_scene, setup_machine, setup_hud))
    .add_systems(
        Update,
        (
            sync_pane,
            drive_scores,
            update_sphere_color,
            update_hud,
            update_pane_monitors,
            update_transition_log,
        ),
    );

    {
        let mut callbacks = app.world_mut().resource_mut::<StateMachineCallbacks>();
        callbacks.register_scorer(SCORE_GATHER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("gather_score").unwrap())
                .unwrap()
                .unwrap_or(0.0)
        });
        callbacks.register_scorer(SCORE_FLEE, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("flee_score").unwrap())
                .unwrap()
                .unwrap_or(0.0)
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
// State machine
// ---------------------------------------------------------------------------

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("utility");
    builder.blackboard_key(
        "gather_score",
        BlackboardValueType::F32,
        false,
        Some(0.4_f32.into()),
    );
    builder.blackboard_key(
        "flee_score",
        BlackboardValueType::F32,
        false,
        Some(0.8_f32.into()),
    );

    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let gather = builder.atomic_state("Gather");
    let flee = builder.atomic_state("Flee");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(gather, root)
        .add_state_to_region(flee, root)
        .set_region_initial(root, idle)
        // Idle → Gather (utility scored)
        .add_transition(
            TransitionDefinition::replace(idle, gather)
                .with_scorer(SCORE_GATHER, UtilityPolicy::BestScore),
        )
        // Idle → Flee (utility scored — competes with Gather)
        .add_transition(
            TransitionDefinition::replace(idle, flee)
                .with_scorer(SCORE_FLEE, UtilityPolicy::BestScore),
        )
        // Gather → Idle after 1.5s (re-evaluate)
        .add_transition(
            TransitionDefinition::replace(gather, idle)
                .with_trigger(TransitionTrigger::after_seconds(1.5)),
        )
        // Flee → Idle after 1.5s (re-evaluate)
        .add_transition(
            TransitionDefinition::replace(flee, idle)
                .with_trigger(TransitionTrigger::after_seconds(1.5)),
        );

    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("UtilityAgent"),
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
                "Utility Scoring:\n\n\
                 Two transitions compete:\n\
                 Idle -> Gather (green)\n\
                 Idle -> Flee (red)\n\n\
                 The highest-scoring\n\
                 transition wins.\n\n\
                 Scores oscillate over\n\
                 time. Use the 'gather\n\
                 bias' slider to shift\n\
                 the balance.",
            ),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.6, 0.65, 0.7)),
        ));
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

fn sync_pane(pane: Res<UtilityPane>, mut virtual_time: ResMut<Time<Virtual>>) {
    if pane.is_changed() {
        virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    }
}

fn drive_scores(
    time: Res<Time>,
    pane: Res<UtilityPane>,
    library: Res<StateMachineLibrary>,
    mut agents: Query<(&StateMachineInstance, &mut Blackboard), With<Agent>>,
) {
    let t = time.elapsed_secs();
    for (instance, mut blackboard) in &mut agents {
        let Some(definition) = library.definition(instance.definition_id) else {
            continue;
        };
        let gather_key = definition.find_blackboard_key("gather_score").unwrap();
        let flee_key = definition.find_blackboard_key("flee_score").unwrap();

        // Oscillating scores influenced by the pane bias
        let raw_gather = 0.5 + 0.5 * (t * 0.8).sin();
        let raw_flee = 0.5 + 0.5 * (t * 0.8 + std::f32::consts::PI).sin();
        let gather_score = (raw_gather * pane.gather_bias * 2.0).clamp(0.0, 1.0);
        let flee_score = (raw_flee * (1.0 - pane.gather_bias) * 2.0).clamp(0.0, 1.0);

        let _ = blackboard.set(gather_key, gather_score);
        let _ = blackboard.set(flee_key, flee_score);
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
            .unwrap_or("Idle");

        let elapsed = time.elapsed_secs();

        let (base, emissive) = match active_name {
            "Gather" => (Color::srgb(0.24, 0.82, 0.44), Color::srgb(0.02, 0.12, 0.04)),
            "Flee" => (Color::srgb(0.92, 0.32, 0.28), Color::srgb(0.18, 0.04, 0.03)),
            _ => (Color::srgb(0.30, 0.60, 0.92), Color::srgb(0.02, 0.05, 0.10)),
        };
        material.base_color = base;
        material.emissive = emissive.into();
        transform.translation.y = 0.55 + (elapsed * 2.0).sin().abs() * 0.03;
    }
}

// ---------------------------------------------------------------------------
// HUD update
// ---------------------------------------------------------------------------

fn update_hud(
    library: Res<StateMachineLibrary>,
    machines: Query<(&StateMachineInstance, &Blackboard), With<Agent>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Ok((instance, blackboard)) = machines.single() else {
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

    let gather_score = definition
        .find_blackboard_key("gather_score")
        .and_then(|k| blackboard.get_f32(k).ok()?)
        .unwrap_or(0.0);
    let flee_score = definition
        .find_blackboard_key("flee_score")
        .and_then(|k| blackboard.get_f32(k).ok()?)
        .unwrap_or(0.0);

    let gather_bar = score_bar(gather_score);
    let flee_bar = score_bar(flee_score);
    let winner = if gather_score > flee_score {
        "Gather"
    } else {
        "Flee"
    };

    **text = format!(
        "State: {state_name}\n\
         \n\
         Gather: {gather_score:.2} {gather_bar}\n\
         Flee:   {flee_score:.2} {flee_bar}\n\
         Winner: {winner}\n\
         Revision: {}",
        instance.runtime_revision,
    );
}

fn score_bar(score: f32) -> String {
    let filled = (score * 20.0).round() as usize;
    let empty = 20_usize.saturating_sub(filled);
    format!("[{}{}]", "|".repeat(filled), ".".repeat(empty))
}

fn update_pane_monitors(
    library: Res<StateMachineLibrary>,
    machines: Query<(&StateMachineInstance, &Blackboard), With<Agent>>,
    mut pane: ResMut<UtilityPane>,
) {
    let Ok((instance, blackboard)) = machines.single() else {
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
    pane.gather_score = format!(
        "{:.2}",
        definition
            .find_blackboard_key("gather_score")
            .and_then(|k| blackboard.get_f32(k).ok()?)
            .unwrap_or(0.0)
    );
    pane.flee_score = format!(
        "{:.2}",
        definition
            .find_blackboard_key("flee_score")
            .and_then(|k| blackboard.get_f32(k).ok()?)
            .unwrap_or(0.0)
    );
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
