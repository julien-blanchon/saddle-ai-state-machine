use super::*;

#[test]
fn builder_assigns_stable_dense_ids() {
    let mut builder = StateMachineBuilder::new("DenseIds");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let run = builder.atomic_state("Run");

    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(run, root)
        .set_region_initial(root, idle)
        .add_transition(TransitionDefinition::replace(idle, run));

    let definition = builder.build().unwrap();
    assert_eq!(definition.states[0].id, idle);
    assert_eq!(definition.states[1].id, run);
    assert_eq!(definition.transitions[0].id, TransitionId(0));
}
