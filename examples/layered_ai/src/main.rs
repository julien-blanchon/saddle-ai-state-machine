use bevy::prelude::*;
use saddle_ai_behavior_tree as bt;
use saddle_ai_goap as goap;
use saddle_ai_state_machine as fsm;
use saddle_ai_utility_ai as uai;
use saddle_pane::prelude::*;

const FSM_VISIBLE: fsm::GuardId = fsm::GuardId(1);
const FSM_HIDDEN: fsm::GuardId = fsm::GuardId(2);
const FSM_PANIC: fsm::GuardId = fsm::GuardId(3);
const FSM_CALM: fsm::GuardId = fsm::GuardId(4);

#[derive(Resource, Clone, Pane)]
#[pane(title = "Layered AI Sandbox")]
struct LayeredAiPane {
    #[pane(slider, min = 0.1, max = 2.5, step = 0.05)]
    time_scale: f32,
    #[pane(slider, min = 120.0, max = 360.0, step = 5.0)]
    target_radius: f32,
    #[pane(slider, min = 80.0, max = 320.0, step = 5.0)]
    alert_distance: f32,
    #[pane(slider, min = 0.0, max = 1.0, step = 0.05)]
    retreat_bias: f32,
    workbench_open: bool,
}

impl Default for LayeredAiPane {
    fn default() -> Self {
        Self {
            time_scale: 1.0,
            target_radius: 240.0,
            alert_distance: 210.0,
            retreat_bias: 0.25,
            workbench_open: true,
        }
    }
}

#[derive(Component)]
struct Guard;

#[derive(Component)]
struct OrbitingTarget;

#[derive(Component)]
struct Worker;

#[derive(Component)]
struct Overlay;

#[derive(Component)]
struct AttackOpportunity;

#[derive(Component)]
struct RetreatPressure;

#[derive(Component, Clone, Copy, Default)]
struct WorkerInventory {
    has_ore: bool,
    delivered: bool,
}

#[derive(Resource, Clone, Copy)]
struct LayeredEntities {
    guard: Entity,
    target: Entity,
    worker: Entity,
}

#[derive(Resource, Clone, Copy)]
struct GuardStateMachineKeys {
    target_visible: fsm::BlackboardKeyId,
    panic: fsm::BlackboardKeyId,
}

#[derive(Resource, Clone, Copy)]
struct GuardBehaviorTreeKeys {
    visible: bt::BlackboardKeyId,
}

#[derive(Resource, Default)]
struct WorkerLoopState {
    delivered_at_seconds: Option<f32>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.03, 0.035, 0.05)));
    app.insert_resource(uai::UtilityAiBudget {
        max_agents_per_update: 32,
    });
    app.init_resource::<LayeredAiPane>();
    app.init_resource::<WorkerLoopState>();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "layered ai sandbox".into(),
            resolution: (1500, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
        fsm::AiStateMachinePlugin::always_on(Update),
        bt::BehaviorTreePlugin::always_on(Update),
        uai::UtilityAiPlugin::always_on(Update),
        goap::GoapPlugin::always_on(Update),
    ));
    app.register_pane::<LayeredAiPane>();
    register_state_machine_callbacks(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            sync_pane_runtime,
            animate_target,
            sync_guard_inputs,
            sync_utility_to_state_machine,
            process_worker_actions,
            reset_worker_cycle,
            tint_actors,
            update_overlay,
        ),
    );
    app.run();
}

