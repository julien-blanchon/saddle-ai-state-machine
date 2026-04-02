use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

fn main() {
    let mut app = common::base_app("ai_state_machine orthogonal regions");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("orthogonal");
    let root = builder.root_region("root");
    let controller = builder.parallel_state("Controller");
    let locomotion = builder.region("locomotion", controller);
    let action = builder.region("action", controller);
    let grounded = builder.atomic_state("Grounded");
    let jump = builder.atomic_state("Jump");
    let idle_action = builder.atomic_state("IdleAction");
    let attack = builder.atomic_state("Attack");
    builder
        .add_state_to_region(controller, root)
        .set_region_initial(root, controller)
        .add_state_to_region(grounded, locomotion)
        .add_state_to_region(jump, locomotion)
        .set_region_initial(locomotion, grounded)
        .add_state_to_region(idle_action, action)
        .add_state_to_region(attack, action)
        .set_region_initial(action, idle_action)
        .add_transition(
            TransitionDefinition::replace(grounded, jump)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        .add_transition(
            TransitionDefinition::replace(jump, grounded)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        .add_transition(
            TransitionDefinition::replace(idle_action, attack)
                .with_trigger(TransitionTrigger::after_seconds(1.5)),
        )
        .add_transition(
            TransitionDefinition::replace(attack, idle_action)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("OrthogonalMachine"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
