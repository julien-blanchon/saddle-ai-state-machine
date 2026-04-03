use bevy::color::palettes::css;
use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::definition::{StateId, TransitionId};

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct DebugTraceConfig {
    pub capacity: usize,
    pub record_blocked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TransitionBlockedReason {
    GuardFalse,
    UtilityBelowThreshold,
    CooldownActive,
    DebounceActive,
    ExitNotReady,
    PendingExit,
    StackEmpty,
    StackOverflow,
    InvalidTransition,
    InvalidHistoryRestore,
    InvalidCrossRegionTransition,
    MaxInternalStepsReached,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum TraceKind {
    EnteredState(StateId),
    ExitedState(StateId),
    TriggeredTransition(TransitionId),
    BlockedTransition {
        transition_id: TransitionId,
        reason: TransitionBlockedReason,
    },
    PendingTransition(TransitionId),
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct StateMachineTraceEntry {
    pub frame_revision: u64,
    pub runtime_revision: u64,
    pub kind: TraceKind,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct StateMachineTrace {
    pub config: DebugTraceConfig,
    pub entries: Vec<StateMachineTraceEntry>,
}

impl StateMachineTrace {
    pub fn new(config: DebugTraceConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, entry: StateMachineTraceEntry) {
        let capacity = self.config.capacity;
        if capacity == 0 {
            return;
        }
        if self.entries.len() < capacity {
            self.entries.push(entry);
            return;
        }
        self.entries.rotate_left(1);
        if let Some(last) = self.entries.last_mut() {
            *last = entry;
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct AiDebugGizmos;

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct AiDebugCircle {
    pub radius: f32,
    pub color: Color,
    pub offset: Vec3,
}

impl AiDebugCircle {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            color: css::AQUA.into(),
            offset: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct AiDebugLine {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct AiDebugPath {
    pub points: Vec<Vec3>,
    pub color: Color,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AiDebugAnnotations {
    pub circles: Vec<AiDebugCircle>,
    pub lines: Vec<AiDebugLine>,
    pub paths: Vec<AiDebugPath>,
}

#[cfg(test)]
#[path = "debug_tests.rs"]
mod tests;
