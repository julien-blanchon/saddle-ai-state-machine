use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

fn main() {
    let mut app = common::base_app("ai_state_machine pushdown");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("pushdown");
    let root = builder.root_region("root");
    let patrol = builder.atomic_state("Patrol");
    let stunned = builder.atomic_state("Stunned");
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, patrol)
        .add_transition(
            TransitionDefinition::push(patrol, stunned)
                .with_trigger(TransitionTrigger::after_seconds(1.2)),
        )
        .add_transition(
            TransitionDefinition::pop(stunned).with_trigger(TransitionTrigger::after_seconds(0.8)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("PushdownMachine"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
