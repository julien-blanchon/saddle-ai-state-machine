use std::time::Duration;

use bevy::gizmos::GizmoPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::*;

#[derive(Resource, Default, Debug)]
struct ActionLog(Vec<String>);

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestDeactivateSchedule;

const GUARD_GO: GuardId = GuardId(1);
const GUARD_ALT: GuardId = GuardId(2);
const GUARD_ENTER: GuardId = GuardId(3);
const GUARD_EXIT: GuardId = GuardId(4);
const GUARD_INTERRUPT: GuardId = GuardId(5);
const GUARD_SWAP: GuardId = GuardId(6);
const SCORE_LOW: ScorerId = ScorerId(1);
const SCORE_HIGH: ScorerId = ScorerId(2);
const ACTION_EXIT_IDLE: ActionId = ActionId(1);
const ACTION_TRANSITION: ActionId = ActionId(2);
const ACTION_ENTER_MOVE: ActionId = ActionId(3);

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), GizmoPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )))
        .add_plugins(AiStateMachinePlugin::new(
            Startup,
            TestDeactivateSchedule,
            Update,
        ))
        .init_resource::<ActionLog>();

    {
        let mut callbacks = app.world_mut().resource_mut::<StateMachineCallbacks>();
        callbacks.register_guard(GUARD_GO, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("go").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_ALT, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("alt").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_ENTER, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("enter").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_EXIT, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("exit").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_INTERRUPT, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("interrupt").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_guard(GUARD_SWAP, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_bool(definition.find_blackboard_key("swap").unwrap())
                .unwrap()
                .unwrap_or(false)
        });
        callbacks.register_scorer(SCORE_LOW, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("low").unwrap())
                .unwrap()
                .unwrap_or(0.0)
        });
        callbacks.register_scorer(SCORE_HIGH, |_, _, definition, _, blackboard, _| {
            blackboard
                .get_f32(definition.find_blackboard_key("high").unwrap())
                .unwrap()
                .unwrap_or(0.0)
        });
        callbacks.register_action(ACTION_EXIT_IDLE, |world, _, _, _, _| {
            world.resource_mut::<ActionLog>().0.push("exit_idle".into());
        });
        callbacks.register_action(ACTION_TRANSITION, |world, _, _, _, _| {
            world
                .resource_mut::<ActionLog>()
                .0
                .push("transition".into());
        });
        callbacks.register_action(ACTION_ENTER_MOVE, |world, _, _, _, _| {
            world
                .resource_mut::<ActionLog>()
                .0
                .push("enter_move".into());
        });
    }
    app
}

fn register_definition(
    app: &mut App,
    definition: StateMachineDefinition,
) -> StateMachineDefinitionId {
    app.world_mut()
        .resource_mut::<StateMachineLibrary>()
        .register(definition)
        .unwrap()
}

fn spawn_machine(app: &mut App, definition_id: StateMachineDefinitionId) -> Entity {
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(definition_id)
        .unwrap()
        .clone();
    app.world_mut()
        .spawn((
            StateMachineInstance::new(definition_id),
            Blackboard::from_schema(&definition.blackboard_schema),
        ))
        .id()
}

