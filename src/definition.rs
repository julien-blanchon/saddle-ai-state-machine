use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::blackboard::{BlackboardKeyDefinition, BlackboardKeyId};
use crate::debug::DebugTraceConfig;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct StateMachineDefinitionId(pub u64);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct StateId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct RegionId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct TransitionId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct GuardId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct ActionId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct ScorerId(pub u16);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct SignalId(pub u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum HistoryMode {
    #[default]
    None,
    Shallow,
    Deep,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum TransitionMode {
    #[default]
    Immediate,
    Pending,
    Force,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum TransitionOperation {
    #[default]
    Replace,
    Push,
    Pop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum TransitionTrigger {
    #[default]
    Automatic,
    AfterSeconds(u32),
    Signal(SignalId),
    Done,
}

impl TransitionTrigger {
    pub fn after_seconds(seconds: f32) -> Self {
        Self::AfterSeconds((seconds.max(0.0) * 1_000.0).round() as u32)
    }

    pub fn seconds(self) -> Option<f32> {
        match self {
            Self::AfterSeconds(milliseconds) => Some(milliseconds as f32 / 1_000.0),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum UtilityPolicy {
    #[default]
    Disabled,
    BestScore,
    BestScoreAbove {
        minimum_score_milli: u32,
    },
}

impl UtilityPolicy {
    pub fn best_score_above(score: f32) -> Self {
        Self::BestScoreAbove {
            minimum_score_milli: (score.max(0.0) * 1_000.0).round() as u32,
        }
    }

    pub fn minimum_score(self) -> f32 {
        match self {
            Self::Disabled | Self::BestScore => 0.0,
            Self::BestScoreAbove {
                minimum_score_milli,
            } => minimum_score_milli as f32 / 1_000.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum StateKind {
    #[default]
    Atomic,
    Compound,
    Parallel,
    Final,
    Transient,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum TransitionSource {
    State(StateId),
    #[default]
    AnyState,
}

impl From<StateId> for TransitionSource {
    fn from(value: StateId) -> Self {
        Self::State(value)
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct StateDefinition {
    pub id: StateId,
    pub name: String,
    pub kind: StateKind,
    pub parent_state: Option<StateId>,
    pub parent_region: Option<RegionId>,
    pub child_regions: Vec<RegionId>,
    pub on_enter: Vec<ActionId>,
    pub on_update: Vec<ActionId>,
    pub on_exit: Vec<ActionId>,
    pub exit_guard: Option<GuardId>,
    pub min_active_seconds: f32,
    pub history_mode: HistoryMode,
    pub tags: Vec<String>,
    pub always_tick: bool,
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct RegionDefinition {
    pub id: RegionId,
    pub name: String,
    pub parent_state: Option<StateId>,
    pub child_states: Vec<StateId>,
    pub initial_state: Option<StateId>,
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct TransitionDefinition {
    pub id: TransitionId,
    pub source: TransitionSource,
    pub operation: TransitionOperation,
    pub target: Option<StateId>,
    pub trigger: TransitionTrigger,
    pub guard: Option<GuardId>,
    pub actions: Vec<ActionId>,
    pub scorer: Option<ScorerId>,
    pub priority: i32,
    pub declaration_order: usize,
    pub mode: TransitionMode,
    pub utility_policy: UtilityPolicy,
    pub cooldown_seconds: f32,
    pub debounce_seconds: f32,
}

impl TransitionDefinition {
    pub fn replace(source: impl Into<TransitionSource>, target: StateId) -> Self {
        Self {
            id: TransitionId::default(),
            source: source.into(),
            operation: TransitionOperation::Replace,
            target: Some(target),
            trigger: TransitionTrigger::Automatic,
            guard: None,
            actions: Vec::new(),
            scorer: None,
            priority: 0,
            declaration_order: 0,
            mode: TransitionMode::Immediate,
            utility_policy: UtilityPolicy::Disabled,
            cooldown_seconds: 0.0,
            debounce_seconds: 0.0,
        }
    }

    pub fn push(source: impl Into<TransitionSource>, target: StateId) -> Self {
        Self {
            operation: TransitionOperation::Push,
            ..Self::replace(source, target)
        }
    }

    pub fn pop(source: impl Into<TransitionSource>) -> Self {
        Self {
            operation: TransitionOperation::Pop,
            target: None,
            ..Self::replace(source, StateId::default())
        }
    }

    pub fn with_trigger(mut self, trigger: TransitionTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    pub fn with_signal(mut self, signal: SignalId) -> Self {
        self.trigger = TransitionTrigger::Signal(signal);
        self
    }

    pub fn with_guard(mut self, guard: GuardId) -> Self {
        self.guard = Some(guard);
        self
    }

    pub fn with_action(mut self, action: ActionId) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_scorer(mut self, scorer: ScorerId, utility_policy: UtilityPolicy) -> Self {
        self.scorer = Some(scorer);
        self.utility_policy = utility_policy;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_mode(mut self, mode: TransitionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_cooldown(mut self, seconds: f32) -> Self {
        self.cooldown_seconds = seconds.max(0.0);
        self
    }

    pub fn with_debounce(mut self, seconds: f32) -> Self {
        self.debounce_seconds = seconds.max(0.0);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct StateMachineDefinition {
    pub id: StateMachineDefinitionId,
    pub name: String,
    pub states: Vec<StateDefinition>,
    pub regions: Vec<RegionDefinition>,
    pub transitions: Vec<TransitionDefinition>,
    pub root_regions: Vec<RegionId>,
    pub blackboard_schema: Vec<BlackboardKeyDefinition>,
    pub debug_trace_config: DebugTraceConfig,
    pub supported_features: Vec<String>,
    pub deferred_features: Vec<String>,
}

impl StateMachineDefinition {
    pub fn state(&self, id: StateId) -> Option<&StateDefinition> {
        self.states.get(id.0 as usize)
    }

    pub fn region(&self, id: RegionId) -> Option<&RegionDefinition> {
        self.regions.get(id.0 as usize)
    }

    pub fn transition(&self, id: TransitionId) -> Option<&TransitionDefinition> {
        self.transitions.get(id.0 as usize)
    }

    pub fn blackboard_key(&self, id: BlackboardKeyId) -> Option<&BlackboardKeyDefinition> {
        self.blackboard_schema.get(id.0 as usize)
    }

    pub fn find_state_id(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .find(|state| state.name == name)
            .map(|state| state.id)
    }

    pub fn find_blackboard_key(&self, name: &str) -> Option<BlackboardKeyId> {
        self.blackboard_schema
            .iter()
            .find(|key| key.name == name)
            .map(|key| key.id)
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct StateMachineLibrary {
    pub definitions: Vec<StateMachineDefinition>,
}

impl StateMachineLibrary {
    pub fn register(
        &mut self,
        definition: StateMachineDefinition,
    ) -> Result<StateMachineDefinitionId, String> {
        if self
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id)
        {
            return Err(format!(
                "state machine definition id {:?} already registered",
                definition.id
            ));
        }
        let id = definition.id;
        self.definitions.push(definition);
        Ok(id)
    }

    pub fn definition(&self, id: StateMachineDefinitionId) -> Option<&StateMachineDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.id == id)
    }
}

#[cfg(test)]
#[path = "definition_tests.rs"]
mod tests;
