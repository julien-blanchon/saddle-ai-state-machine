use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

fn main() {
    let mut app = common::base_app("ai_state_machine basic");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("basic");
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
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("BasicMachine"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
