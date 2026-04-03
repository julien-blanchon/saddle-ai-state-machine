use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::blackboard::{BlackboardKeyId, BlackboardValue};
use crate::debug::{DebugTraceConfig, StateMachineTrace, TransitionBlockedReason};
use crate::definition::{SignalId, StateId, StateMachineDefinitionId, TransitionId};
use crate::stack::{ActiveRegionState, HistorySnapshot, StateStack};

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct InstanceBlackboardOverride {
    pub key: BlackboardKeyId,
    pub value: BlackboardValue,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct InstanceThresholdOverride {
    pub transition_id: TransitionId,
    pub minimum_score: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum PendingTransition {
    Ready(TransitionId),
    Waiting {
        transition_id: TransitionId,
        reason: TransitionBlockedReason,
        waiting_since_revision: u64,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum StateMachineStatus {
    #[default]
    Uninitialized,
    Active,
    Inactive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum StateMachineEvaluationMode {
    #[default]
    EveryFrame,
    OnSignalOrBlackboardChange,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct StateMachineInstanceConfig {
    pub evaluation_mode: StateMachineEvaluationMode,
    pub enabled_regions: Vec<crate::definition::RegionId>,
    pub trace_config: DebugTraceConfig,
    pub max_internal_steps: usize,
    pub max_stack_depth: usize,
    pub blackboard_overrides: Vec<InstanceBlackboardOverride>,
    pub utility_threshold_overrides: Vec<InstanceThresholdOverride>,
}

impl Default for StateMachineInstanceConfig {
    fn default() -> Self {
        Self {
            evaluation_mode: StateMachineEvaluationMode::EveryFrame,
            enabled_regions: Vec::new(),
            trace_config: DebugTraceConfig {
                capacity: 32,
                record_blocked: true,
            },
            max_internal_steps: 16,
            max_stack_depth: 8,
            blackboard_overrides: Vec::new(),
            utility_threshold_overrides: Vec::new(),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[require(crate::blackboard::Blackboard)]
#[reflect(Component)]
pub struct StateMachineInstance {
    pub definition_id: StateMachineDefinitionId,
    pub config: StateMachineInstanceConfig,
    pub status: StateMachineStatus,
    pub active_path: Vec<crate::definition::StateId>,
    pub active_leaf_states: Vec<crate::definition::StateId>,
    pub active_regions: Vec<ActiveRegionState>,
    pub pending_transition: Option<PendingTransition>,
    pub stack: StateStack,
    pub history: Vec<Option<HistorySnapshot>>,
    pub state_elapsed_seconds: Vec<f32>,
    pub transition_cooldowns_seconds: Vec<f32>,
    pub guard_true_for_seconds: Vec<f32>,
    pub pending_signals: Vec<crate::definition::SignalId>,
    pub runtime_revision: u64,
    pub last_blackboard_revision: u64,
    pub trace: StateMachineTrace,
}

impl StateMachineInstance {
    pub fn new(definition_id: StateMachineDefinitionId) -> Self {
        let config = StateMachineInstanceConfig::default();
        Self {
            definition_id,
            status: StateMachineStatus::Uninitialized,
            active_path: Vec::new(),
            active_leaf_states: Vec::new(),
            active_regions: Vec::new(),
            pending_transition: None,
            stack: StateStack::new(config.max_stack_depth),
            history: Vec::new(),
            state_elapsed_seconds: Vec::new(),
            transition_cooldowns_seconds: Vec::new(),
            guard_true_for_seconds: Vec::new(),
            pending_signals: Vec::new(),
            runtime_revision: 0,
            last_blackboard_revision: 0,
            trace: StateMachineTrace::new(config.trace_config.clone()),
            config,
        }
    }

    pub fn with_config(mut self, config: StateMachineInstanceConfig) -> Self {
        self.stack = StateStack::new(config.max_stack_depth);
        self.trace = StateMachineTrace::new(config.trace_config.clone());
        self.config = config;
        self
    }

    pub fn bump_revision(&mut self) {
        self.runtime_revision = self.runtime_revision.saturating_add(1);
    }

    pub fn clear_active_state(&mut self) {
        self.active_path.clear();
        self.active_leaf_states.clear();
        self.active_regions.clear();
        self.pending_transition = None;
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            StateMachineStatus::Active | StateMachineStatus::Uninitialized
        )
    }

    pub fn active_leaf(&self) -> Option<StateId> {
        self.active_leaf_states.first().copied()
    }

    pub fn is_in_state(&self, state_id: StateId) -> bool {
        self.active_path.contains(&state_id) || self.active_leaf_states.contains(&state_id)
    }

    pub fn queue_signal(&mut self, signal_id: SignalId) -> bool {
        if self.pending_signals.contains(&signal_id) {
            return false;
        }
        self.pending_signals.push(signal_id);
        true
    }

    pub fn has_signal(&self, signal_id: SignalId) -> bool {
        self.pending_signals.contains(&signal_id)
    }

    pub fn clear_signal(&mut self, signal_id: SignalId) -> bool {
        let before = self.pending_signals.len();
        self.pending_signals.retain(|queued| *queued != signal_id);
        before != self.pending_signals.len()
    }

    pub fn clear_signals(&mut self) {
        self.pending_signals.clear();
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
