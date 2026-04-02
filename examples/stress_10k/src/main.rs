use bevy::prelude::*;
use saddle_ai_state_machine::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machines)
        .add_systems(Update, report_once);
    app.run();
}

fn setup_machines(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("stress");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, run)
                .with_trigger(TransitionTrigger::after_seconds(0.5)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    for _ in 0..10_000 {
        commands.spawn((
            StateMachineInstance::new(definition_id),
            Blackboard::default(),
        ));
    }
}

fn report_once(query: Query<&StateMachineInstance>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    info!("Spawned {} state machine instances", query.iter().count());
}
