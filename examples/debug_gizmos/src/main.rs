use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

fn main() {
    let mut app = common::base_app("ai_state_machine debug gizmos");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .add_systems(Startup, setup_machine);
    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("debug_gizmos");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let alert = builder.atomic_state("Alert");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(alert, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, alert)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        )
        .add_transition(
            TransitionDefinition::replace(alert, idle)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("DebugMachine"),
        StateMachineInstance::new(definition_id),
        AiDebugAnnotations {
            circles: vec![AiDebugCircle {
                radius: 3.0,
                color: Color::srgb(0.1, 0.8, 0.9),
                offset: Vec3::ZERO,
            }],
            lines: vec![AiDebugLine {
                start: Vec3::new(0.0, 0.2, 0.0),
                end: Vec3::new(2.0, 1.0, 0.0),
                color: Color::srgb(1.0, 0.6, 0.2),
            }],
            paths: vec![AiDebugPath {
                points: vec![
                    Vec3::new(-2.0, 0.05, -1.0),
                    Vec3::new(-1.0, 0.05, 1.0),
                    Vec3::new(1.0, 0.05, 1.5),
                ],
                color: Color::srgb(0.9, 0.2, 0.6),
            }],
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}
