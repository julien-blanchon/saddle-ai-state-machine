use super::*;
use bevy::prelude::AppTypeRegistry;
use bevy::reflect::serde::{ReflectDeserializer, ReflectSerializer};
use serde::de::DeserializeSeed;

#[test]
fn instance_uses_default_runtime_config() {
    let instance = StateMachineInstance::new(crate::definition::StateMachineDefinitionId(7));
    assert_eq!(
        instance.definition_id,
        crate::definition::StateMachineDefinitionId(7)
    );
    assert_eq!(instance.config.max_internal_steps, 16);
    assert_eq!(instance.stack.max_depth, 8);
    assert!(instance.config.blackboard_overrides.is_empty());
    assert!(matches!(instance.status, StateMachineStatus::Uninitialized));
}

#[test]
fn signal_helpers_are_deduplicated() {
    let mut instance = StateMachineInstance::new(crate::definition::StateMachineDefinitionId(11));
    let signal = crate::definition::SignalId(4);

    assert!(instance.queue_signal(signal));
    assert!(!instance.queue_signal(signal));
    assert!(instance.has_signal(signal));
    assert!(instance.clear_signal(signal));
    assert!(!instance.has_signal(signal));
}

#[test]
fn reflect_serialization_roundtrip() {
    let registry = AppTypeRegistry::default();
    registry.write().register::<StateMachineInstance>();
    registry.write().register::<StateMachineInstanceConfig>();
    registry.write().register::<StateStack>();
    registry.write().register::<crate::StateStackFrame>();
    registry.write().register::<ActiveRegionState>();
    registry.write().register::<HistorySnapshot>();
    registry.write().register::<PendingTransition>();
    registry
        .write()
        .register::<crate::definition::StateMachineDefinitionId>();
    registry.write().register::<crate::definition::StateId>();
    registry
        .write()
        .register::<crate::debug::TransitionBlockedReason>();
    registry
        .write()
        .register::<crate::debug::StateMachineTrace>();
    registry
        .write()
        .register::<crate::debug::StateMachineTraceEntry>();
    registry
        .write()
        .register::<crate::debug::DebugTraceConfig>();
    registry
        .write()
        .register::<crate::instance::InstanceBlackboardOverride>();
    registry
        .write()
        .register::<crate::blackboard::BlackboardKeyId>();
    registry
        .write()
        .register::<crate::blackboard::BlackboardValue>();

    let mut instance = StateMachineInstance::new(crate::definition::StateMachineDefinitionId(3));
    instance.active_leaf_states = vec![crate::definition::StateId(2)];
    instance.active_path = vec![crate::definition::StateId(1), crate::definition::StateId(2)];
    instance.pending_transition =
        Some(PendingTransition::Ready(crate::definition::TransitionId(4)));
    instance
        .config
        .blackboard_overrides
        .push(InstanceBlackboardOverride {
            key: crate::blackboard::BlackboardKeyId(1),
            value: crate::blackboard::BlackboardValue::Bool(true),
        });

    let registry = registry.read();
    let serializer = ReflectSerializer::new(&instance, &registry);
    let json = serde_json::to_value(&serializer).unwrap();
    let deserializer = ReflectDeserializer::new(&registry);
    let reflected = deserializer.deserialize(json).unwrap();
    let restored = StateMachineInstance::from_reflect(reflected.as_ref()).unwrap();

    assert_eq!(restored.definition_id, instance.definition_id);
    assert_eq!(restored.active_leaf_states, instance.active_leaf_states);
    assert_eq!(restored.pending_transition, instance.pending_transition);
    assert_eq!(
        restored.config.blackboard_overrides,
        instance.config.blackboard_overrides
    );
}