fn register_state_machine_callbacks(app: &mut App) {
    let mut callbacks = app.world_mut().resource_mut::<fsm::StateMachineCallbacks>();
    callbacks.register_guard(FSM_VISIBLE, |_, _, definition, _, blackboard, _| {
        blackboard
            .get_bool(definition.find_blackboard_key("target_visible").unwrap())
            .unwrap()
            .unwrap_or(false)
    });
    callbacks.register_guard(FSM_HIDDEN, |_, _, definition, _, blackboard, _| {
        !blackboard
            .get_bool(definition.find_blackboard_key("target_visible").unwrap())
            .unwrap()
            .unwrap_or(false)
    });
    callbacks.register_guard(FSM_PANIC, |_, _, definition, _, blackboard, _| {
        blackboard
            .get_bool(definition.find_blackboard_key("panic").unwrap())
            .unwrap()
            .unwrap_or(false)
    });
    callbacks.register_guard(FSM_CALM, |_, _, definition, _, blackboard, _| {
        !blackboard
            .get_bool(definition.find_blackboard_key("panic").unwrap())
            .unwrap()
            .unwrap_or(false)
    });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state_machines: ResMut<fsm::StateMachineLibrary>,
    mut trees: ResMut<bt::BehaviorTreeLibrary>,
    mut tree_handlers: ResMut<bt::BehaviorTreeHandlers>,
    mut goap_library: ResMut<goap::GoapLibrary>,
    mut goap_hooks: ResMut<goap::GoapHooks>,
) {
    commands.spawn((Name::new("Camera"), Camera2d));
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.05, 0.06, 0.09), Vec2::new(1800.0, 1000.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));
    commands.spawn((
        Name::new("Guard Lane"),
        Sprite::from_color(
            Color::srgba(0.15, 0.20, 0.28, 0.95),
            Vec2::new(1360.0, 280.0),
        ),
        Transform::from_xyz(0.0, 185.0, -10.0),
    ));
    commands.spawn((
        Name::new("Worker Lane"),
        Sprite::from_color(
            Color::srgba(0.11, 0.18, 0.16, 0.95),
            Vec2::new(1360.0, 240.0),
        ),
        Transform::from_xyz(0.0, -170.0, -10.0),
    ));
    commands.spawn((
        Name::new("HUD Card"),
        Sprite::from_color(
            Color::srgba(0.01, 0.02, 0.03, 0.88),
            Vec2::new(430.0, 780.0),
        ),
        Transform::from_xyz(495.0, 0.0, -8.0),
    ));
    for (name, label, x, y) in [
        ("Guard Label", "Guard: FSM + BT + Utility AI", -620.0, 300.0),
        ("Worker Label", "Worker: GOAP planning loop", -620.0, -58.0),
    ] {
        commands.spawn((
            Name::new(name),
            Text2d::new(label),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.94, 0.88)),
            Transform::from_xyz(x, y, 2.0),
        ));
    }

    let target = commands
        .spawn((
            Name::new("Threat Beacon"),
            OrbitingTarget,
            Sprite::from_color(Color::srgb(0.92, 0.30, 0.25), Vec2::new(42.0, 42.0)),
            Transform::from_xyz(-60.0, 185.0, 1.0),
        ))
        .id();

    let (fsm_definition_id, state_machine_keys) = build_guard_state_machine(&mut state_machines);
    let (tree_definition_id, behavior_tree_keys) =
        build_guard_behavior_tree(&mut trees, &mut tree_handlers);

    let guard = commands
        .spawn((
            Name::new("Decision Guard"),
            Guard,
            fsm::StateMachineInstance::new(fsm_definition_id),
            fsm::Blackboard::from_schema(
                &state_machines
                    .definition(fsm_definition_id)
                    .unwrap()
                    .blackboard_schema,
            ),
            bt::BehaviorTreeAgent::new(tree_definition_id).with_config(bt::BehaviorTreeConfig {
                restart_on_completion: true,
                tick_mode: bt::TickMode::Interval {
                    seconds: 0.1,
                    phase_offset: 0.0,
                },
                ..default()
            }),
            uai::UtilityAgent::default(),
            uai::EvaluationPolicy {
                base_interval_seconds: 0.1,
                pending_request: true,
                ..default()
            },
            uai::DecisionMomentum {
                active_action_bonus: 0.12,
                hysteresis_band: 0.06,
                momentum_decay_per_second: 0.0,
            },
            Sprite::from_color(Color::srgb(0.26, 0.62, 0.94), Vec2::new(64.0, 64.0)),
            Transform::from_xyz(-420.0, 185.0, 1.0),
        ))
        .id();

    commands.entity(guard).with_children(|guard_children| {
        guard_children
            .spawn((Name::new("Attack"), uai::UtilityAction::new("attack")))
            .with_children(|action| {
                action.spawn((
                    Name::new("Attack Opportunity"),
                    AttackOpportunity,
                    uai::UtilityConsideration::new("opportunity", uai::ResponseCurve::SmoothStep),
                    uai::ConsiderationInput::default(),
                ));
            });

        guard_children
            .spawn((Name::new("Retreat"), uai::UtilityAction::new("retreat")))
            .with_children(|action| {
                action.spawn((
                    Name::new("Retreat Pressure"),
                    RetreatPressure,
                    uai::UtilityConsideration::new(
                        "pressure",
                        uai::ResponseCurve::Logistic {
                            midpoint: 0.45,
                            steepness: 8.0,
                        },
                    ),
                    uai::ConsiderationInput::default(),
                ));
            });
    });

    let worker_domain = build_worker_domain(&mut goap_library, &mut goap_hooks);
    let worker = commands
        .spawn((
            Name::new("Village Worker"),
            Worker,
            WorkerInventory::default(),
            goap::GoapAgent::new(worker_domain),
            Sprite::from_color(Color::srgb(0.26, 0.78, 0.52), Vec2::new(58.0, 58.0)),
            Transform::from_xyz(-520.0, -170.0, 1.0),
        ))
        .id();

    for (name, color, x) in [
        ("Ore Node", Color::srgb(0.54, 0.38, 0.28), -250.0),
        ("Workbench", Color::srgb(0.82, 0.68, 0.26), -10.0),
        ("Depot", Color::srgb(0.26, 0.72, 0.44), 220.0),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh2d(meshes.add(Rectangle::new(120.0, 96.0))),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(x, -170.0, 0.0),
        ));
    }

    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Text::new(String::new()),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(1040.0),
            top: px(56.0),
            width: px(360.0),
            ..default()
        },
    ));

    commands.insert_resource(LayeredEntities {
        guard,
        target,
        worker,
    });
    commands.insert_resource(state_machine_keys);
    commands.insert_resource(behavior_tree_keys);
}

