use bevy::prelude::*;

use crate::blackboard::{
    BlackboardKeyDefinition, BlackboardKeyId, BlackboardValue, BlackboardValueType,
};
use crate::debug::DebugTraceConfig;
use crate::definition::{
    ActionId, GuardId, HistoryMode, RegionDefinition, RegionId, StateDefinition, StateId,
    StateKind, StateMachineDefinition, StateMachineDefinitionId, TransitionDefinition,
    TransitionId,
};
use crate::validation::{ValidationReport, validate_definition};

#[derive(Clone, Debug)]
pub struct StateMachineBuilder {
    name: String,
    definition_id: Option<StateMachineDefinitionId>,
    states: Vec<StateDefinition>,
    regions: Vec<RegionDefinition>,
    transitions: Vec<TransitionDefinition>,
    root_regions: Vec<RegionId>,
    blackboard_schema: Vec<BlackboardKeyDefinition>,
    debug_trace_config: DebugTraceConfig,
    supported_features: Vec<String>,
    deferred_features: Vec<String>,
}

impl StateMachineBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            definition_id: None,
            states: Vec::new(),
            regions: Vec::new(),
            transitions: Vec::new(),
            root_regions: Vec::new(),
            blackboard_schema: Vec::new(),
            debug_trace_config: DebugTraceConfig {
                capacity: 32,
                record_blocked: true,
            },
            supported_features: vec![
                "hierarchy".to_string(),
                "pushdown".to_string(),
                "utility_scoring".to_string(),
                "orthogonal_regions".to_string(),
                "delayed_transitions".to_string(),
                "signal_inbox".to_string(),
                "reflection_serialization".to_string(),
            ],
            deferred_features: vec![
                "sleep_wake_scheduling".to_string(),
                "marker_component_mirroring".to_string(),
                "scoped_blackboards".to_string(),
                "score_hysteresis".to_string(),
                "overlay_labels".to_string(),
            ],
        }
    }

    pub fn with_definition_id(mut self, id: StateMachineDefinitionId) -> Self {
        self.definition_id = Some(id);
        self
    }

    pub fn set_debug_trace_config(&mut self, config: DebugTraceConfig) -> &mut Self {
        self.debug_trace_config = config;
        self
    }

    pub fn atomic_state(&mut self, name: impl Into<String>) -> StateId {
        self.add_state(name, StateKind::Atomic)
    }

    pub fn compound_state(&mut self, name: impl Into<String>) -> StateId {
        self.add_state(name, StateKind::Compound)
    }

    pub fn parallel_state(&mut self, name: impl Into<String>) -> StateId {
        self.add_state(name, StateKind::Parallel)
    }

    pub fn final_state(&mut self, name: impl Into<String>) -> StateId {
        self.add_state(name, StateKind::Final)
    }

    pub fn transient_state(&mut self, name: impl Into<String>) -> StateId {
        self.add_state(name, StateKind::Transient)
    }

    pub fn root_region(&mut self, name: impl Into<String>) -> RegionId {
        let id = self.region_internal(name, None);
        self.root_regions.push(id);
        id
    }

    pub fn region(&mut self, name: impl Into<String>, parent_state: StateId) -> RegionId {
        let id = self.region_internal(name, Some(parent_state));
        if let Some(parent) = self.states.get_mut(parent_state.0 as usize) {
            parent.child_regions.push(id);
        }
        id
    }

    pub fn add_state_to_region(&mut self, state_id: StateId, region_id: RegionId) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.parent_region = Some(region_id);
            state.parent_state = self
                .regions
                .get(region_id.0 as usize)
                .and_then(|region| region.parent_state);
        }
        if let Some(region) = self.regions.get_mut(region_id.0 as usize) {
            region.child_states.push(state_id);
        }
        self
    }

    pub fn set_region_initial(&mut self, region_id: RegionId, state_id: StateId) -> &mut Self {
        if let Some(region) = self.regions.get_mut(region_id.0 as usize) {
            region.initial_state = Some(state_id);
        }
        self
    }

    pub fn set_state_history_mode(
        &mut self,
        state_id: StateId,
        history_mode: HistoryMode,
    ) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.history_mode = history_mode;
        }
        self
    }

    pub fn set_state_min_active_seconds(&mut self, state_id: StateId, seconds: f32) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.min_active_seconds = seconds.max(0.0);
        }
        self
    }

    pub fn set_state_exit_guard(&mut self, state_id: StateId, guard: GuardId) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.exit_guard = Some(guard);
        }
        self
    }

    pub fn set_state_always_tick(&mut self, state_id: StateId, always_tick: bool) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.always_tick = always_tick;
        }
        self
    }

    pub fn add_state_tag(&mut self, state_id: StateId, tag: impl Into<String>) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.tags.push(tag.into());
        }
        self
    }

    pub fn add_on_enter(&mut self, state_id: StateId, action: ActionId) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.on_enter.push(action);
        }
        self
    }

    pub fn add_on_update(&mut self, state_id: StateId, action: ActionId) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.on_update.push(action);
        }
        self
    }

    pub fn add_on_exit(&mut self, state_id: StateId, action: ActionId) -> &mut Self {
        if let Some(state) = self.states.get_mut(state_id.0 as usize) {
            state.on_exit.push(action);
        }
        self
    }

    pub fn add_transition(&mut self, mut transition: TransitionDefinition) -> &mut Self {
        transition.id = TransitionId(self.transitions.len() as u16);
        transition.declaration_order = self.transitions.len();
        self.transitions.push(transition);
        self
    }

    pub fn blackboard_key(
        &mut self,
        name: impl Into<String>,
        value_type: BlackboardValueType,
        required: bool,
        default_value: Option<BlackboardValue>,
    ) -> BlackboardKeyId {
        let id = BlackboardKeyId(self.blackboard_schema.len() as u16);
        self.blackboard_schema.push(BlackboardKeyDefinition {
            id,
            name: name.into(),
            value_type,
            required,
            default_value,
        });
        id
    }

    pub fn build(self) -> Result<StateMachineDefinition, ValidationReport> {
        let definition = StateMachineDefinition {
            id: self
                .definition_id
                .unwrap_or_else(|| StateMachineDefinitionId(stable_hash(&self.name))),
            name: self.name,
            states: self.states,
            regions: self.regions,
            transitions: self.transitions,
            root_regions: self.root_regions,
            blackboard_schema: self.blackboard_schema,
            debug_trace_config: self.debug_trace_config,
            supported_features: self.supported_features,
            deferred_features: self.deferred_features,
        };

        let report = validate_definition(&definition);
        if report.has_errors() {
            Err(report)
        } else {
            Ok(definition)
        }
    }

    fn add_state(&mut self, name: impl Into<String>, kind: StateKind) -> StateId {
        let id = StateId(self.states.len() as u16);
        self.states.push(StateDefinition {
            id,
            name: name.into(),
            kind,
            parent_state: None,
            parent_region: None,
            child_regions: Vec::new(),
            on_enter: Vec::new(),
            on_update: Vec::new(),
            on_exit: Vec::new(),
            exit_guard: None,
            min_active_seconds: 0.0,
            history_mode: HistoryMode::None,
            tags: Vec::new(),
            always_tick: false,
        });
        id
    }

    fn region_internal(
        &mut self,
        name: impl Into<String>,
        parent_state: Option<StateId>,
    ) -> RegionId {
        let id = RegionId(self.regions.len() as u16);
        self.regions.push(RegionDefinition {
            id,
            name: name.into(),
            parent_state,
            child_states: Vec::new(),
            initial_state: None,
        });
        id
    }
}

fn stable_hash(input: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
