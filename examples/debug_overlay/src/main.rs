use saddle_ai_state_machine_example_common as common;

use bevy::color::palettes::css;
use bevy::prelude::*;
use saddle_ai_state_machine::*;

const GUARD_VISIBLE: GuardId = GuardId(1);
const GUARD_HIDDEN: GuardId = GuardId(2);
const GUARD_IN_RANGE: GuardId = GuardId(3);
const GUARD_OUT_OF_RANGE: GuardId = GuardId(4);
const SIGNAL_STUN: SignalId = SignalId(1);

#[derive(Component)]
struct ShowcaseAgent;

#[derive(Component)]
struct DemoTarget;

#[derive(Component)]
struct OverlayText;

#[derive(Resource, Default)]
struct DemoTimeline {
    elapsed: f32,
    stun_cycle: Option<u32>,
}

#[derive(Resource, Clone, Copy)]
struct DemoKeys {
    target_visible: BlackboardKeyId,
    in_attack_range: BlackboardKeyId,
}

fn main() {
    let mut app = common::base_app("ai_state_machine debug overlay");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .init_resource::<DemoTimeline>()
        .add_systems(Startup, (setup_machine, setup_overlay))
        .add_systems(
            Update,
            (
                drive_demo_inputs,
                animate_target,
                sync_debug_annotations,
                update_overlay,
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

fn setup_machine(
    mut commands: Commands,
    mut definitions: ResMut<StateMachineLibrary>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut builder = StateMachineBuilder::new("debug_overlay");
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

    commands.insert_resource(DemoKeys {
        target_visible,
        in_attack_range,
    });

    commands.spawn((
        Name::new("Debug Overlay Agent"),
        ShowcaseAgent,
        StateMachineInstance::new(definition_id).with_config(StateMachineInstanceConfig {
            trace_config: DebugTraceConfig {
                capacity: 12,
                record_blocked: true,
            },
            ..default()
        }),
        Blackboard::from_schema(&definition.blackboard_schema),
        AiDebugAnnotations {
            circles: vec![
                AiDebugCircle {
                    radius: 3.2,
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
                end: Vec3::new(2.5, 0.3, 0.0),
                color: css::WHITE.into(),
            }],
            paths: vec![AiDebugPath {
                points: vec![
                    Vec3::new(-2.0, 0.05, -1.5),
                    Vec3::new(-0.5, 0.05, 1.2),
                    Vec3::new(1.5, 0.05, -0.6),
                ],
                color: css::TURQUOISE.into(),
            }],
        },
        Mesh3d(meshes.add(Cuboid::new(0.65, 1.2, 0.65))),
        MeshMaterial3d(materials.add(Color::srgb(0.82, 0.48, 0.18))),
        Transform::from_xyz(0.0, 0.6, 0.0),
    ));

    commands.spawn((
        Name::new("Debug Overlay Target"),
        DemoTarget,
        Mesh3d(meshes.add(Cuboid::from_length(0.25))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.72, 0.95))),
        Transform::from_xyz(2.5, 0.3, 0.0),
    ));
}

fn setup_overlay(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                left: px(12),
                width: px(420),
                padding: UiRect::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.05, 0.09, 0.88)),
        ))
        .with_child((
            Text::new("waiting for machine..."),
            TextFont::from_font_size(14.0),
            TextColor(Color::WHITE),
            OverlayText,
        ));
}

fn drive_demo_inputs(
    time: Res<Time>,
    keys: Res<DemoKeys>,
    mut timeline: ResMut<DemoTimeline>,
    mut signals: MessageWriter<StateMachineSignal>,
    mut agent_query: Query<(Entity, &mut Blackboard), With<ShowcaseAgent>>,
) {
    timeline.elapsed += time.delta_secs();
    let cycle = (timeline.elapsed / 10.0).floor() as u32;
    let phase = timeline.elapsed % 10.0;

    let Ok((entity, mut blackboard)) = agent_query.single_mut() else {
        return;
    };

    let target_visible = (1.5..8.0).contains(&phase);
    let in_attack_range = (4.0..5.5).contains(&phase) || (6.8..7.6).contains(&phase);
    blackboard.set(keys.target_visible, target_visible).unwrap();
    blackboard
        .set(keys.in_attack_range, in_attack_range)
        .unwrap();

    if phase >= 5.5 && timeline.stun_cycle != Some(cycle) {
        signals.write(StateMachineSignal::new(entity, SIGNAL_STUN));
        timeline.stun_cycle = Some(cycle);
    }
}

