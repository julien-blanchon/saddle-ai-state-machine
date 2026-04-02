use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

fn main() {
    let mut app = common::base_app("ai_state_machine delayed transitions");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("delayed");
    let root = builder.root_region("root");
    let closed = builder.atomic_state("Closed");
    let opening = builder.atomic_state("Opening");
    let open = builder.atomic_state("Open");
    let closing = builder.atomic_state("Closing");
    builder
        .add_state_to_region(closed, root)
        .add_state_to_region(opening, root)
        .add_state_to_region(open, root)
        .add_state_to_region(closing, root)
        .set_region_initial(root, closed)
        .add_transition(
            TransitionDefinition::replace(closed, opening)
                .with_trigger(TransitionTrigger::after_seconds(0.6)),
        )
        .add_transition(
            TransitionDefinition::replace(opening, open)
                .with_trigger(TransitionTrigger::after_seconds(0.6)),
        )
        .add_transition(
            TransitionDefinition::replace(open, closing)
                .with_trigger(TransitionTrigger::after_seconds(0.8)),
        )
        .add_transition(
            TransitionDefinition::replace(closing, closed)
                .with_trigger(TransitionTrigger::after_seconds(0.6)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("DoorFlow"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