fn run_updates(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

fn active_leaf_names(app: &App, entity: Entity) -> Vec<String> {
    let instance = app.world().get::<StateMachineInstance>(entity).unwrap();
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(instance.definition_id)
        .unwrap();
    instance
        .active_leaf_states
        .iter()
        .map(|state_id| definition.state(*state_id).unwrap().name.clone())
        .collect()
}

fn set_bool(app: &mut App, entity: Entity, key: &str, value: bool) {
    let definition_id = app
        .world()
        .get::<StateMachineInstance>(entity)
        .unwrap()
        .definition_id;
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(definition_id)
        .unwrap()
        .clone();
    let key = definition.find_blackboard_key(key).unwrap();
    app.world_mut()
        .get_mut::<Blackboard>(entity)
        .unwrap()
        .set(key, value)
        .unwrap();
}

fn set_f32(app: &mut App, entity: Entity, key: &str, value: f32) {
    let definition_id = app
        .world()
        .get::<StateMachineInstance>(entity)
        .unwrap()
        .definition_id;
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(definition_id)
        .unwrap()
        .clone();
    let key = definition.find_blackboard_key(key).unwrap();
    app.world_mut()
        .get_mut::<Blackboard>(entity)
        .unwrap()
        .set(key, value)
        .unwrap();
}

fn simple_builder(name: &str) -> (StateMachineBuilder, RegionId) {
    let mut builder = StateMachineBuilder::new(name);
    for key in ["go", "alt", "enter", "exit", "interrupt", "swap"] {
        builder.blackboard_key(key, BlackboardValueType::Bool, false, Some(false.into()));
    }
    builder.blackboard_key("low", BlackboardValueType::F32, false, Some(0.0_f32.into()));
    builder.blackboard_key(
        "high",
        BlackboardValueType::F32,
        false,
        Some(0.0_f32.into()),
    );
    let root = builder.root_region("root");
    (builder, root)
}

#[test]
fn basic_transition() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("basic_transition");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, run).with_guard(GUARD_GO));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Idle"]);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Run"]);
}

#[test]
fn transition_priority() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("transition_priority");
    let idle = builder.atomic_state("Idle");
    let low = builder.atomic_state("Low");
    let high = builder.atomic_state("High");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(low, root)
        .add_state_to_region(high, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, low)
                .with_guard(GUARD_GO)
                .with_priority(1),
        )
        .add_transition(
            TransitionDefinition::replace(idle, high)
                .with_guard(GUARD_ALT)
                .with_priority(10),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["High"]);
}

#[test]
fn on_change_evaluation_mode_sleeps_until_input_changes() {
    let mut app = test_app();

    let (mut builder, root) = simple_builder("event_driven");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, run).with_guard(GUARD_GO));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let schema = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(def_id)
        .unwrap()
        .blackboard_schema
        .clone();
    let entity = app
        .world_mut()
        .spawn((
            StateMachineInstance::new(def_id).with_config(StateMachineInstanceConfig {
                evaluation_mode: StateMachineEvaluationMode::OnSignalOrBlackboardChange,
                ..default()
            }),
            Blackboard::from_schema(&schema),
        ))
        .id();

    run_updates(&mut app, 1);
    let initial_revision = app
        .world()
        .get::<StateMachineInstance>(entity)
        .unwrap()
        .last_blackboard_revision;

    run_updates(&mut app, 1);
    let second_revision = app
        .world()
        .get::<StateMachineInstance>(entity)
        .unwrap()
        .last_blackboard_revision;
    assert_eq!(second_revision, initial_revision);
    assert_eq!(active_leaf_names(&app, entity), vec!["Idle"]);

    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);

    let final_revision = app
        .world()
        .get::<StateMachineInstance>(entity)
        .unwrap()
        .last_blackboard_revision;
    assert!(final_revision > second_revision);
    assert_eq!(active_leaf_names(&app, entity), vec!["Run"]);
}

#[test]
fn transition_tiebreak_declaration_order() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("declaration_order");
    let idle = builder.atomic_state("Idle");
    let first = builder.atomic_state("First");
    let second = builder.atomic_state("Second");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(first, root)
        .add_state_to_region(second, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, first).with_guard(GUARD_GO))
        .add_transition(TransitionDefinition::replace(idle, second).with_guard(GUARD_GO));
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["First"]);
}

#[test]
fn deepest_child_checked_first() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("deepest_child");
    let combat = builder.compound_state("Combat");
    let combat_region = builder.region("combat_region", combat);
    let windup = builder.atomic_state("Windup");
    let strike = builder.atomic_state("Strike");
    let recover = builder.atomic_state("Recover");

    builder
        .add_state_to_region(combat, root)
        .add_state_to_region(recover, root)
        .set_region_initial(root, combat)
        .add_state_to_region(windup, combat_region)
        .add_state_to_region(strike, combat_region)
        .set_region_initial(combat_region, windup)
        .add_transition(TransitionDefinition::replace(combat, recover).with_guard(GUARD_ALT))
        .add_transition(TransitionDefinition::replace(windup, strike).with_guard(GUARD_GO));
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Strike"]);
}

