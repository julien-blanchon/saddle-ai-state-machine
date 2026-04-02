#[test]
fn duplicate_state_names_are_rejected() {
    let mut builder = crate::builder::StateMachineBuilder::new("Invalid");
    let root = builder.root_region("root");
    let idle_a = builder.atomic_state("Idle");
    let idle_b = builder.atomic_state("Idle");

    builder
        .add_state_to_region(idle_a, root)
        .add_state_to_region(idle_b, root)
        .set_region_initial(root, idle_a);

    let report = builder.build().unwrap_err();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "duplicate_state_name")
    );
}

#[test]
fn sibling_parallel_region_transitions_are_reported() {
    let mut builder = crate::builder::StateMachineBuilder::new("Parallel");
    let root = builder.root_region("root");
    let parent = builder.parallel_state("Parent");
    let left_region = builder.region("left", parent);
    let right_region = builder.region("right", parent);
    let left = builder.atomic_state("Left");
    let right = builder.atomic_state("Right");

    builder
        .add_state_to_region(parent, root)
        .set_region_initial(root, parent)
        .add_state_to_region(left, left_region)
        .add_state_to_region(right, right_region)
        .set_region_initial(left_region, left)
        .set_region_initial(right_region, right)
        .add_transition(crate::definition::TransitionDefinition::replace(
            left, right,
        ));

    let report = builder.build().unwrap_err();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unsupported_cross_region_transition")
    );
}

#[test]
fn same_parallel_region_transitions_are_allowed() {
    let mut builder = crate::builder::StateMachineBuilder::new("ParallelSameRegion");
    let root = builder.root_region("root");
    let parent = builder.parallel_state("Parent");
    let left_region = builder.region("left", parent);
    let right_region = builder.region("right", parent);
    let left_a = builder.atomic_state("LeftA");
    let left_b = builder.atomic_state("LeftB");
    let right = builder.atomic_state("Right");

    builder
        .add_state_to_region(parent, root)
        .set_region_initial(root, parent)
        .add_state_to_region(left_a, left_region)
        .add_state_to_region(left_b, left_region)
        .add_state_to_region(right, right_region)
        .set_region_initial(left_region, left_a)
        .set_region_initial(right_region, right)
        .add_transition(crate::definition::TransitionDefinition::replace(
            left_a, left_b,
        ));

    let definition = builder.build().unwrap();
    assert_eq!(definition.transitions.len(), 1);
}

#[test]
fn duplicate_blackboard_key_names_are_rejected() {
    let mut builder = crate::builder::StateMachineBuilder::new("DuplicateBlackboard");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");

    builder.blackboard_key(
        "target_visible",
        crate::blackboard::BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    builder.blackboard_key(
        "target_visible",
        crate::blackboard::BlackboardValueType::Bool,
        false,
        Some(false.into()),
    );
    builder
        .add_state_to_region(idle, root)
        .set_region_initial(root, idle);

    let report = builder.build().unwrap_err();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "duplicate_blackboard_key_name")
    );
}

#[test]
fn blackboard_default_type_mismatch_is_rejected() {
    let mut builder = crate::builder::StateMachineBuilder::new("BlackboardDefaultMismatch");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");

    builder.blackboard_key(
        "target_visible",
        crate::blackboard::BlackboardValueType::Bool,
        false,
        Some(1_i32.into()),
    );
    builder
        .add_state_to_region(idle, root)
        .set_region_initial(root, idle);

    let report = builder.build().unwrap_err();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "blackboard_default_type_mismatch")
    );
}

#[test]
fn region_initial_state_must_belong_to_region() {
    let mut builder = crate::builder::StateMachineBuilder::new("InitialStateMembership");
    let root = builder.root_region("root");
    let other = builder.root_region("other");
    let idle = builder.atomic_state("Idle");
    let stray = builder.atomic_state("Stray");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(stray, other)
        .set_region_initial(root, stray);

    let report = builder.build().unwrap_err();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "invalid_initial_state")
    );
}