fn animate_target(time: Res<Time>, mut query: Query<&mut Transform, With<DemoTarget>>) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };
    let angle = time.elapsed_secs() * 0.7;
    transform.translation = Vec3::new(angle.cos() * 2.4, 0.3, angle.sin() * 1.6);
}

fn sync_debug_annotations(
    target_query: Query<&Transform, With<DemoTarget>>,
    mut agent_query: Query<
        (&Transform, &StateMachineInstance, &mut AiDebugAnnotations),
        With<ShowcaseAgent>,
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
        line.color = match instance.active_leaf() {
            Some(StateId(0)) => css::WHITE.into(),
            _ if !instance.stack.is_empty() => css::HOT_PINK.into(),
            _ => css::GOLD.into(),
        };
    }
}

fn update_overlay(
    definitions: Res<StateMachineLibrary>,
    keys: Res<DemoKeys>,
    agent_query: Query<(&StateMachineInstance, &Blackboard), With<ShowcaseAgent>>,
    mut text_query: Query<&mut Text, With<OverlayText>>,
) {
    let Ok((instance, blackboard)) = agent_query.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    let Some(definition) = definitions.definition(instance.definition_id) else {
        return;
    };

    let active_leaf = instance
        .active_leaf_states
        .iter()
        .filter_map(|state_id| definition.state(*state_id).map(|state| state.name.clone()))
        .collect::<Vec<_>>()
        .join(", ");
    let active_path = instance
        .active_path
        .iter()
        .filter_map(|state_id| definition.state(*state_id).map(|state| state.name.clone()))
        .collect::<Vec<_>>()
        .join(" > ");
    let trace = instance
        .trace
        .entries
        .iter()
        .rev()
        .take(4)
        .map(|entry| format_trace(definition, entry))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    **text = format!(
        "HFSM Debug Overlay\n\
leaf: {active_leaf}\n\
path: {active_path}\n\
stack depth: {}\n\
runtime revision: {}\n\
queued signals: {}\n\
target_visible: {}\n\
in_attack_range: {}\n\
trace:\n{}",
        instance.stack.len(),
        instance.runtime_revision,
        instance.pending_signals.len(),
        blackboard
            .get_bool(keys.target_visible)
            .unwrap()
            .unwrap_or(false),
        blackboard
            .get_bool(keys.in_attack_range)
            .unwrap()
            .unwrap_or(false),
        if trace.is_empty() {
            "  <empty>".to_string()
        } else {
            trace
                .lines()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
    );
}

fn format_trace(definition: &StateMachineDefinition, entry: &StateMachineTraceEntry) -> String {
    match &entry.kind {
        TraceKind::EnteredState(state_id) => format!(
            "enter {}",
            definition
                .state(*state_id)
                .map(|state| state.name.as_str())
                .unwrap_or("<unknown>")
        ),
        TraceKind::ExitedState(state_id) => format!(
            "exit {}",
            definition
                .state(*state_id)
                .map(|state| state.name.as_str())
                .unwrap_or("<unknown>")
        ),
        TraceKind::TriggeredTransition(transition_id) => {
            format!("transition {:?}", transition_id)
        }
        TraceKind::BlockedTransition {
            transition_id,
            reason,
        } => format!("blocked {:?}: {:?}", transition_id, reason),
        TraceKind::PendingTransition(transition_id) => {
            format!("pending {:?}", transition_id)
        }
    }
}
