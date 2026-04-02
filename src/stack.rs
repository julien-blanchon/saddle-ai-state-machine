use bevy::prelude::*;

use crate::definition::{StateId, TransitionId};

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct ActiveRegionState {
    pub region_id: crate::definition::RegionId,
    pub leaf_state: StateId,
    pub path: Vec<StateId>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct HistorySnapshot {
    pub shallow_children: Vec<(crate::definition::RegionId, StateId)>,
    pub deep_regions: Vec<ActiveRegionState>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct StateStackFrame {
    pub active_regions: Vec<ActiveRegionState>,
    pub history: Vec<Option<HistorySnapshot>>,
    pub state_elapsed_seconds: Vec<f32>,
    pub transition_cooldowns_seconds: Vec<f32>,
    pub guard_true_for_seconds: Vec<f32>,
    pub pending_transition: Option<TransitionId>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct StateStack {
    pub max_depth: usize,
    pub frames: Vec<StateStackFrame>,
}

impl StateStack {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            frames: Vec::new(),
        }
    }

    pub fn push(&mut self, frame: StateStackFrame) -> bool {
        if self.frames.len() >= self.max_depth {
            return false;
        }
        self.frames.push(frame);
        true
    }

    pub fn pop(&mut self) -> Option<StateStackFrame> {
        self.frames.pop()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[cfg(test)]
#[path = "stack_tests.rs"]
mod tests;
