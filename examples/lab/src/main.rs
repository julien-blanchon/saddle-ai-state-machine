use bevy::color::palettes::css;
use bevy::prelude::*;
use saddle_ai_state_machine::*;
use saddle_pane::prelude::*;

const GUARD_VISIBLE: GuardId = GuardId(1);
const GUARD_HIDDEN: GuardId = GuardId(2);
const GUARD_IN_RANGE: GuardId = GuardId(3);
const GUARD_OUT_OF_RANGE: GuardId = GuardId(4);
const SIGNAL_STUN: SignalId = SignalId(1);

#[derive(Component)]
struct LabAgent;

#[derive(Component)]
struct LabTarget;

#[derive(Resource, Default)]
struct LabClock {
    elapsed: f32,
    stun_cycle: Option<u32>,
}

#[derive(Resource, Clone, Pane)]
#[pane(title = "FSM Lab")]
struct StateMachineLabPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    event_driven: bool,
    #[pane(slider, min = 1.0, max = 8.0, step = 0.1)]
    visibility_radius: f32,
    #[pane(slider, min = 0.5, max = 4.0, step = 0.1)]
    attack_radius: f32,
    #[pane(slider, min = 1.0, max = 12.0, step = 0.25)]
    stun_interval: f32,
}

impl Default for StateMachineLabPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            event_driven: false,
            visibility_radius: 4.6,
            attack_radius: 1.8,
            stun_interval: 5.5,
        }
    }
}