#[test]
fn any_state_transition() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("any_state");
    let idle = builder.atomic_state("Idle");
    let disabled = builder.atomic_state("Disabled");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(disabled, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(TransitionSource::AnyState, disabled)
                .with_guard(GUARD_ALT),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Disabled"]);
}

#[test]
fn enter_exit_action_order() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("action_order");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_on_exit(idle, ACTION_EXIT_IDLE)
        .add_on_enter(run, ACTION_ENTER_MOVE)
        .add_transition(
            TransitionDefinition::replace(idle, run)
                .with_guard(GUARD_GO)
                .with_action(ACTION_TRANSITION),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    app.world_mut().resource_mut::<ActionLog>().0.clear();
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(
        app.world().resource::<ActionLog>().0,
        vec!["exit_idle", "transition", "enter_move"]
    );
}

#[test]
fn delayed_transition_fires() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("delayed");
    let idle = builder.atomic_state("Idle");
    let timed = builder.atomic_state("Timed");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(timed, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, timed)
                .with_trigger(TransitionTrigger::after_seconds(0.2)),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Idle"]);
    run_updates(&mut app, 2);
    assert_eq!(active_leaf_names(&app, entity), vec!["Timed"]);
}

#[test]
fn delayed_transition_cancels_on_exit() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("delay_cancel");
    let idle = builder.atomic_state("Idle");
    let timed = builder.atomic_state("Timed");
    let override_state = builder.atomic_state("Override");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(timed, root)
        .add_state_to_region(override_state, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, timed)
                .with_trigger(TransitionTrigger::after_seconds(0.4)),
        )
        .add_transition(
            TransitionDefinition::replace(TransitionSource::AnyState, override_state)
                .with_guard(GUARD_ALT)
                .with_priority(10),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 1);
    run_updates(&mut app, 5);
    assert_eq!(active_leaf_names(&app, entity), vec!["Override"]);
}

