use crate::definition::{StateId, StateMachineDefinition};

pub fn path_to_root(definition: &StateMachineDefinition, state_id: StateId) -> Vec<StateId> {
    let mut path = Vec::new();
    let mut current = Some(state_id);
    while let Some(state) = current.and_then(|id| definition.state(id)) {
        path.push(state.id);
        current = state.parent_state;
    }
    path.reverse();
    path
}

pub fn depth(definition: &StateMachineDefinition, state_id: StateId) -> usize {
    path_to_root(definition, state_id).len()
}

pub fn least_common_ancestor(
    definition: &StateMachineDefinition,
    left: StateId,
    right: StateId,
) -> Option<StateId> {
    let left_path = path_to_root(definition, left);
    let right_path = path_to_root(definition, right);
    let mut lca = None;
    for (left, right) in left_path.into_iter().zip(right_path) {
        if left == right {
            lca = Some(left);
        } else {
            break;
        }
    }
    lca
}

pub fn is_descendant_of(
    definition: &StateMachineDefinition,
    candidate: StateId,
    ancestor: StateId,
) -> bool {
    path_to_root(definition, candidate).contains(&ancestor)
}

pub fn direct_child_below(
    definition: &StateMachineDefinition,
    ancestor: StateId,
    descendant: StateId,
) -> Option<StateId> {
    let path = path_to_root(definition, descendant);
    let index = path.iter().position(|state| *state == ancestor)?;
    path.get(index + 1).copied()
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