#[derive(Resource, Clone, Copy)]
struct LabKeys {
    target_visible: BlackboardKeyId,
    in_attack_range: BlackboardKeyId,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
        AiStateMachinePlugin::always_on(Update),
    ));
    app.register_pane::<StateMachineLabPane>();
    app.init_resource::<LabClock>();
    app.init_resource::<StateMachineLabPane>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            sync_pane_to_runtime,
            drive_machine,
            animate_target,
            sync_annotations,
        ),
    );

    {
        let mut callbacks = app.world_mut().resource_mut::<StateMachineCallbacks>();
        callbacks.register_guard(GUARD_VISIBLE, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("target_visible").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_HIDDEN, |_, _, definition, _, blackboard, _| {
            !blackboard
                .get_bool(definition.find_blackboard_key("target_visible").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_IN_RANGE, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("in_attack_range").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_OUT_OF_RANGE, |_, _, definition, _, blackboard, _| {
            !blackboard
                .get_bool(definition.find_blackboard_key("in_attack_range").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
    }

    app.run();
}

fn sync_pane_to_runtime(
    pane: Res<StateMachineLabPane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut machines: Query<&mut StateMachineInstance, With<LabAgent>>,
) {
    if !pane.is_changed() {
        return;
    }

    virtual_time.set_relative_speed(pane.time_scale.max(0.1));

    for mut instance in &mut machines {
        instance.config.evaluation_mode = if pane.event_driven {
            StateMachineEvaluationMode::OnSignalOrBlackboardChange
        } else {
            StateMachineEvaluationMode::EveryFrame
        };
    }
}

fn setup(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Lab Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Light"),
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.13, 0.15, 0.18))),
    ));

    let mut builder = StateMachineBuilder::new("sandbox_lab");
    let target_visible = builder.blackboard_key(
        "target_visible",
        BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    let in_attack_range = builder.blackboard_key(
        "in_attack_range",
        BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let patrol = builder.atomic_state("Patrol");
    let combat = builder.compound_state("Combat");
    let stunned = builder.atomic_state("Stunned");
    let combat_region = builder.region("combat_region", combat);
    let chase = builder.atomic_state("Chase");
    let attack = builder.atomic_state("Attack");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(patrol, root)
        .add_state_to_region(combat, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, idle)
        .add_state_to_region(chase, combat_region)
        .add_state_to_region(attack, combat_region)
        .set_region_initial(combat_region, chase)
        .set_state_history_mode(combat, HistoryMode::Deep)
        .set_state_min_active_seconds(attack, 0.35)
        .add_transition(
            TransitionDefinition::replace(idle, patrol)
                .with_trigger(TransitionTrigger::after_seconds(0.8)),
        )
        .add_transition(TransitionDefinition::replace(patrol, combat).with_guard(GUARD_VISIBLE))
        .add_transition(TransitionDefinition::replace(combat, patrol).with_guard(GUARD_HIDDEN))
        .add_transition(TransitionDefinition::replace(chase, attack).with_guard(GUARD_IN_RANGE))
        .add_transition(
            TransitionDefinition::replace(attack, chase)
                .with_guard(GUARD_OUT_OF_RANGE)
                .with_mode(TransitionMode::Pending),
        )
        .add_transition(
            TransitionDefinition::push(TransitionSource::AnyState, stunned)
                .with_signal(SIGNAL_STUN),
        )
        .add_transition(
            TransitionDefinition::pop(stunned).with_trigger(TransitionTrigger::after_seconds(1.0)),
        );

    let definition = builder.build().unwrap();
    let definition_id = definitions.register(definition.clone()).unwrap();
    commands.insert_resource(LabKeys {
        target_visible,
        in_attack_range,
    });

    commands.spawn((
        Name::new("Lab Agent"),
        LabAgent,
        StateMachineInstance::new(definition_id),
        Blackboard::from_schema(&definition.blackboard_schema),
        AiDebugAnnotations {
            circles: vec![
                AiDebugCircle {
                    radius: 3.0,
                    color: css::AQUA.into(),
                    offset: Vec3::new(0.0, 0.05, 0.0),
                },
                AiDebugCircle {
                    radius: 1.4,
                    color: css::ORANGE.into(),
                    offset: Vec3::new(0.0, 0.05, 0.0),
                },
            ],
            lines: vec![AiDebugLine {
                start: Vec3::ZERO,
                end: Vec3::new(2.2, 0.4, 0.0),
                color: css::WHITE.into(),
            }],
            paths: vec![AiDebugPath {
                points: vec![
                    Vec3::new(-2.0, 0.05, -1.0),
                    Vec3::new(-0.5, 0.05, 1.4),
                    Vec3::new(1.8, 0.05, -0.8),
                ],
                color: css::TURQUOISE.into(),
            }],
        },
        Mesh3d(meshes.add(Cuboid::new(0.65, 1.2, 0.65))),
        MeshMaterial3d(materials.add(Color::srgb(0.78, 0.42, 0.16))),
        Transform::from_xyz(0.0, 0.6, 0.0),
    ));

    commands.spawn((
        Name::new("Lab Target"),
        LabTarget,
        Mesh3d(meshes.add(Cuboid::from_length(0.28))),
        MeshMaterial3d(materials.add(Color::srgb(0.16, 0.74, 0.94))),
        Transform::from_xyz(2.2, 0.3, 0.0),
    ));
}

fn drive_machine(
    time: Res<Time>,
    pane: Res<StateMachineLabPane>,
    keys: Res<LabKeys>,
    mut clock: ResMut<LabClock>,
    mut signals: MessageWriter<StateMachineSignal>,
    mut agents: Query<(Entity, &Transform, &mut Blackboard), With<LabAgent>>,
    targets: Query<&Transform, With<LabTarget>>,
) {
    clock.elapsed += time.delta_secs();
    let stun_interval = pane.stun_interval.max(0.25);
    let cycle = (clock.elapsed / stun_interval).floor() as u32;
    let Ok((entity, agent_transform, mut blackboard)) = agents.single_mut() else {
        return;
    };
    let Ok(target_transform) = targets.single() else {
        return;
    };

    let distance = agent_transform
        .translation
        .distance(target_transform.translation);
    let target_visible = distance <= pane.visibility_radius;
    let in_attack_range = distance <= pane.attack_radius;
    blackboard.set(keys.target_visible, target_visible).unwrap();
    blackboard
        .set(keys.in_attack_range, in_attack_range)
        .unwrap();

    if target_visible && clock.stun_cycle != Some(cycle) {
        signals.write(StateMachineSignal::new(entity, SIGNAL_STUN));
        clock.stun_cycle = Some(cycle);
    }
}

fn animate_target(time: Res<Time>, mut query: Query<&mut Transform, With<LabTarget>>) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };
    let angle = time.elapsed_secs() * 0.7;
    transform.translation = Vec3::new(angle.cos() * 2.4, 0.3, angle.sin() * 1.6);
}

fn sync_annotations(
    target_query: Query<&Transform, With<LabTarget>>,
    mut agent_query: Query<
        (&Transform, &StateMachineInstance, &mut AiDebugAnnotations),
        With<LabAgent>,
    >,
) {
    let Ok(target) = target_query.single() else {
        return;
    };
    let Ok((agent_transform, instance, mut annotations)) = agent_query.single_mut() else {
        return;
    };
    if let Some(line) = annotations.lines.first_mut() {
        line.start = agent_transform.translation + Vec3::Y * 0.5;
        line.end = target.translation;
        line.color = if !instance.stack.is_empty() {
            css::HOT_PINK.into()
        } else {
            css::WHITE.into()
        };
    }
}
