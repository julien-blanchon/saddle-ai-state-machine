use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

const GUARD_ENTER: GuardId = GuardId(1);

fn main() {
    let mut app = common::base_app("ai_state_machine hierarchical");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.world_mut()
        .resource_mut::<StateMachineCallbacks>()
        .register_guard(GUARD_ENTER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("enter").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("hierarchical");
    builder.blackboard_key("enter", BlackboardValueType::Bool, false, Some(true.into()));
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let combat = builder.compound_state("Combat");
    let combat_region = builder.region("combat_region", combat);
    let windup = builder.atomic_state("Windup");
    let strike = builder.atomic_state("Strike");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(combat, root)
        .set_region_initial(root, idle)
        .add_state_to_region(windup, combat_region)
        .add_state_to_region(strike, combat_region)
        .set_region_initial(combat_region, windup)
        .add_transition(TransitionDefinition::replace(idle, combat).with_guard(GUARD_ENTER))
        .add_transition(
            TransitionDefinition::replace(windup, strike)
                .with_trigger(TransitionTrigger::after_seconds(0.8)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("HierarchicalMachine"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