#[test]
fn push_then_pop_resumes() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("push_pop");
    let patrol = builder.atomic_state("Patrol");
    let stunned = builder.atomic_state("Stunned");
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, patrol)
        .add_transition(TransitionDefinition::push(patrol, stunned).with_guard(GUARD_INTERRUPT))
        .add_transition(
            TransitionDefinition::pop(stunned).with_trigger(TransitionTrigger::after_seconds(0.2)),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    set_bool(&mut app, entity, "interrupt", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Stunned"]);
    set_bool(&mut app, entity, "interrupt", false);
    run_updates(&mut app, 2);
    assert_eq!(active_leaf_names(&app, entity), vec!["Patrol"]);
}

#[test]
fn shallow_history_restores_last_child() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("shallow_history");
    let idle = builder.atomic_state("Idle");
    let combat = builder.compound_state("Combat");
    let combat_region = builder.region("combat_region", combat);
    let melee = builder.atomic_state("Melee");
    let ranged = builder.atomic_state("Ranged");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(combat, root)
        .set_region_initial(root, idle)
        .add_state_to_region(melee, combat_region)
        .add_state_to_region(ranged, combat_region)
        .set_region_initial(combat_region, melee)
        .set_state_history_mode(combat, HistoryMode::Shallow)
        .add_transition(TransitionDefinition::replace(idle, combat).with_guard(GUARD_ENTER))
        .add_transition(TransitionDefinition::replace(melee, ranged).with_guard(GUARD_SWAP))
        .add_transition(
            TransitionDefinition::replace(TransitionSource::AnyState, idle)
                .with_guard(GUARD_EXIT)
                .with_priority(10),
        );

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "enter", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Melee"]);
    set_bool(&mut app, entity, "swap", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Ranged"]);
    set_bool(&mut app, entity, "swap", false);
    set_bool(&mut app, entity, "enter", false);
    set_bool(&mut app, entity, "exit", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Idle"]);
    set_bool(&mut app, entity, "exit", false);
    set_bool(&mut app, entity, "enter", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Ranged"]);
}

#[test]
fn deep_history_restores_subtree() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("deep_history");
    let idle = builder.atomic_state("Idle");
    let combat = builder.compound_state("Combat");
    let combat_region = builder.region("combat_region", combat);
    let phase = builder.compound_state("Phase");
    let phase_region = builder.region("phase_region", phase);
    let a = builder.atomic_state("A");
    let b = builder.atomic_state("B");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(combat, root)
        .set_region_initial(root, idle)
        .add_state_to_region(phase, combat_region)
        .set_region_initial(combat_region, phase)
        .add_state_to_region(a, phase_region)
        .add_state_to_region(b, phase_region)
        .set_region_initial(phase_region, a)
        .set_state_history_mode(combat, HistoryMode::Deep)
        .add_transition(TransitionDefinition::replace(idle, combat).with_guard(GUARD_ENTER))
        .add_transition(TransitionDefinition::replace(a, b).with_guard(GUARD_SWAP))
        .add_transition(
            TransitionDefinition::replace(TransitionSource::AnyState, idle)
                .with_guard(GUARD_EXIT)
                .with_priority(10),
        );

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "enter", true);
    run_updates(&mut app, 1);
    set_bool(&mut app, entity, "swap", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["B"]);
    set_bool(&mut app, entity, "swap", false);
    set_bool(&mut app, entity, "enter", false);
    set_bool(&mut app, entity, "exit", true);
    run_updates(&mut app, 1);
    set_bool(&mut app, entity, "exit", false);
    set_bool(&mut app, entity, "enter", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["B"]);
}

#[test]
fn orthogonal_regions_activate_together() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("parallel_activate");
    let controller = builder.parallel_state("Controller");
    let locomotion = builder.region("locomotion", controller);
    let action = builder.region("action", controller);
    let grounded = builder.atomic_state("Grounded");
    let idle_action = builder.atomic_state("IdleAction");
    builder
        .add_state_to_region(controller, root)
        .set_region_initial(root, controller)
        .add_state_to_region(grounded, locomotion)
        .set_region_initial(locomotion, grounded)
        .add_state_to_region(idle_action, action)
        .set_region_initial(action, idle_action);
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    assert_eq!(
        active_leaf_names(&app, entity),
        vec!["Grounded", "IdleAction"]
    );
}

#[test]
fn orthogonal_region_done() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("parallel_done");
    let controller = builder.parallel_state("Controller");
    let complete = builder.atomic_state("Complete");
    let task = builder.region("task", controller);
    let alert = builder.region("alert", controller);
    let a = builder.atomic_state("TaskA");
    let a_done = builder.final_state("TaskDone");
    let b = builder.atomic_state("AlertA");
    let b_done = builder.final_state("AlertDone");
    builder
        .add_state_to_region(controller, root)
        .add_state_to_region(complete, root)
        .set_region_initial(root, controller)
        .add_state_to_region(a, task)
        .add_state_to_region(a_done, task)
        .set_region_initial(task, a)
        .add_state_to_region(b, alert)
        .add_state_to_region(b_done, alert)
        .set_region_initial(alert, b)
        .add_transition(TransitionDefinition::replace(a, a_done).with_guard(GUARD_GO))
        .add_transition(TransitionDefinition::replace(b, b_done).with_guard(GUARD_ALT))
        .add_transition(
            TransitionDefinition::replace(controller, complete)
                .with_trigger(TransitionTrigger::Done),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 3);
    assert_eq!(active_leaf_names(&app, entity), vec!["Complete"]);
}

