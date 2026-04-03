use bevy::color::palettes::css;
use bevy::ecs::message::Messages;
use bevy::prelude::*;

use crate::blackboard::Blackboard;
use crate::debug::{
    AiDebugAnnotations, AiDebugGizmos, StateMachineTraceEntry, TraceKind, TransitionBlockedReason,
};
use crate::definition::{
    StateId, StateKind, StateMachineDefinition, StateMachineLibrary, TransitionDefinition,
    TransitionId, TransitionMode, TransitionOperation, TransitionSource, TransitionTrigger,
    UtilityPolicy,
};
use crate::hierarchy::{depth, least_common_ancestor, path_to_root};
use crate::instance::{
    PendingTransition, StateMachineEvaluationMode, StateMachineInstance, StateMachineStatus,
};
use crate::messages::{
    StateEntered, StateExited, StateMachineSignal, TransitionBlocked, TransitionTriggered,
};
use crate::regions::region_is_enabled;
use crate::stack::{ActiveRegionState, HistorySnapshot, StateStackFrame};
use crate::timers::{decay_toward_zero, tick_active};
use crate::transitions::{
    EvaluatedTransition, StateMachineCallbacks, choose_best_candidate, pending_transition_id,
    threshold_for,
};

#[derive(Default)]
struct TransitionMessageBuffer {
    triggered: Vec<TransitionTriggered>,
    entered: Vec<StateEntered>,
    exited: Vec<StateExited>,
}

#[derive(Default)]
struct TransitionStepDelta {
    entered_states: Vec<StateId>,
    exited_states: Vec<StateId>,
}

pub fn activate_instances(world: &mut World) {
    let mut query = world.query::<&mut StateMachineInstance>();
    for mut instance in query.iter_mut(world) {
        if matches!(instance.status, StateMachineStatus::Inactive) {
            instance.status = StateMachineStatus::Active;
        }
    }
}

pub fn deactivate_instances(world: &mut World) {
    let mut query = world.query::<&mut StateMachineInstance>();
    for mut instance in query.iter_mut(world) {
        instance.status = StateMachineStatus::Inactive;
        instance.pending_transition = None;
    }
}

pub fn intake_signals(
    definitions: Res<StateMachineLibrary>,
    mut signals: MessageReader<StateMachineSignal>,
    mut query: Query<(Entity, &mut StateMachineInstance, Option<&mut Blackboard>)>,
) {
    for signal in signals.read() {
        if let Ok((_, mut instance, _)) = query.get_mut(signal.entity) {
            instance.queue_signal(signal.signal_id);
        }
    }

    for (_, mut instance, blackboard) in &mut query {
        let Some(definition) = definitions.definition(instance.definition_id) else {
            continue;
        };
        if !instance.is_active() {
            continue;
        }

        if let Some(mut blackboard) = blackboard {
            blackboard.ensure_schema(&definition.blackboard_schema);
            if matches!(instance.status, StateMachineStatus::Uninitialized) {
                apply_blackboard_overrides(&instance, &mut blackboard);
            }
            ensure_runtime_shape(definition, &mut instance);
        } else {
            ensure_runtime_shape(definition, &mut instance);
        }
    }
}

pub fn advance_timers(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let mut query = world.query::<&mut StateMachineInstance>();
    for mut instance in query.iter_mut(world) {
        if !instance.is_active() {
            continue;
        }
        let mut active_indices: Vec<usize> = instance
            .active_regions
            .iter()
            .flat_map(|active_region| active_region.path.iter().map(|state| state.0 as usize))
            .collect();
        active_indices.sort_unstable();
        active_indices.dedup();
        tick_active(&mut instance.state_elapsed_seconds, active_indices, delta);
        decay_toward_zero(&mut instance.transition_cooldowns_seconds, delta);
    }
}

