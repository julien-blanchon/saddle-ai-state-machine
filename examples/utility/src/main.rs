use saddle_ai_state_machine_example_common as common;

use saddle_ai_state_machine::*;
use bevy::prelude::*;

const SCORE_GATHER: ScorerId = ScorerId(1);
const SCORE_FLEE: ScorerId = ScorerId(2);

#[derive(Resource, Default)]
struct UtilityClock(f32);

fn main() {
    let mut app = common::base_app("ai_state_machine utility");
    app.add_plugins(AiStateMachinePlugin::always_on(Update))
        .init_resource::<UtilityClock>()
        .add_systems(Startup, setup_machine)
        .add_systems(Update, drive_scores);

    {
        let mut callbacks = app.world_mut().resource_mut::<StateMachineCallbacks>();
        callbacks.register_scorer(SCORE_GATHER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("gather_score").unwrap())
                .unwrap()
                .unwrap_or(0.0)
        });
        callbacks.register_scorer(SCORE_FLEE, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("flee_score").unwrap())
                .unwrap()
                .unwrap_or(0.0)
        });
    }

    app.run();
}

fn setup_machine(mut commands: Commands, mut definitions: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("utility");
    builder.blackboard_key(
        "gather_score",
        BlackboardValueType::F32,
        false,
        Some(0.4_f32.into()),
    );
    builder.blackboard_key(
        "flee_score",
        BlackboardValueType::F32,
        false,
        Some(0.8_f32.into()),
    );
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let gather = builder.atomic_state("Gather");
    let flee = builder.atomic_state("Flee");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(gather, root)
        .add_state_to_region(flee, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, gather)
                .with_scorer(SCORE_GATHER, UtilityPolicy::BestScore),
        )
        .add_transition(
            TransitionDefinition::replace(idle, flee)
                .with_scorer(SCORE_FLEE, UtilityPolicy::BestScore),
        );
    let definition_id = definitions.register(builder.build().unwrap()).unwrap();

    commands.spawn((
        Name::new("UtilityMachine"),
        StateMachineInstance::new(definition_id),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}

fn drive_scores(
    time: Res<Time>,
    mut clock: ResMut<UtilityClock>,
    definitions: Res<StateMachineLibrary>,
    query: Query<(Entity, &StateMachineInstance)>,
    mut blackboards: Query<&mut Blackboard>,
) {
    clock.0 += time.delta_secs();
    for (entity, instance) in &query {
        let definition = definitions.definition(instance.definition_id).unwrap();
        let gather = definition.find_blackboard_key("gather_score").unwrap();
        let flee = definition.find_blackboard_key("flee_score").unwrap();
        let mut blackboard = blackboards.get_mut(entity).unwrap();
        let gather_value = 0.5 + 0.5 * clock.0.sin();
        let flee_value = 1.0 - gather_value;
        blackboard.set(gather, gather_value).unwrap();
        blackboard.set(flee, flee_value).unwrap();
    }
}