#[test]
fn utility_best_score_wins() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("utility_best");
    let idle = builder.atomic_state("Idle");
    let low = builder.atomic_state("Low");
    let high = builder.atomic_state("High");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(low, root)
        .add_state_to_region(high, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, low)
                .with_scorer(SCORE_LOW, UtilityPolicy::BestScore),
        )
        .add_transition(
            TransitionDefinition::replace(idle, high)
                .with_scorer(SCORE_HIGH, UtilityPolicy::BestScore),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_f32(&mut app, entity, "low", 0.2);
    set_f32(&mut app, entity, "high", 0.8);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["High"]);
}

#[test]
fn utility_threshold_blocks() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("utility_threshold");
    let idle = builder.atomic_state("Idle");
    let high = builder.atomic_state("High");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(high, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, high)
                .with_scorer(SCORE_HIGH, UtilityPolicy::best_score_above(0.5)),
        );
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_f32(&mut app, entity, "high", 0.2);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Idle"]);
}

#[test]
fn transition_cooldown_blocks_refire_until_elapsed() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("cooldown");
    let a = builder.atomic_state("A");
    let b = builder.atomic_state("B");
    builder
        .add_state_to_region(a, root)
        .add_state_to_region(b, root)
        .set_region_initial(root, a)
        .add_transition(
            TransitionDefinition::replace(a, b)
                .with_guard(GUARD_GO)
                .with_cooldown(0.3),
        )
        .add_transition(TransitionDefinition::replace(b, a).with_guard(GUARD_ALT));
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["B"]);
    set_bool(&mut app, entity, "go", false);
    set_bool(&mut app, entity, "alt", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["A"]);
    set_bool(&mut app, entity, "alt", false);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["A"]);
    run_updates(&mut app, 3);
    assert_eq!(active_leaf_names(&app, entity), vec!["B"]);
}

#[test]
fn transient_chain_terminates() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("transient_chain");
    let idle = builder.atomic_state("Idle");
    let handoff = builder.transient_state("Handoff");
    let final_state = builder.atomic_state("Final");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(handoff, root)
        .add_state_to_region(final_state, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, handoff).with_guard(GUARD_GO))
        .add_transition(TransitionDefinition::replace(handoff, final_state));
    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Final"]);
}

#[test]
fn transient_trace_records_each_state_once() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("transient_trace");
    let idle = builder.atomic_state("Idle");
    let handoff = builder.transient_state("Handoff");
    let final_state = builder.atomic_state("Final");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(handoff, root)
        .add_state_to_region(final_state, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, handoff).with_guard(GUARD_GO))
        .add_transition(TransitionDefinition::replace(handoff, final_state));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    set_bool(&mut app, entity, "go", true);
    run_updates(&mut app, 1);

    let instance = app.world().get::<StateMachineInstance>(entity).unwrap();
    let entered_handoff = instance
        .trace
        .entries
        .iter()
        .filter(|entry| matches!(entry.kind, TraceKind::EnteredState(state) if state == handoff))
        .count();
    let exited_handoff = instance
        .trace
        .entries
        .iter()
        .filter(|entry| matches!(entry.kind, TraceKind::ExitedState(state) if state == handoff))
        .count();

    assert_eq!(entered_handoff, 1);
    assert_eq!(exited_handoff, 1);
    assert_eq!(active_leaf_names(&app, entity), vec!["Final"]);
}

#[test]
fn parallel_parent_timer_ticks_once_per_frame() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("parallel_timer");
    let controller = builder.parallel_state("Controller");
    let locomotion = builder.region("locomotion", controller);
    let action = builder.region("action", controller);
    let grounded = builder.atomic_state("Grounded");
    let idle_action = builder.atomic_state("IdleAction");
    builder
        .add_state_to_region(controller, root)
        .set_region_initial(root, controller)
        .add_state_to_region(grounded, locomotion)
        .set_region_initial(locomotion, grounded)
        .add_state_to_region(idle_action, action)
        .set_region_initial(action, idle_action);

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 2);

    let instance = app.world().get::<StateMachineInstance>(entity).unwrap();
    assert_eq!(instance.state_elapsed_seconds[controller.0 as usize], 0.1);
}