pub fn evaluate_transitions(world: &mut World) {
    let definitions = world.resource::<StateMachineLibrary>().clone();
    {
        let mut query = world.query::<(&mut StateMachineInstance, &mut Blackboard)>();
        for (mut instance, mut blackboard) in query.iter_mut(world) {
            if !instance.is_active() {
                continue;
            }
            let Some(definition) = definitions.definition(instance.definition_id) else {
                continue;
            };

            blackboard.ensure_schema(&definition.blackboard_schema);
            if matches!(instance.status, StateMachineStatus::Uninitialized) {
                apply_blackboard_overrides(&instance, &mut blackboard);
            }
            ensure_runtime_shape(definition, &mut instance);
            if matches!(instance.status, StateMachineStatus::Uninitialized) {
                initialize_instance(definition, &mut instance, &blackboard);
                instance.status = StateMachineStatus::Active;
            }
        }
    }

    let callbacks = world.resource::<StateMachineCallbacks>().clone();
    let updates: Vec<_> = {
        let mut query = world.query::<(Entity, &StateMachineInstance, &Blackboard)>();
        query
            .iter(world)
            .filter_map(|(entity, instance, blackboard)| {
                if !instance.is_active() {
                    return None;
                }
                if !should_evaluate_instance(instance, blackboard) {
                    return None;
                }
                let definition = definitions.definition(instance.definition_id)?;
                let best_candidate =
                    evaluate_instance(world, entity, definition, instance, blackboard, &callbacks);
                Some((entity, blackboard.revision, best_candidate))
            })
            .collect()
    };

    let mut blocked_messages = Vec::new();
    let mut query = world.query::<(Entity, &mut StateMachineInstance)>();
    for (entity, blackboard_revision, best_candidate) in updates {
        let Ok((_, mut instance)) = query.get_mut(world, entity) else {
            continue;
        };

        match best_candidate.map(|candidate| (candidate.transition_id, candidate.reason_if_blocked))
        {
            Some((transition_id, Some(TransitionBlockedReason::PendingExit))) => {
                let should_trace_pending = !matches!(
                    instance.pending_transition,
                    Some(PendingTransition::Waiting {
                        transition_id: existing,
                        ..
                    }) if existing == transition_id
                );
                instance.pending_transition = Some(PendingTransition::Waiting {
                    transition_id,
                    reason: TransitionBlockedReason::PendingExit,
                    waiting_since_revision: instance.runtime_revision,
                });
                if should_trace_pending {
                    let runtime_revision = instance.runtime_revision;
                    instance.trace.push(StateMachineTraceEntry {
                        frame_revision: blackboard_revision,
                        runtime_revision,
                        kind: TraceKind::PendingTransition(transition_id),
                    });
                }
            }
            Some((transition_id, Some(reason))) => {
                blocked_messages.push((entity, instance.definition_id, transition_id, reason));
                if instance.config.trace_config.record_blocked {
                    let runtime_revision = instance.runtime_revision;
                    instance.trace.push(StateMachineTraceEntry {
                        frame_revision: blackboard_revision,
                        runtime_revision,
                        kind: TraceKind::BlockedTransition {
                            transition_id,
                            reason,
                        },
                    });
                }
                instance.pending_transition = None;
            }
            Some((transition_id, None)) => {
                instance.pending_transition = Some(PendingTransition::Ready(transition_id));
            }
            None => {
                instance.pending_transition = None;
            }
        }
        instance.last_blackboard_revision = blackboard_revision;
    }

    drop(query);
    write_blocked_messages(world, blocked_messages);
}

fn should_evaluate_instance(instance: &StateMachineInstance, blackboard: &Blackboard) -> bool {
    if matches!(instance.status, StateMachineStatus::Uninitialized) {
        return true;
    }

    match instance.config.evaluation_mode {
        StateMachineEvaluationMode::EveryFrame => true,
        StateMachineEvaluationMode::OnSignalOrBlackboardChange => {
            !instance.pending_signals.is_empty()
                || blackboard.changed_since(instance.last_blackboard_revision)
        }
    }
}

