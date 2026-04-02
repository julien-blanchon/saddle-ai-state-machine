use std::sync::Arc;

use bevy::prelude::*;

use crate::blackboard::Blackboard;
use crate::debug::TransitionBlockedReason;
use crate::definition::{
    ActionId, GuardId, ScorerId, StateMachineDefinition, TransitionDefinition, TransitionId,
    UtilityPolicy,
};
use crate::instance::{PendingTransition, StateMachineInstance};

pub type GuardCallback = dyn Fn(
        &World,
        Entity,
        &StateMachineDefinition,
        &StateMachineInstance,
        &Blackboard,
        &TransitionDefinition,
    ) -> bool
    + Send
    + Sync
    + 'static;

pub type ActionCallback = dyn Fn(&mut World, Entity, &StateMachineDefinition, &StateMachineInstance, &TransitionDefinition)
    + Send
    + Sync
    + 'static;

pub type ScorerCallback = dyn Fn(
        &World,
        Entity,
        &StateMachineDefinition,
        &StateMachineInstance,
        &Blackboard,
        &TransitionDefinition,
    ) -> f32
    + Send
    + Sync
    + 'static;

#[derive(Resource, Default, Clone)]
pub struct StateMachineCallbacks {
    guards: Vec<Option<Arc<GuardCallback>>>,
    actions: Vec<Option<Arc<ActionCallback>>>,
    scorers: Vec<Option<Arc<ScorerCallback>>>,
}

impl StateMachineCallbacks {
    pub fn register_guard<F>(&mut self, id: GuardId, callback: F)
    where
        F: Fn(
                &World,
                Entity,
                &StateMachineDefinition,
                &StateMachineInstance,
                &Blackboard,
                &TransitionDefinition,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        let index = id.0 as usize;
        if self.guards.len() <= index {
            self.guards.resize(index + 1, None);
        }
        self.guards[index] = Some(Arc::new(callback));
    }

    pub fn register_action<F>(&mut self, id: ActionId, callback: F)
    where
        F: Fn(
                &mut World,
                Entity,
                &StateMachineDefinition,
                &StateMachineInstance,
                &TransitionDefinition,
            ) + Send
            + Sync
            + 'static,
    {
        let index = id.0 as usize;
        if self.actions.len() <= index {
            self.actions.resize(index + 1, None);
        }
        self.actions[index] = Some(Arc::new(callback));
    }

    pub fn register_scorer<F>(&mut self, id: ScorerId, callback: F)
    where
        F: Fn(
                &World,
                Entity,
                &StateMachineDefinition,
                &StateMachineInstance,
                &Blackboard,
                &TransitionDefinition,
            ) -> f32
            + Send
            + Sync
            + 'static,
    {
        let index = id.0 as usize;
        if self.scorers.len() <= index {
            self.scorers.resize(index + 1, None);
        }
        self.scorers[index] = Some(Arc::new(callback));
    }

    pub fn guard(&self, id: GuardId) -> Option<&Arc<GuardCallback>> {
        self.guards.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub fn action(&self, id: ActionId) -> Option<&Arc<ActionCallback>> {
        self.actions.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub fn scorer(&self, id: ScorerId) -> Option<&Arc<ScorerCallback>> {
        self.scorers.get(id.0 as usize).and_then(Option::as_ref)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EvaluatedTransition {
    pub transition_id: TransitionId,
    pub source_depth: usize,
    pub priority: i32,
    pub score: f32,
    pub declaration_order: usize,
    pub reason_if_blocked: Option<TransitionBlockedReason>,
}

pub(crate) fn choose_best_candidate(
    candidates: &[EvaluatedTransition],
) -> Option<EvaluatedTransition> {
    let mut ordered = candidates.to_vec();
    ordered.sort_by(|left, right| {
        right
            .source_depth
            .cmp(&left.source_depth)
            .then_with(|| right.priority.cmp(&left.priority))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.declaration_order.cmp(&right.declaration_order))
    });
    ordered.into_iter().next()
}

pub(crate) fn threshold_for(
    instance: &StateMachineInstance,
    transition_id: TransitionId,
    policy: UtilityPolicy,
) -> f32 {
    instance
        .config
        .utility_threshold_overrides
        .iter()
        .find(|override_| override_.transition_id == transition_id)
        .map(|override_| override_.minimum_score)
        .unwrap_or_else(|| policy.minimum_score())
}

pub(crate) fn pending_transition_id(pending: &Option<PendingTransition>) -> Option<TransitionId> {
    match pending {
        Some(PendingTransition::Ready(id)) => Some(*id),
        Some(PendingTransition::Waiting { transition_id, .. }) => Some(*transition_id),
        None => None,
    }
}

#[cfg(test)]
#[path = "transitions_tests.rs"]
mod tests;
