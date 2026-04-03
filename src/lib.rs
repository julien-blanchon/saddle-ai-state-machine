use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

pub mod assets;
pub mod blackboard;
pub mod builder;
pub mod debug;
pub mod definition;
pub mod hierarchy;
pub mod instance;
pub mod messages;
pub mod regions;
pub mod stack;
pub mod systems;
pub mod timers;
pub mod transitions;
pub mod validation;

pub use assets::{
    StateMachineDefinitionAsset, StateMachineDefinitionAssetLoader,
    StateMachineDefinitionAssetLoaderError,
};
pub use blackboard::{
    Blackboard, BlackboardError, BlackboardKeyDefinition, BlackboardKeyId, BlackboardValue,
    BlackboardValueType,
};
pub use builder::StateMachineBuilder;
pub use debug::{
    AiDebugAnnotations, AiDebugCircle, AiDebugGizmos, AiDebugLine, AiDebugPath, DebugTraceConfig,
    StateMachineTrace, StateMachineTraceEntry, TraceKind, TransitionBlockedReason,
};
pub use definition::{
    ActionId, GuardId, HistoryMode, RegionDefinition, RegionId, ScorerId, SignalId,
    StateDefinition, StateId, StateKind, StateMachineDefinition, StateMachineDefinitionId,
    StateMachineLibrary, TransitionDefinition, TransitionId, TransitionMode, TransitionOperation,
    TransitionSource, TransitionTrigger, UtilityPolicy,
};
pub use instance::{
    InstanceBlackboardOverride, InstanceThresholdOverride, PendingTransition,
    StateMachineEvaluationMode, StateMachineInstance, StateMachineInstanceConfig,
    StateMachineStatus,
};
pub use messages::{
    StateEntered, StateExited, StateMachineSignal, TransitionBlocked, TransitionTriggered,
};
pub use stack::{ActiveRegionState, HistorySnapshot, StateStack, StateStackFrame};
pub use transitions::StateMachineCallbacks;
pub use validation::{ValidationIssue, ValidationReport, ValidationSeverity};

/// Public system ordering for the runtime pipeline.
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum AiStateMachineSystems {
    IntakeSignals,
    AdvanceTimers,
    EvaluateTransitions,
    ExecuteTransitions,
    UpdateStates,
    DebugVisualize,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

/// Sandbox-friendly plugin.
pub struct AiStateMachinePlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl AiStateMachinePlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    /// Convenience constructor for apps where machines should stay live for the app lifetime.
    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Plugin for AiStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StateMachineLibrary>()
            .init_resource::<StateMachineCallbacks>()
            .init_gizmo_group::<AiDebugGizmos>()
            .init_asset::<StateMachineDefinitionAsset>()
            .register_asset_loader(StateMachineDefinitionAssetLoader)
            .add_message::<StateMachineSignal>()
            .add_message::<StateEntered>()
            .add_message::<StateExited>()
            .add_message::<TransitionTriggered>()
            .add_message::<TransitionBlocked>()
            .register_type::<ActiveRegionState>()
            .register_type::<AiDebugAnnotations>()
            .register_type::<AiDebugCircle>()
            .register_type::<AiDebugLine>()
            .register_type::<AiDebugPath>()
            .register_type::<Blackboard>()
            .register_type::<BlackboardKeyDefinition>()
            .register_type::<BlackboardKeyId>()
            .register_type::<BlackboardValue>()
            .register_type::<BlackboardValueType>()
            .register_type::<ActionId>()
            .register_type::<DebugTraceConfig>()
            .register_type::<GuardId>()
            .register_type::<HistoryMode>()
            .register_type::<HistorySnapshot>()
            .register_type::<InstanceBlackboardOverride>()
            .register_type::<InstanceThresholdOverride>()
            .register_type::<PendingTransition>()
            .register_type::<RegionDefinition>()
            .register_type::<RegionId>()
            .register_type::<ScorerId>()
            .register_type::<SignalId>()
            .register_type::<StateDefinition>()
            .register_type::<StateEntered>()
            .register_type::<StateExited>()
            .register_type::<StateMachineDefinitionAsset>()
            .register_type::<StateMachineSignal>()
            .register_type::<StateId>()
            .register_type::<StateKind>()
            .register_type::<StateMachineDefinition>()
            .register_type::<StateMachineDefinitionId>()
            .register_type::<StateMachineEvaluationMode>()
            .register_type::<StateMachineInstance>()
            .register_type::<StateMachineInstanceConfig>()
            .register_type::<StateMachineLibrary>()
            .register_type::<StateMachineStatus>()
            .register_type::<StateMachineTrace>()
            .register_type::<StateMachineTraceEntry>()
            .register_type::<StateStack>()
            .register_type::<StateStackFrame>()
            .register_type::<TransitionBlocked>()
            .register_type::<TransitionBlockedReason>()
            .register_type::<TransitionDefinition>()
            .register_type::<TransitionId>()
            .register_type::<TransitionMode>()
            .register_type::<TransitionOperation>()
            .register_type::<TransitionSource>()
            .register_type::<TransitionTrigger>()
            .register_type::<TransitionTriggered>()
            .register_type::<TraceKind>()
            .register_type::<UtilityPolicy>();

        app.add_systems(self.activate_schedule, systems::activate_instances);
        app.add_systems(self.deactivate_schedule, systems::deactivate_instances);
        app.add_systems(
            self.update_schedule,
            (
                systems::intake_signals.in_set(AiStateMachineSystems::IntakeSignals),
                systems::advance_timers.in_set(AiStateMachineSystems::AdvanceTimers),
                systems::evaluate_transitions.in_set(AiStateMachineSystems::EvaluateTransitions),
                systems::execute_transitions.in_set(AiStateMachineSystems::ExecuteTransitions),
                systems::update_states.in_set(AiStateMachineSystems::UpdateStates),
                systems::debug_visualize.in_set(AiStateMachineSystems::DebugVisualize),
            )
                .chain(),
        );
    }
}
