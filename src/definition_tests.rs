#[test]
fn find_state_and_blackboard_key_by_name() {
    let mut builder = crate::builder::StateMachineBuilder::new("Lookup");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    builder
        .add_state_to_region(idle, root)
        .set_region_initial(root, idle);
    let alert_key = builder.blackboard_key(
        "alert",
        crate::blackboard::BlackboardValueType::Bool,
        false,
        None,
    );

    let definition = builder.build().unwrap();
    assert_eq!(definition.find_state_id("Idle"), Some(idle));
    assert_eq!(definition.find_blackboard_key("alert"), Some(alert_key));
}
