use super::*;
use crate::{StateMachineBuilder, StateMachineLibrary, TransitionDefinition, TransitionTrigger};

#[test]
fn state_machine_asset_round_trips_through_ron() {
    let mut builder = StateMachineBuilder::new("asset_roundtrip");
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

    let asset = StateMachineDefinitionAsset::from(builder.build().unwrap());
    let serialized = ron::ser::to_string(&asset).unwrap();
    let decoded: StateMachineDefinitionAsset = ron::de::from_str(&serialized).unwrap();

    let mut library = StateMachineLibrary::default();
    let definition_id = decoded.register(&mut library).unwrap();
    let definition = library.definition(definition_id).unwrap();

    assert_eq!(definition.name, "asset_roundtrip");
    assert_eq!(definition.states.len(), 2);
    assert_eq!(definition.transitions.len(), 1);
}
