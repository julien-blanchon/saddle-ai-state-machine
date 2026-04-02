use saddle_ai_state_machine_example_common as common;

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy::reflect::serde::{ReflectDeserializer, TypedReflectSerializer};
use saddle_ai_state_machine::*;
use serde::de::DeserializeSeed;

#[derive(Resource, Default)]
struct SaveTimer {
    elapsed: f32,
    saved: bool,
}

fn main() {
    let mut app = common::base_app("ai_state_machine save/load");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .init_resource::<SaveTimer>()
        .add_systems(Startup, setup_machine)
        .add_systems(Update, save_snapshot_once);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("save_load");
    let mood = builder.blackboard_key(
        "mood",
        BlackboardValueType::String,
        false,
        Some("idle".into()),
    );
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, run)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );
    let definition = builder.build().unwrap();
    let definition_id = definitions.register(definition.clone()).unwrap();

    commands.spawn((
        Name::new("SerializableMachine"),
        StateMachineInstance::new(definition_id).with_config(StateMachineInstanceConfig {
            blackboard_overrides: vec![InstanceBlackboardOverride {
                key: mood,
                value: BlackboardValue::String("persisted".to_string()),
            }],
            trace_config: DebugTraceConfig {
                capacity: 8,
                record_blocked: true,
            },
            ..default()
        }),
        Blackboard::from_schema(&definition.blackboard_schema),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}

fn save_snapshot_once(
    time: Res<Time>,
    mut timer: ResMut<SaveTimer>,
    registry: Res<AppTypeRegistry>,
    query: Query<(&StateMachineInstance, &Blackboard)>,
) {
    if timer.saved {
        return;
    }
    timer.elapsed += time.delta_secs();
    if timer.elapsed < 1.5 {
        return;
    }

    let Some((instance, blackboard)) = query.iter().next() else {
        return;
    };

    let registry = registry.read();
    let instance_json =
        serde_json::to_string_pretty(&TypedReflectSerializer::new(instance, &registry)).unwrap();
    let blackboard_json =
        serde_json::to_string_pretty(&TypedReflectSerializer::new(blackboard, &registry)).unwrap();

    let restored_instance: StateMachineInstance = deserialize_reflect(&registry, &instance_json);
    let restored_blackboard: Blackboard = deserialize_reflect(&registry, &blackboard_json);

    assert_eq!(
        restored_instance.active_leaf_states,
        instance.active_leaf_states
    );
    assert_eq!(restored_instance.history, instance.history);
    assert_eq!(restored_instance.stack, instance.stack);
    assert_eq!(restored_instance.trace, instance.trace);
    assert_eq!(restored_blackboard, *blackboard);

    info!("Instance round-trip verified:\n{instance_json}");
    info!("Blackboard round-trip verified:\n{blackboard_json}");
    timer.saved = true;
}

fn deserialize_reflect<T: FromReflect>(registry: &TypeRegistry, json: &str) -> T {
    let deserializer = ReflectDeserializer::new(registry);
    let mut json_deserializer = serde_json::Deserializer::from_str(json);
    let reflected = deserializer.deserialize(&mut json_deserializer).unwrap();
    T::from_reflect(reflected.as_ref()).unwrap()
}
