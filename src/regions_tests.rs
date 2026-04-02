use super::*;

#[test]
fn root_region_resolves_for_nested_state() {
    let mut builder = crate::builder::StateMachineBuilder::new("Regions");
    let root = builder.root_region("root");
    let parent = builder.parallel_state("Parent");
    let locomotion = builder.region("locomotion", parent);
    let grounded = builder.atomic_state("Grounded");

    builder
        .add_state_to_region(parent, root)
        .set_region_initial(root, parent)
        .add_state_to_region(grounded, locomotion)
        .set_region_initial(locomotion, grounded);

    let definition = builder.build().unwrap();
    assert_eq!(root_region_for_state(&definition, grounded), Some(root));
}