fn build_guard_state_machine(
    library: &mut fsm::StateMachineLibrary,
) -> (fsm::StateMachineDefinitionId, GuardStateMachineKeys) {
    let mut builder = fsm::StateMachineBuilder::new("guard_modes");
    let target_visible = builder.blackboard_key(
        "target_visible",
        fsm::BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    let panic = builder.blackboard_key(
        "panic",
        fsm::BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    let root = builder.root_region("root");
    let patrol = builder.atomic_state("Patrol");
    let combat = builder.atomic_state("Combat");
    let retreat = builder.atomic_state("Retreat");
    builder
        .add_state_to_region(patrol, root)
        .add_state_to_region(combat, root)
        .add_state_to_region(retreat, root)
        .set_region_initial(root, patrol)
        .add_transition(fsm::TransitionDefinition::replace(patrol, combat).with_guard(FSM_VISIBLE))
        .add_transition(fsm::TransitionDefinition::replace(combat, patrol).with_guard(FSM_HIDDEN))
        .add_transition(fsm::TransitionDefinition::replace(combat, retreat).with_guard(FSM_PANIC))
        .add_transition(
            fsm::TransitionDefinition::replace(retreat, combat)
                .with_guard(FSM_CALM)
                .with_guard(FSM_VISIBLE),
        )
        .add_transition(fsm::TransitionDefinition::replace(retreat, patrol).with_guard(FSM_HIDDEN));

    let definition = builder.build().unwrap();
    let definition_id = library.register(definition).unwrap();
    (
        definition_id,
        GuardStateMachineKeys {
            target_visible,
            panic,
        },
    )
}

fn build_guard_behavior_tree(
    library: &mut bt::BehaviorTreeLibrary,
    handlers: &mut bt::BehaviorTreeHandlers,
) -> (bt::BehaviorTreeDefinitionId, GuardBehaviorTreeKeys) {
    let mut builder = bt::BehaviorTreeBuilder::new("guard_tactics");
    let visible = builder.bool_key(
        "target_visible",
        bt::BlackboardKeyDirection::Input,
        false,
        Some(false),
    );
    let can_chase = builder.condition_with_watch_keys("CanChase", "visible", [visible]);
    let chase = builder.action("Chase", "chase");
    let chase_branch = builder.sequence("ChaseBranch", [can_chase, chase]);
    let hold = builder.action("Hold", "hold");
    let root =
        builder.reactive_selector("Root", bt::AbortPolicy::LowerPriority, [chase_branch, hold]);
    builder.set_root(root);
    let definition_id = library.register(builder.build().unwrap()).unwrap();

    handlers.register_condition(
        "visible",
        bt::ConditionHandler::new(move |ctx| ctx.blackboard.get_bool(visible).unwrap_or(false)),
    );
    handlers.register_action(
        "chase",
        bt::ActionHandler::stateful(
            |_ctx| bt::BehaviorStatus::Running,
            |ctx| {
                let delta = ctx.world.resource::<Time>().delta_secs();
                let target = ctx.world.resource::<LayeredEntities>().target;
                let Some(target_position) = ctx
                    .world
                    .get::<Transform>(target)
                    .map(|transform| transform.translation)
                else {
                    return bt::BehaviorStatus::Failure;
                };
                let Some(current_position) = ctx
                    .world
                    .get::<Transform>(ctx.entity)
                    .map(|transform| transform.translation)
                else {
                    return bt::BehaviorStatus::Failure;
                };
                let offset = target_position - current_position;
                if offset.length() <= 18.0 {
                    return bt::BehaviorStatus::Success;
                }
                if let Some(mut transform) = ctx.world.get_mut::<Transform>(ctx.entity) {
                    transform.translation += offset.normalize_or_zero() * (140.0 * delta);
                }
                bt::BehaviorStatus::Running
            },
            |_ctx| {},
        ),
    );
    handlers.register_action(
        "hold",
        bt::ActionHandler::stateful(
            |_ctx| bt::BehaviorStatus::Running,
            |ctx| {
                let home = Vec3::new(-420.0, 185.0, 1.0);
                let Some(current_position) = ctx
                    .world
                    .get::<Transform>(ctx.entity)
                    .map(|transform| transform.translation)
                else {
                    return bt::BehaviorStatus::Failure;
                };
                if let Some(mut transform) = ctx.world.get_mut::<Transform>(ctx.entity) {
                    transform.translation = current_position.lerp(home, 0.06);
                }
                bt::BehaviorStatus::Running
            },
            |_ctx| {},
        ),
    );

    (definition_id, GuardBehaviorTreeKeys { visible })
}

fn build_worker_domain(
    library: &mut goap::GoapLibrary,
    hooks: &mut goap::GoapHooks,
) -> goap::GoapDomainId {
    let mut domain = goap::GoapDomainDefinition::new("worker_cycle");
    let has_ore = domain.add_bool_key("has_ore", Some("worker carries ore".into()), Some(false));
    let delivered = domain.add_bool_key(
        "delivered",
        Some("worker delivered a package".into()),
        Some(false),
    );
    let workbench_open = domain.add_bool_key(
        "workbench_open",
        Some("workbench accepts ore".into()),
        Some(true),
    );

    domain.add_local_sensor(
        goap::SensorDefinition::new(
            goap::SensorId(0),
            "worker_inventory",
            goap::SensorScope::Local,
            "worker_inventory",
            [has_ore, delivered],
        )
        .with_interval(goap::SensorInterval::every(0.0)),
    );
    domain.add_global_sensor(
        goap::SensorDefinition::new(
            goap::SensorId(0),
            "workbench_sensor",
            goap::SensorScope::Global,
            "workbench_sensor",
            [workbench_open],
        )
        .with_interval(goap::SensorInterval::every(0.0)),
    );
    domain.add_goal(
        goap::GoalDefinition::new(goap::GoalId(0), "deliver supply")
            .with_priority(12)
            .with_desired_state([goap::FactCondition::equals_bool(delivered, true)]),
    );
    domain.add_action(
        goap::ActionDefinition::new(goap::ActionId(0), "gather ore", "gather_ore")
            .with_effects([goap::FactEffect::set_bool(has_ore, true)]),
    );
    domain.add_action(
        goap::ActionDefinition::new(goap::ActionId(1), "deliver ore", "deliver_ore")
            .with_preconditions([
                goap::FactCondition::equals_bool(has_ore, true),
                goap::FactCondition::equals_bool(workbench_open, true),
            ])
            .with_effects([
                goap::FactEffect::set_bool(has_ore, false),
                goap::FactEffect::set_bool(delivered, true),
            ]),
    );

    hooks.register_local_sensor("worker_inventory", move |world, ctx| {
        let inventory = world
            .get::<WorkerInventory>(ctx.entity)
            .copied()
            .unwrap_or_default();
        goap::SensorOutput::new([
            goap::FactPatch::set_bool(has_ore, inventory.has_ore),
            goap::FactPatch::set_bool(delivered, inventory.delivered),
        ])
    });
    hooks.register_global_sensor("workbench_sensor", move |world, _ctx| {
        goap::SensorOutput::new([goap::FactPatch::set_bool(
            workbench_open,
            world.resource::<LayeredAiPane>().workbench_open,
        )])
    });

    library.register(domain)
}

fn sync_pane_runtime(
    pane: Res<LayeredAiPane>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut planner: ResMut<goap::GoapPlannerScheduler>,
    mut budget: ResMut<uai::UtilityAiBudget>,
    mut goap_agents: Query<&mut goap::GoapAgent, With<Worker>>,
) {
    if !pane.is_changed() {
        return;
    }

    virtual_time.set_relative_speed(pane.time_scale.max(0.1));
    planner.max_agents_per_frame = 8;
    budget.max_agents_per_update = 32;

    for mut agent in &mut goap_agents {
        agent.config.goal_switch_margin = 0.25;
        agent.config.replan_on_sensed_state_change = true;
    }
}

fn animate_target(
    time: Res<Time>,
    pane: Res<LayeredAiPane>,
    mut target: Query<&mut Transform, With<OrbitingTarget>>,
) {
    let Ok(mut transform) = target.single_mut() else {
        return;
    };
    let angle = time.elapsed_secs() * 0.7;
    transform.translation = Vec3::new(
        -140.0 + angle.cos() * pane.target_radius,
        185.0 + angle.sin() * 90.0,
        1.0,
    );
}

fn sync_guard_inputs(
    pane: Res<LayeredAiPane>,
    entities: Res<LayeredEntities>,
    state_machine_keys: Res<GuardStateMachineKeys>,
    behavior_tree_keys: Res<GuardBehaviorTreeKeys>,
    guard_query: Query<&Transform, With<Guard>>,
    target_query: Query<&Transform, With<OrbitingTarget>>,
    mut blackboards: Query<&mut fsm::Blackboard, With<Guard>>,
    mut bt_blackboards: Query<&mut bt::BehaviorTreeBlackboard, With<Guard>>,
    mut attack_inputs: Query<
        &mut uai::ConsiderationInput,
        (With<AttackOpportunity>, Without<RetreatPressure>),
    >,
    mut retreat_inputs: Query<
        &mut uai::ConsiderationInput,
        (With<RetreatPressure>, Without<AttackOpportunity>),
    >,
) {
    let Ok(guard_transform) = guard_query.get(entities.guard) else {
        return;
    };
    let Ok(target_transform) = target_query.get(entities.target) else {
        return;
    };
    let distance = guard_transform
        .translation
        .distance(target_transform.translation);
    let visible = distance <= pane.alert_distance;
    let normalized_distance = 1.0 - (distance / pane.alert_distance.max(1.0)).clamp(0.0, 1.0);
    let retreat_pressure = (pane.retreat_bias + (1.0 - normalized_distance) * 0.85).clamp(0.0, 1.0);

    if let Ok(mut blackboard) = blackboards.get_mut(entities.guard) {
        let _ = blackboard.set(state_machine_keys.target_visible, visible);
    }
    if let Ok(mut blackboard) = bt_blackboards.get_mut(entities.guard) {
        let _ = blackboard.set(behavior_tree_keys.visible, visible);
    }
    if let Ok(mut input) = attack_inputs.single_mut() {
        input.value = Some(normalized_distance.clamp(0.0, 1.0));
        input.enabled = true;
    }
    if let Ok(mut input) = retreat_inputs.single_mut() {
        input.value = Some(retreat_pressure);
        input.enabled = true;
    }
}

fn sync_utility_to_state_machine(
    entities: Res<LayeredEntities>,
    keys: Res<GuardStateMachineKeys>,
    active_actions: Query<&uai::ActiveAction, With<Guard>>,
    mut blackboards: Query<&mut fsm::Blackboard, With<Guard>>,
) {
    let Ok(active_action) = active_actions.get(entities.guard) else {
        return;
    };
    let Ok(mut blackboard) = blackboards.get_mut(entities.guard) else {
        return;
    };
    let panic = matches!(active_action.label.as_deref(), Some("retreat"));
    let _ = blackboard.set(keys.panic, panic);
}

fn process_worker_actions(
    time: Res<Time>,
    pane: Res<LayeredAiPane>,
    mut loop_state: ResMut<WorkerLoopState>,
    mut dispatched: MessageReader<goap::ActionDispatched>,
    mut reports: MessageWriter<goap::ActionExecutionReport>,
    mut workers: Query<&mut WorkerInventory, With<Worker>>,
) {
    for message in dispatched.read() {
        let Ok(mut inventory) = workers.get_mut(message.entity) else {
            continue;
        };
        match message.executor.as_str() {
            "gather_ore" => {
                inventory.has_ore = true;
                inventory.delivered = false;
                reports.write(goap::ActionExecutionReport::new(
                    message.entity,
                    message.ticket,
                    goap::ActionExecutionStatus::Success,
                ));
            }
            "deliver_ore" if pane.workbench_open => {
                inventory.has_ore = false;
                inventory.delivered = true;
                loop_state.delivered_at_seconds = Some(time.elapsed_secs());
                reports.write(goap::ActionExecutionReport::new(
                    message.entity,
                    message.ticket,
                    goap::ActionExecutionStatus::Success,
                ));
            }
            "deliver_ore" => {
                reports.write(goap::ActionExecutionReport::new(
                    message.entity,
                    message.ticket,
                    goap::ActionExecutionStatus::Failure {
                        reason: "workbench closed".into(),
                    },
                ));
            }
            _ => {}
        }
    }
}

fn reset_worker_cycle(
    time: Res<Time>,
    mut loop_state: ResMut<WorkerLoopState>,
    entities: Res<LayeredEntities>,
    mut workers: Query<&mut WorkerInventory, With<Worker>>,
    mut invalidations: MessageWriter<goap::InvalidateGoapAgent>,
) {
    let Some(delivered_at) = loop_state.delivered_at_seconds else {
        return;
    };
    if time.elapsed_secs() - delivered_at < 2.0 {
        return;
    }
    if let Ok(mut inventory) = workers.get_mut(entities.worker) {
        inventory.has_ore = false;
        inventory.delivered = false;
    }
    invalidations.write(goap::InvalidateGoapAgent {
        entity: entities.worker,
        reason: goap::PlanInvalidationReason::Manual {
            reason: "restart worker loop".into(),
        },
    });
    loop_state.delivered_at_seconds = None;
}

fn tint_actors(
    entities: Res<LayeredEntities>,
    state_machines: Res<fsm::StateMachineLibrary>,
    guard_states: Query<&fsm::StateMachineInstance, With<Guard>>,
    utility_actions: Query<&uai::ActiveAction, With<Guard>>,
    worker_inventory: Query<&WorkerInventory, With<Worker>>,
    mut sprites: Query<&mut Sprite>,
) {
    if let (Ok(instance), Ok(active_action), Ok(mut sprite)) = (
        guard_states.get(entities.guard),
        utility_actions.get(entities.guard),
        sprites.get_mut(entities.guard),
    ) {
        let state_name = instance
            .active_leaf()
            .and_then(|state_id| {
                state_machines
                    .definition(instance.definition_id)?
                    .state(state_id)
            })
            .map(|state| state.name.as_str())
            .unwrap_or("Patrol");

        sprite.color = match (state_name, active_action.label.as_deref()) {
            ("Retreat", _) => Color::srgb(0.92, 0.32, 0.26),
            ("Combat", Some("attack")) => Color::srgb(0.96, 0.78, 0.28),
            ("Combat", _) => Color::srgb(0.86, 0.54, 0.22),
            _ => Color::srgb(0.26, 0.62, 0.94),
        };
    }

    if let (Ok(inventory), Ok(mut sprite)) = (
        worker_inventory.get(entities.worker),
        sprites.get_mut(entities.worker),
    ) {
        sprite.color = if inventory.delivered {
            Color::srgb(0.92, 0.78, 0.30)
        } else if inventory.has_ore {
            Color::srgb(0.24, 0.74, 0.88)
        } else {
            Color::srgb(0.26, 0.78, 0.52)
        };
    }
}

fn update_overlay(
    entities: Res<LayeredEntities>,
    pane: Res<LayeredAiPane>,
    state_machine_library: Res<fsm::StateMachineLibrary>,
    guard_states: Query<&fsm::StateMachineInstance, With<Guard>>,
    guard_tree: Query<&bt::BehaviorTreeInstance, With<Guard>>,
    guard_utility: Query<&uai::ActiveAction, With<Guard>>,
    worker_runtime: Query<&goap::GoapRuntime, With<Worker>>,
    worker_inventory: Query<&WorkerInventory, With<Worker>>,
    mut overlay: Query<&mut Text, With<Overlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    let guard_mode = guard_states
        .get(entities.guard)
        .ok()
        .and_then(|instance| {
            instance
                .active_leaf()
                .and_then(|state_id| {
                    state_machine_library
                        .definition(instance.definition_id)?
                        .state(state_id)
                })
                .map(|state| state.name.clone())
        })
        .unwrap_or_else(|| "Unknown".into());
    let tree_status = guard_tree
        .get(entities.guard)
        .map(|instance| format!("{:?}", instance.status))
        .unwrap_or_else(|_| "Missing".into());
    let utility_action = guard_utility
        .get(entities.guard)
        .ok()
        .and_then(|action| action.label.clone())
        .unwrap_or_else(|| "None".into());
    let worker_goal = worker_runtime
        .get(entities.worker)
        .ok()
        .and_then(|runtime| runtime.current_goal.as_ref().map(|goal| goal.name.clone()))
        .unwrap_or_else(|| "Idle".into());
    let worker_step = worker_runtime
        .get(entities.worker)
        .ok()
        .and_then(|runtime| {
            runtime
                .active_action
                .as_ref()
                .map(|action| action.action_name.clone())
        })
        .unwrap_or_else(|| "Waiting".into());
    let worker_inventory = worker_inventory
        .get(entities.worker)
        .copied()
        .unwrap_or_default();

    text.0 = format!(
        "Guard Layering\n\
         FSM mode: {guard_mode}\n\
         Behavior tree: {tree_status}\n\
         Utility choice: {utility_action}\n\n\
         Worker Planning\n\
         Goal: {worker_goal}\n\
         Step: {worker_step}\n\
         Has ore: {}\n\
         Delivered: {}\n\n\
         Live Tunables\n\
         Alert distance: {:.0}\n\
         Retreat bias: {:.2}\n\
         Workbench open: {}",
        worker_inventory.has_ore,
        worker_inventory.delivered,
        pane.alert_distance,
        pane.retreat_bias,
        pane.workbench_open,
    );
}
