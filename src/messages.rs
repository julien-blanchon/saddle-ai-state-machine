use bevy::prelude::*;

use crate::debug::TransitionBlockedReason;
use crate::definition::{
    SignalId, StateId, StateMachineDefinitionId, TransitionId, TransitionOperation,
};

#[derive(Clone, Debug, Message, Reflect)]
pub struct StateMachineSignal {
    pub entity: Entity,
    pub signal_id: SignalId,
}

impl StateMachineSignal {
    pub fn new(entity: Entity, signal_id: SignalId) -> Self {
        Self { entity, signal_id }
    }
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct StateEntered {
    pub entity: Entity,
    pub definition_id: StateMachineDefinitionId,
    pub state_id: StateId,
    pub active_path: Vec<StateId>,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct StateExited {
    pub entity: Entity,
    pub definition_id: StateMachineDefinitionId,
    pub state_id: StateId,
    pub active_path: Vec<StateId>,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct TransitionTriggered {
    pub entity: Entity,
    pub definition_id: StateMachineDefinitionId,
    pub transition_id: TransitionId,
    pub operation: TransitionOperation,
    pub source: Option<StateId>,
    pub target: Option<StateId>,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct TransitionBlocked {
    pub entity: Entity,
    pub definition_id: StateMachineDefinitionId,
    pub transition_id: TransitionId,
    pub reason: TransitionBlockedReason,
}
