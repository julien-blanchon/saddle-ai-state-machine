use crate::definition::{RegionId, StateId, StateMachineDefinition};

pub fn region_is_enabled(enabled_regions: &[RegionId], region_id: RegionId) -> bool {
    enabled_regions.is_empty() || enabled_regions.contains(&region_id)
}

pub fn state_region(definition: &StateMachineDefinition, state_id: StateId) -> Option<RegionId> {
    definition
        .state(state_id)
        .and_then(|state| state.parent_region)
}

pub fn root_region_for_state(
    definition: &StateMachineDefinition,
    state_id: StateId,
) -> Option<RegionId> {
    let mut current_region = state_region(definition, state_id)?;
    loop {
        let region = definition.region(current_region)?;
        match region.parent_state {
            Some(parent_state) => {
                current_region = definition.state(parent_state)?.parent_region?;
            }
            None => return Some(current_region),
        }
    }
}

#[cfg(test)]
#[path = "regions_tests.rs"]
mod tests;
