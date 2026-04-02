use super::*;

#[test]
fn least_common_ancestor_finds_branch_point() {
    let mut builder = crate::builder::StateMachineBuilder::new("Hierarchy");
    let root = builder.root_region("root");
    let parent = builder.compound_state("Parent");
    let child_region = builder.region("child", parent);
    let left = builder.atomic_state("Left");
    let right = builder.atomic_state("Right");

    builder
        .add_state_to_region(parent, root)
        .set_region_initial(root, parent)
        .add_state_to_region(left, child_region)
        .add_state_to_region(right, child_region)
        .set_region_initial(child_region, left);

    let definition = builder.build().unwrap();
    assert_eq!(path_to_root(&definition, left), vec![parent, left]);
    assert_eq!(
        least_common_ancestor(&definition, left, right),
        Some(parent)
    );
}