pub fn execute_transitions(world: &mut World) {
    let definitions = world.resource::<StateMachineLibrary>().clone();
    let callbacks = world.resource::<StateMachineCallbacks>().clone();
    let mut message_buffer = TransitionMessageBuffer::default();

    let entities: Vec<Entity> = {
        let mut query = world.query::<(Entity, &StateMachineInstance)>();
        query
            .iter(world)
            .filter_map(|(entity, instance)| {
                pending_transition_id(&instance.pending_transition).map(|_| entity)
            })
            .collect()
    };

    for entity in entities {
        let Some((definition_id, blackboard_revision)) = world
            .get::<StateMachineInstance>(entity)
            .map(|instance| (instance.definition_id, instance.last_blackboard_revision))
        else {
            continue;
        };
        let Some(definition) = definitions.definition(definition_id) else {
            continue;
        };
        execute_entity(
            world,
            entity,
            definition,
            &callbacks,
            blackboard_revision,
            &mut message_buffer,
        );
    }

    {
        let mut messages = world.resource_mut::<Messages<TransitionTriggered>>();
        for message in message_buffer.triggered {
            messages.write(message);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<StateEntered>>();
        for message in message_buffer.entered {
            messages.write(message);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<StateExited>>();
        for message in message_buffer.exited {
            messages.write(message);
        }
    }
}

pub fn update_states(world: &mut World) {
    let definitions = world.resource::<StateMachineLibrary>().clone();
    let callbacks = world.resource::<StateMachineCallbacks>().clone();

    let entities: Vec<Entity> = {
        let mut query = world.query::<(Entity, &StateMachineInstance)>();
        query
            .iter(world)
            .filter_map(|(entity, instance)| instance.is_active().then_some(entity))
            .collect()
    };

    for entity in entities {
        let Some((definition_id, active_regions)) = world
            .get::<StateMachineInstance>(entity)
            .map(|instance| (instance.definition_id, instance.active_regions.clone()))
        else {
            continue;
        };
        let Some(definition) = definitions.definition(definition_id) else {
            continue;
        };

        let mut active_states = Vec::new();
        for active_region in &active_regions {
            for state in &active_region.path {
                if !active_states.contains(state) {
                    active_states.push(*state);
                }
            }
        }
        active_states.sort_by_key(|state| depth(definition, *state));
        if active_states.iter().all(|state_id| {
            definition
                .state(*state_id)
                .is_none_or(|state| state.on_update.is_empty())
        }) {
            continue;
        }

        let Some(instance_snapshot) = world.get::<StateMachineInstance>(entity).cloned() else {
            continue;
        };

        for state_id in active_states {
            let Some(state) = definition.state(state_id) else {
                continue;
            };
            for action_id in &state.on_update {
                if let Some(action) = callbacks.action(*action_id) {
                    action(
                        world,
                        entity,
                        definition,
                        &instance_snapshot,
                        &dummy_transition(),
                    );
                }
            }
        }
    }
}

pub fn debug_visualize(
    mut gizmos: Gizmos<AiDebugGizmos>,
    query: Query<(
        &Transform,
        &StateMachineInstance,
        Option<&AiDebugAnnotations>,
    )>,
) {
    for (transform, instance, annotations) in &query {
        let stack_depth = instance.stack.len() as f32;
        let color = state_color(instance.active_leaf_states.first().copied());
        gizmos.circle(
            transform.translation + Vec3::Y * 0.05,
            0.5 + stack_depth * 0.05,
            color,
        );

        if let Some(annotations) = annotations {
            for circle in &annotations.circles {
                gizmos.circle(
                    transform.translation + circle.offset,
                    circle.radius,
                    circle.color,
                );
            }
            for line in &annotations.lines {
                gizmos.line(line.start, line.end, line.color);
            }
            for path in &annotations.paths {
                for points in path.points.windows(2) {
                    gizmos.line(points[0], points[1], path.color);
                }
            }
        }
    }
}

fn evaluate_instance(
    world: &World,
    entity: Entity,
    definition: &StateMachineDefinition,
    instance: &StateMachineInstance,
    blackboard: &Blackboard,
    callbacks: &StateMachineCallbacks,
) -> Option<EvaluatedTransition> {
    let mut ready_candidates = Vec::new();
    let mut blocked_candidates = Vec::new();
    for transition in &definition.transitions {
        let Some(source_depth) = source_depth(definition, instance, transition) else {
            continue;
        };

        let mut blocked_reason = trigger_block_reason(definition, instance, transition);
        if blocked_reason.is_none() {
            blocked_reason = transition
                .guard
                .and_then(|guard_id| callbacks.guard(guard_id))
                .filter(|guard| !guard(world, entity, definition, instance, blackboard, transition))
                .map(|_| TransitionBlockedReason::GuardFalse);
        }
        let score = if blocked_reason.is_none() {
            score_transition(
                world, entity, definition, instance, blackboard, transition, callbacks,
            )
        } else {
            0.0
        };
        if blocked_reason.is_none()
            && !matches!(transition.utility_policy, UtilityPolicy::Disabled)
            && score < threshold_for(instance, transition.id, transition.utility_policy)
        {
            blocked_reason = Some(TransitionBlockedReason::UtilityBelowThreshold);
        }
        if blocked_reason.is_none()
            && instance
                .transition_cooldowns_seconds
                .get(transition.id.0 as usize)
                .is_some_and(|cooldown| *cooldown > 0.0)
        {
            blocked_reason = Some(TransitionBlockedReason::CooldownActive);
        }
        if blocked_reason.is_none()
            && transition.debounce_seconds > 0.0
            && instance
                .guard_true_for_seconds
                .get(transition.id.0 as usize)
                .is_some_and(|elapsed| *elapsed < transition.debounce_seconds)
        {
            blocked_reason = Some(TransitionBlockedReason::DebounceActive);
        }
        if blocked_reason.is_none() {
            blocked_reason = match transition.operation {
                TransitionOperation::Push
                    if instance.stack.len() >= instance.config.max_stack_depth =>
                {
                    Some(TransitionBlockedReason::StackOverflow)
                }
                TransitionOperation::Pop if instance.stack.is_empty() => {
                    Some(TransitionBlockedReason::StackEmpty)
                }
                TransitionOperation::Replace
                | TransitionOperation::Push
                | TransitionOperation::Pop => None,
            };
        }
        if blocked_reason.is_none()
            && !can_exit_transition(
                world, entity, definition, instance, blackboard, transition, callbacks,
            )
        {
            blocked_reason = Some(match transition.mode {
                TransitionMode::Pending => TransitionBlockedReason::PendingExit,
                _ => TransitionBlockedReason::ExitNotReady,
            });
        }
        let candidate = EvaluatedTransition {
            transition_id: transition.id,
            source_depth,
            priority: transition.priority,
            score,
            declaration_order: transition.declaration_order,
            reason_if_blocked: blocked_reason,
        };
        if candidate.reason_if_blocked.is_none() {
            ready_candidates.push(candidate);
        } else {
            blocked_candidates.push(candidate);
        }
    }
    choose_best_candidate(&ready_candidates).or_else(|| choose_best_candidate(&blocked_candidates))
}

fn execute_entity(
    world: &mut World,
    entity: Entity,
    definition: &StateMachineDefinition,
    callbacks: &StateMachineCallbacks,
    blackboard_revision: u64,
    message_buffer: &mut TransitionMessageBuffer,
) {
    let Some(instance_snapshot) = world.get::<StateMachineInstance>(entity).cloned() else {
        return;
    };
    let Some(pending) = instance_snapshot.pending_transition.clone() else {
        return;
    };
    let initial_transition_id = match pending {
        PendingTransition::Ready(id) => id,
        PendingTransition::Waiting { .. } => return,
    };
    let Some(initial_transition) = definition.transition(initial_transition_id).cloned() else {
        return;
    };

    let mut current = instance_snapshot.clone();
    let mut next_transition = Some(initial_transition);
    let mut steps = 0usize;

    while let Some(transition) = next_transition.take() {
        let pre_transition_path = current.active_path.clone();
        let mut step_delta = TransitionStepDelta::default();
        steps += 1;
        if steps > current.config.max_internal_steps {
            if let Some(mut instance) = world.get_mut::<StateMachineInstance>(entity) {
                instance.pending_transition = None;
                let runtime_revision = instance.runtime_revision;
                instance.trace.push(StateMachineTraceEntry {
                    frame_revision: blackboard_revision,
                    runtime_revision,
                    kind: TraceKind::BlockedTransition {
                        transition_id: initial_transition_id,
                        reason: TransitionBlockedReason::MaxInternalStepsReached,
                    },
                });
            }
            break;
        }

        if !apply_transition(
            world,
            entity,
            definition,
            callbacks,
            &transition,
            &mut current,
            &mut step_delta,
        ) {
            break;
        }
        let post_transition_path = current.active_path.clone();

        let Some(mut instance) = world.get_mut::<StateMachineInstance>(entity) else {
            break;
        };
        *instance = current.clone();
        instance.pending_transition = None;
        instance.bump_revision();
        let runtime_revision = instance.runtime_revision;
        for state_id in &step_delta.exited_states {
            let runtime_revision = instance.runtime_revision;
            instance.trace.push(StateMachineTraceEntry {
                frame_revision: blackboard_revision,
                runtime_revision,
                kind: TraceKind::ExitedState(*state_id),
            });
        }
        instance.trace.push(StateMachineTraceEntry {
            frame_revision: blackboard_revision,
            runtime_revision,
            kind: TraceKind::TriggeredTransition(transition.id),
        });
        for state_id in &step_delta.entered_states {
            let runtime_revision = instance.runtime_revision;
            instance.trace.push(StateMachineTraceEntry {
                frame_revision: blackboard_revision,
                runtime_revision,
                kind: TraceKind::EnteredState(*state_id),
            });
        }
        current = instance.clone();

        message_buffer.triggered.push(TransitionTriggered {
            entity,
            definition_id: definition.id,
            transition_id: transition.id,
            operation: transition.operation,
            source: match transition.source {
                TransitionSource::State(source) => Some(source),
                TransitionSource::AnyState => None,
            },
            target: transition.target,
        });
        for state_id in &step_delta.exited_states {
            message_buffer.exited.push(StateExited {
                entity,
                definition_id: definition.id,
                state_id: *state_id,
                active_path: pre_transition_path.clone(),
            });
        }
        for state_id in &step_delta.entered_states {
            message_buffer.entered.push(StateEntered {
                entity,
                definition_id: definition.id,
                state_id: *state_id,
                active_path: post_transition_path.clone(),
            });
        }

        let Some(blackboard) = world.get::<Blackboard>(entity).cloned() else {
            break;
        };
        let chained_candidate =
            evaluate_instance(world, entity, definition, &current, &blackboard, callbacks)
                .and_then(|candidate| {
                    candidate
                        .reason_if_blocked
                        .is_none()
                        .then_some(candidate.transition_id)
                })
                .and_then(|transition_id| definition.transition(transition_id).cloned())
                .filter(|transition| match transition.trigger {
                    TransitionTrigger::Done => true,
                    TransitionTrigger::Automatic => transition
                        .source_state()
                        .and_then(|state_id| definition.state(state_id))
                        .is_some_and(|state| state.kind == StateKind::Transient),
                    TransitionTrigger::AfterSeconds(_) | TransitionTrigger::Signal(_) => false,
                });

        next_transition = chained_candidate;
    }
}

fn apply_transition(
    world: &mut World,
    entity: Entity,
    definition: &StateMachineDefinition,
    callbacks: &StateMachineCallbacks,
    transition: &TransitionDefinition,
    instance: &mut StateMachineInstance,
    step_delta: &mut TransitionStepDelta,
) -> bool {
    let Some(instance_before_actions) = world.get::<StateMachineInstance>(entity).cloned() else {
        return false;
    };

    let affected_paths: Vec<ActiveRegionState> = match transition.source {
        TransitionSource::AnyState => instance.active_regions.clone(),
        TransitionSource::State(source_state) => instance
            .active_regions
            .iter()
            .filter(|region| region.path.contains(&source_state))
            .cloned()
            .collect(),
    };

    let target = transition.target;
    let lca = match (transition.source, target) {
        (TransitionSource::State(source), Some(target)) => {
            least_common_ancestor(definition, source, target)
        }
        _ => None,
    };

    let mut states_to_exit = Vec::new();
    for active_region in &affected_paths {
        for state_id in active_region.path.iter().rev() {
            if Some(*state_id) == lca {
                break;
            }
            if !states_to_exit.contains(state_id) {
                states_to_exit.push(*state_id);
            }
        }
    }
    states_to_exit.sort_by_key(|state_id| std::cmp::Reverse(depth(definition, *state_id)));

    for state_id in &states_to_exit {
        let Some(state) = definition.state(*state_id) else {
            continue;
        };
        update_history_for_exit(instance, *state_id);
        for action_id in &state.on_exit {
            if let Some(action) = callbacks.action(*action_id) {
                action(
                    world,
                    entity,
                    definition,
                    &instance_before_actions,
                    transition,
                );
            }
        }
        step_delta.exited_states.push(*state_id);
    }

    for action_id in &transition.actions {
        if let Some(action) = callbacks.action(*action_id) {
            action(
                world,
                entity,
                definition,
                &instance_before_actions,
                transition,
            );
        }
    }

    match transition.operation {
        TransitionOperation::Replace => {
            for active_region in &affected_paths {
                instance
                    .active_regions
                    .retain(|region| region.region_id != active_region.region_id);
            }
            if let Some(target_state) = target {
                let entered_from = step_delta.entered_states.len();
                enter_target(
                    definition,
                    instance,
                    target_state,
                    lca,
                    &mut step_delta.entered_states,
                );
                let entered_snapshot = instance.clone();
                for state_id in step_delta.entered_states.iter().skip(entered_from) {
                    let Some(state) = definition.state(*state_id) else {
                        continue;
                    };
                    for action_id in &state.on_enter {
                        if let Some(action) = callbacks.action(*action_id) {
                            action(world, entity, definition, &entered_snapshot, transition);
                        }
                    }
                }
            }
        }
        TransitionOperation::Push => {
            let frame = StateStackFrame {
                active_regions: instance.active_regions.clone(),
                history: instance.history.clone(),
                state_elapsed_seconds: instance.state_elapsed_seconds.clone(),
                transition_cooldowns_seconds: instance.transition_cooldowns_seconds.clone(),
                guard_true_for_seconds: instance.guard_true_for_seconds.clone(),
                pending_transition: pending_transition_id(&instance.pending_transition),
            };
            if !instance.stack.push(frame) {
                return false;
            }
            instance.clear_active_state();
            if let Some(target_state) = target {
                let entered_from = step_delta.entered_states.len();
                enter_target(
                    definition,
                    instance,
                    target_state,
                    None,
                    &mut step_delta.entered_states,
                );
                let entered_snapshot = instance.clone();
                for state_id in step_delta.entered_states.iter().skip(entered_from) {
                    let Some(state) = definition.state(*state_id) else {
                        continue;
                    };
                    for action_id in &state.on_enter {
                        if let Some(action) = callbacks.action(*action_id) {
                            action(world, entity, definition, &entered_snapshot, transition);
                        }
                    }
                }
            }
        }
        TransitionOperation::Pop => {
            let Some(frame) = instance.stack.pop() else {
                return false;
            };
            instance.active_regions = frame.active_regions;
            instance.history = frame.history;
            instance.state_elapsed_seconds = frame.state_elapsed_seconds;
            instance.transition_cooldowns_seconds = frame.transition_cooldowns_seconds;
            instance.guard_true_for_seconds = frame.guard_true_for_seconds;
            sync_active_path(instance);
        }
    }

    instance.pending_signals.clear();
    if let Some(cooldown) = instance
        .transition_cooldowns_seconds
        .get_mut(transition.id.0 as usize)
    {
        *cooldown = transition.cooldown_seconds;
    }

    true
}

fn enter_target(
    definition: &StateMachineDefinition,
    instance: &mut StateMachineInstance,
    target_state: StateId,
    lca: Option<StateId>,
    entered_states: &mut Vec<StateId>,
) {
    if let Some(state) = definition.state(target_state) {
        let path = path_to_root(definition, target_state);
        let start_index = lca
            .and_then(|ancestor| path.iter().position(|state_id| *state_id == ancestor))
            .map(|index| index + 1)
            .unwrap_or(0);
        for state_id in path.iter().skip(start_index) {
            if !entered_states.contains(state_id) {
                entered_states.push(*state_id);
            }
        }

        if matches!(state.kind, StateKind::Compound | StateKind::Parallel) {
            restore_or_enter_children(definition, instance, target_state, entered_states);
        } else {
            let region_id = state.parent_region.unwrap_or(definition.root_regions[0]);
            instance.active_regions.push(ActiveRegionState {
                region_id,
                leaf_state: target_state,
                path,
            });
        }
        sync_active_path(instance);
    }
}

fn restore_or_enter_children(
    definition: &StateMachineDefinition,
    instance: &mut StateMachineInstance,
    state_id: StateId,
    entered_states: &mut Vec<StateId>,
) {
    let Some(state) = definition.state(state_id) else {
        return;
    };

    let history = instance
        .history
        .get(state_id.0 as usize)
        .and_then(Option::clone);

    for region_id in &state.child_regions {
        if !region_is_enabled(&instance.config.enabled_regions, *region_id) {
            continue;
        }
        let Some(region) = definition.region(*region_id) else {
            continue;
        };
        let target_state = match (state.history_mode, history.as_ref()) {
            (crate::definition::HistoryMode::Shallow, Some(history)) => history
                .shallow_children
                .iter()
                .find(|(id, _)| *id == *region_id)
                .map(|(_, state_id)| *state_id)
                .or(region.initial_state),
            (crate::definition::HistoryMode::Deep, Some(history)) => history
                .deep_regions
                .iter()
                .find(|entry| leaf_belongs_to_region(definition, entry.leaf_state, *region_id))
                .map(|entry| entry.leaf_state)
                .or(region.initial_state),
            _ => region.initial_state,
        };

        if let Some(target_state) = target_state {
            enter_target(
                definition,
                instance,
                target_state,
                Some(state_id),
                entered_states,
            );
        }
    }
}

fn leaf_belongs_to_region(
    definition: &StateMachineDefinition,
    mut state_id: StateId,
    region_id: crate::definition::RegionId,
) -> bool {
    while let Some(state) = definition.state(state_id) {
        if state.parent_region == Some(region_id) {
            return true;
        }
        let Some(parent) = state.parent_state else {
            return false;
        };
        state_id = parent;
    }
    false
}

fn initialize_instance(
    definition: &StateMachineDefinition,
    instance: &mut StateMachineInstance,
    blackboard: &Blackboard,
) {
    instance.history = vec![None; definition.states.len()];
    instance.state_elapsed_seconds = vec![0.0; definition.states.len()];
    instance.transition_cooldowns_seconds = vec![0.0; definition.transitions.len()];
    instance.guard_true_for_seconds = vec![0.0; definition.transitions.len()];
    instance.trace = crate::debug::StateMachineTrace::new(instance.config.trace_config.clone());
    instance.last_blackboard_revision = blackboard.revision;
    instance.active_regions.clear();
    instance.active_leaf_states.clear();
    instance.active_path.clear();

    let mut entered_states = Vec::new();
    for region_id in &definition.root_regions {
        if !region_is_enabled(&instance.config.enabled_regions, *region_id) {
            continue;
        }
        let Some(region) = definition.region(*region_id) else {
            continue;
        };
        if let Some(initial_state) = region.initial_state {
            enter_target(
                definition,
                instance,
                initial_state,
                None,
                &mut entered_states,
            );
        }
    }
}

fn apply_blackboard_overrides(instance: &StateMachineInstance, blackboard: &mut Blackboard) {
    for override_ in &instance.config.blackboard_overrides {
        let _ = blackboard.set(override_.key, override_.value.clone());
    }
}

fn ensure_runtime_shape(definition: &StateMachineDefinition, instance: &mut StateMachineInstance) {
    if instance.history.len() != definition.states.len() {
        instance.history.resize(definition.states.len(), None);
    }
    if instance.state_elapsed_seconds.len() != definition.states.len() {
        instance
            .state_elapsed_seconds
            .resize(definition.states.len(), 0.0);
    }
    if instance.transition_cooldowns_seconds.len() != definition.transitions.len() {
        instance
            .transition_cooldowns_seconds
            .resize(definition.transitions.len(), 0.0);
    }
    if instance.guard_true_for_seconds.len() != definition.transitions.len() {
        instance
            .guard_true_for_seconds
            .resize(definition.transitions.len(), 0.0);
    }
}

fn update_history_for_exit(instance: &mut StateMachineInstance, state_id: StateId) {
    let shallow_children = instance
        .active_regions
        .iter()
        .filter_map(|region| {
            let index = region
                .path
                .iter()
                .position(|candidate| *candidate == state_id)?;
            region
                .path
                .get(index + 1)
                .copied()
                .map(|child| (region.region_id, child))
        })
        .collect::<Vec<_>>();

    let deep_regions = instance
        .active_regions
        .iter()
        .filter(|region| region.path.contains(&state_id))
        .cloned()
        .collect::<Vec<_>>();

    if let Some(slot) = instance.history.get_mut(state_id.0 as usize) {
        *slot = Some(HistorySnapshot {
            shallow_children,
            deep_regions,
        });
    }
}

fn can_exit_transition(
    world: &World,
    entity: Entity,
    definition: &StateMachineDefinition,
    instance: &StateMachineInstance,
    blackboard: &Blackboard,
    transition: &TransitionDefinition,
    callbacks: &StateMachineCallbacks,
) -> bool {
    if matches!(transition.mode, TransitionMode::Force) {
        return true;
    }

    let states_to_check = match transition.source {
        TransitionSource::AnyState => instance
            .active_regions
            .iter()
            .flat_map(|region| region.path.iter().copied())
            .collect::<Vec<_>>(),
        TransitionSource::State(source) => instance
            .active_regions
            .iter()
            .filter(|region| region.path.contains(&source))
            .flat_map(|region| {
                region
                    .path
                    .iter()
                    .rev()
                    .copied()
                    .take_while(move |state| *state != source)
                    .chain(std::iter::once(source))
            })
            .collect::<Vec<_>>(),
    };

    for state_id in states_to_check {
        let Some(state) = definition.state(state_id) else {
            continue;
        };
        if instance
            .state_elapsed_seconds
            .get(state_id.0 as usize)
            .is_some_and(|elapsed| *elapsed < state.min_active_seconds)
        {
            return false;
        }
        if let Some(guard_id) = state.exit_guard
            && let Some(guard) = callbacks.guard(guard_id)
            && !guard(world, entity, definition, instance, blackboard, transition)
        {
            return false;
        }
    }

    true
}

fn trigger_block_reason(
    definition: &StateMachineDefinition,
    instance: &StateMachineInstance,
    transition: &TransitionDefinition,
) -> Option<TransitionBlockedReason> {
    match transition.trigger {
        TransitionTrigger::Automatic => None,
        TransitionTrigger::AfterSeconds(milliseconds) => {
            let required = milliseconds as f32 / 1_000.0;
            let Some(source) = transition.source_state() else {
                return Some(TransitionBlockedReason::InvalidTransition);
            };
            let elapsed = instance
                .state_elapsed_seconds
                .get(source.0 as usize)
                .copied()
                .unwrap_or_default();
            (elapsed < required).then_some(TransitionBlockedReason::GuardFalse)
        }
        TransitionTrigger::Signal(signal_id) => (!instance.pending_signals.contains(&signal_id))
            .then_some(TransitionBlockedReason::GuardFalse),
        TransitionTrigger::Done => {
            let Some(source_state) = transition.source_state() else {
                return Some(TransitionBlockedReason::InvalidTransition);
            };
            let Some(state) = definition.state(source_state) else {
                return Some(TransitionBlockedReason::InvalidTransition);
            };
            if !matches!(state.kind, StateKind::Compound | StateKind::Parallel) {
                return Some(TransitionBlockedReason::InvalidTransition);
            }
            let all_done = state.child_regions.iter().all(|region_id| {
                instance
                    .active_regions
                    .iter()
                    .filter(|active_region| active_region.region_id == *region_id)
                    .all(|active_region| {
                        definition
                            .state(active_region.leaf_state)
                            .is_some_and(|state| state.kind == StateKind::Final)
                    })
            });
            (!all_done).then_some(TransitionBlockedReason::GuardFalse)
        }
    }
}

fn score_transition(
    world: &World,
    entity: Entity,
    definition: &StateMachineDefinition,
    instance: &StateMachineInstance,
    blackboard: &Blackboard,
    transition: &TransitionDefinition,
    callbacks: &StateMachineCallbacks,
) -> f32 {
    transition
        .scorer
        .and_then(|id| callbacks.scorer(id))
        .map(|scorer| scorer(world, entity, definition, instance, blackboard, transition))
        .unwrap_or_default()
}

fn source_depth(
    definition: &StateMachineDefinition,
    instance: &StateMachineInstance,
    transition: &TransitionDefinition,
) -> Option<usize> {
    match transition.source {
        TransitionSource::AnyState => Some(0),
        TransitionSource::State(source_state) => instance
            .active_regions
            .iter()
            .any(|region| region.path.contains(&source_state))
            .then(|| depth(definition, source_state)),
    }
}

fn sync_active_path(instance: &mut StateMachineInstance) {
    instance
        .active_regions
        .sort_by_key(|region| (region.region_id.0, region.leaf_state.0));
    instance.active_leaf_states = instance
        .active_regions
        .iter()
        .map(|region| region.leaf_state)
        .collect();
    instance.active_path = instance
        .active_regions
        .first()
        .map(|region| region.path.clone())
        .unwrap_or_default();
}

fn state_color(state_id: Option<StateId>) -> Color {
    let Some(state_id) = state_id else {
        return css::GRAY.into();
    };
    match state_id.0 % 6 {
        0 => css::AQUA.into(),
        1 => css::LIME.into(),
        2 => css::GOLD.into(),
        3 => css::ORANGE.into(),
        4 => css::HOT_PINK.into(),
        _ => css::TURQUOISE.into(),
    }
}

fn write_blocked_messages(
    world: &mut World,
    blocked_messages: Vec<(
        Entity,
        crate::definition::StateMachineDefinitionId,
        TransitionId,
        TransitionBlockedReason,
    )>,
) {
    let mut messages = world.resource_mut::<Messages<TransitionBlocked>>();
    for (entity, definition_id, transition_id, reason) in blocked_messages {
        messages.write(TransitionBlocked {
            entity,
            definition_id,
            transition_id,
            reason,
        });
    }
}

fn dummy_transition() -> TransitionDefinition {
    TransitionDefinition::replace(StateId::default(), StateId::default())
}

impl TransitionDefinition {
    fn source_state(&self) -> Option<StateId> {
        match self.source {
            TransitionSource::State(source) => Some(source),
            TransitionSource::AnyState => None,
        }
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