#[test]
fn push_transition_blocked_when_stack_is_full() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("push_blocked");
    let patrol = builder.atomic_state("Patrol");
    let stunned = builder.atomic_state("Stunned");
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(stunned, root)
        .set_region_initial(root, patrol)
        .add_transition(TransitionDefinition::push(patrol, stunned).with_guard(GUARD_INTERRUPT));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(def_id)
        .unwrap()
        .clone();
    let entity = app
        .world_mut()
        .spawn((
            StateMachineInstance::new(def_id).with_config(StateMachineInstanceConfig {
                max_stack_depth: 0,
                ..default()
            }),
            Blackboard::from_schema(&definition.blackboard_schema),
        ))
        .id();

    run_updates(&mut app, 1);
    set_bool(&mut app, entity, "interrupt", true);
    run_updates(&mut app, 1);

    let blocked_messages = app.world().resource::<Messages<TransitionBlocked>>();
    let mut cursor = blocked_messages.get_cursor();
    let messages: Vec<_> = cursor.read(blocked_messages).cloned().collect();
    assert!(messages.iter().any(|message| {
        message.entity == entity && message.reason == TransitionBlockedReason::StackOverflow
    }));
    assert_eq!(active_leaf_names(&app, entity), vec!["Patrol"]);
}

#[test]
fn signal_transition_consumes_queued_signal() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("signal_transition");
    let patrol = builder.atomic_state("Patrol");
    let alert = builder.atomic_state("Alert");
    let signal = SignalId(1);
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(alert, root)
        .set_region_initial(root, patrol)
        .add_transition(TransitionDefinition::replace(patrol, alert).with_signal(signal));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    app.world_mut()
        .get_mut::<StateMachineInstance>(entity)
        .unwrap()
        .queue_signal(signal);
    run_updates(&mut app, 1);

    let instance = app.world().get::<StateMachineInstance>(entity).unwrap();
    assert_eq!(active_leaf_names(&app, entity), vec!["Alert"]);
    assert!(!instance.has_signal(signal));
}

#[test]
fn signal_message_queues_signal_during_intake() {
    let mut app = test_app();
    let (mut builder, root) = simple_builder("signal_message");
    let patrol = builder.atomic_state("Patrol");
    let alert = builder.atomic_state("Alert");
    let signal = SignalId(9);
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(alert, root)
        .set_region_initial(root, patrol)
        .add_transition(TransitionDefinition::replace(patrol, alert).with_signal(signal));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let entity = spawn_machine(&mut app, def_id);
    run_updates(&mut app, 1);
    app.world_mut()
        .resource_mut::<Messages<StateMachineSignal>>()
        .write(StateMachineSignal::new(entity, signal));
    run_updates(&mut app, 1);

    let instance = app.world().get::<StateMachineInstance>(entity).unwrap();
    assert_eq!(active_leaf_names(&app, entity), vec!["Alert"]);
    assert!(!instance.has_signal(signal));
}

#[test]
fn blackboard_overrides_apply_before_first_transition_evaluation() {
    let mut app = test_app();
    let mut builder = StateMachineBuilder::new("blackboard_override");
    let go = builder.blackboard_key("go", BlackboardValueType::Bool, false, Some(false.into()));
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, run).with_guard(GUARD_GO));

    let def_id = register_definition(&mut app, builder.build().unwrap());
    let definition = app
        .world()
        .resource::<StateMachineLibrary>()
        .definition(def_id)
        .unwrap()
        .clone();
    let entity = app
        .world_mut()
        .spawn((
            StateMachineInstance::new(def_id).with_config(StateMachineInstanceConfig {
                blackboard_overrides: vec![InstanceBlackboardOverride {
                    key: go,
                    value: BlackboardValue::Bool(true),
                }],
                ..default()
            }),
            Blackboard::from_schema(&definition.blackboard_schema),
        ))
        .id();

    run_updates(&mut app, 1);

    let blackboard = app.world().get::<Blackboard>(entity).unwrap();
    assert_eq!(blackboard.get_bool(go).unwrap(), Some(true));
    assert_eq!(active_leaf_names(&app, entity), vec!["Run"]);
}
