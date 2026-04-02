use std::collections::{BTreeMap, BTreeSet};

use crate::definition::{
    HistoryMode, RegionId, StateId, StateKind, StateMachineDefinition, TransitionOperation,
    TransitionSource, TransitionTrigger,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn push(
        &mut self,
        severity: ValidationSeverity,
        code: &'static str,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue {
            severity,
            code,
            message: message.into(),
        });
    }

    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error)
    }
}

pub fn validate_definition(definition: &StateMachineDefinition) -> ValidationReport {
    let mut report = ValidationReport::default();

    if definition.root_regions.is_empty() {
        report.push(
            ValidationSeverity::Error,
            "missing_root_region",
            "state machine must declare at least one root region",
        );
    }

    let mut blackboard_names = BTreeSet::new();
    for key in &definition.blackboard_schema {
        if !blackboard_names.insert(key.name.clone()) {
            report.push(
                ValidationSeverity::Error,
                "duplicate_blackboard_key_name",
                format!("duplicate blackboard key name '{}'", key.name),
            );
        }
        if let Some(default_value) = &key.default_value
            && default_value.value_type() != key.value_type
        {
            report.push(
                ValidationSeverity::Error,
                "blackboard_default_type_mismatch",
                format!(
                    "blackboard key '{}' declares {:?} but default value is {:?}",
                    key.name,
                    key.value_type,
                    default_value.value_type()
                ),
            );
        }
    }

    let mut state_names = BTreeSet::new();
    for (index, state) in definition.states.iter().enumerate() {
        if state.id.0 as usize != index {
            report.push(
                ValidationSeverity::Error,
                "invalid_state_id",
                format!(
                    "state '{}' uses non-dense or out-of-order id {:?}",
                    state.name, state.id
                ),
            );
        }
        if !state_names.insert(state.name.clone()) {
            report.push(
                ValidationSeverity::Error,
                "duplicate_state_name",
                format!("duplicate state name '{}'", state.name),
            );
        }
        if let Some(parent_region) = state.parent_region
            && definition.region(parent_region).is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_parent_region",
                format!(
                    "state '{}' references missing parent region {:?}",
                    state.name, parent_region
                ),
            );
        }
        if let Some(parent_state) = state.parent_state
            && definition.state(parent_state).is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_parent_state",
                format!(
                    "state '{}' references missing parent state {:?}",
                    state.name, parent_state
                ),
            );
        }

        if matches!(state.kind, StateKind::Compound | StateKind::Parallel)
            && state.child_regions.is_empty()
        {
            report.push(
                ValidationSeverity::Error,
                "missing_child_region",
                format!("state '{}' requires at least one child region", state.name),
            );
        }

        if matches!(
            state.kind,
            StateKind::Atomic | StateKind::Final | StateKind::Transient
        ) && !state.child_regions.is_empty()
        {
            report.push(
                ValidationSeverity::Error,
                "unexpected_child_region",
                format!("state '{}' cannot own child regions", state.name),
            );
        }
    }

    let mut region_names = BTreeSet::new();
    for (index, region) in definition.regions.iter().enumerate() {
        if region.id.0 as usize != index {
            report.push(
                ValidationSeverity::Error,
                "invalid_region_id",
                format!(
                    "region '{}' uses non-dense or out-of-order id {:?}",
                    region.name, region.id
                ),
            );
        }
        if !region_names.insert(region.name.clone()) {
            report.push(
                ValidationSeverity::Error,
                "duplicate_region_name",
                format!("duplicate region name '{}'", region.name),
            );
        }
        if let Some(parent_state) = region.parent_state
            && definition.state(parent_state).is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_region_parent",
                format!(
                    "region '{}' references missing parent state {:?}",
                    region.name, parent_state
                ),
            );
        }

        if region.child_states.is_empty() {
            report.push(
                ValidationSeverity::Warning,
                "empty_region",
                format!("region '{}' has no child states", region.name),
            );
        }

        if !region.child_states.is_empty() && region.initial_state.is_none() {
            report.push(
                ValidationSeverity::Error,
                "missing_initial_state",
                format!("region '{}' is missing an initial state", region.name),
            );
        }
        if let Some(initial_state) = region.initial_state
            && !region.child_states.contains(&initial_state)
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_initial_state",
                format!(
                    "region '{}' uses initial state {:?} that is not one of its children",
                    region.name, initial_state
                ),
            );
        }
        for child_state in &region.child_states {
            let Some(state) = definition.state(*child_state) else {
                report.push(
                    ValidationSeverity::Error,
                    "invalid_region_child",
                    format!(
                        "region '{}' references missing child state {:?}",
                        region.name, child_state
                    ),
                );
                continue;
            };
            if state.parent_region != Some(region.id) {
                report.push(
                    ValidationSeverity::Error,
                    "inconsistent_parent_region",
                    format!(
                        "state '{}' is listed in region '{}' but points to {:?} as parent region",
                        state.name, region.name, state.parent_region
                    ),
                );
            }
        }
    }

    let mut root_regions = BTreeSet::new();
    for root_region in &definition.root_regions {
        let Some(region) = definition.region(*root_region) else {
            report.push(
                ValidationSeverity::Error,
                "invalid_root_region",
                format!(
                    "root region list references missing region {:?}",
                    root_region
                ),
            );
            continue;
        };
        if !root_regions.insert(*root_region) {
            report.push(
                ValidationSeverity::Error,
                "duplicate_root_region",
                format!("root region '{:?}' is listed more than once", root_region),
            );
        }
        if region.parent_state.is_some() {
            report.push(
                ValidationSeverity::Error,
                "non_root_region_in_root_list",
                format!(
                    "region '{}' is listed as a root region but has a parent state",
                    region.name
                ),
            );
        }
    }

    let mut reachable = BTreeSet::new();
    for root_region in &definition.root_regions {
        mark_reachable(definition, *root_region, &mut reachable);
    }
    for state in &definition.states {
        if !reachable.contains(&state.id) {
            report.push(
                ValidationSeverity::Error,
                "unreachable_state",
                format!(
                    "state '{}' is not reachable from any root region",
                    state.name
                ),
            );
        }
    }

    for transition in &definition.transitions {
        if let TransitionSource::State(source) = transition.source
            && definition.state(source).is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_transition_source",
                format!(
                    "transition {:?} references missing source state {:?}",
                    transition.id, source
                ),
            );
            continue;
        }
        if let Some(target) = transition.target
            && definition.state(target).is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_transition_target",
                format!(
                    "transition {:?} references missing target state {:?}",
                    transition.id, target
                ),
            );
            continue;
        }
        if matches!(transition.operation, TransitionOperation::Pop) && transition.target.is_some() {
            report.push(
                ValidationSeverity::Error,
                "pop_has_target",
                format!(
                    "transition {:?} is a pop transition and cannot have a target",
                    transition.id
                ),
            );
        }

        if !matches!(transition.operation, TransitionOperation::Pop) && transition.target.is_none()
        {
            report.push(
                ValidationSeverity::Error,
                "missing_target",
                format!("transition {:?} is missing a target", transition.id),
            );
        }

        if matches!(transition.source, TransitionSource::AnyState)
            && matches!(transition.trigger, TransitionTrigger::AfterSeconds(_))
        {
            report.push(
                ValidationSeverity::Error,
                "after_on_any_state",
                format!(
                    "transition {:?} cannot use `after` from AnyState",
                    transition.id
                ),
            );
        }
        if matches!(transition.trigger, TransitionTrigger::Done)
            && transition
                .source
                .and_then_state()
                .and_then(|state_id| definition.state(state_id))
                .is_none_or(|state| {
                    !matches!(state.kind, StateKind::Compound | StateKind::Parallel)
                })
        {
            report.push(
                ValidationSeverity::Error,
                "done_requires_compound_or_parallel_source",
                format!(
                    "transition {:?} uses `done` but its source is not a compound or parallel state",
                    transition.id
                ),
            );
        }

        if let TransitionSource::State(source) = transition.source {
            if let Some(target) = transition.target
                && unsupported_cross_region_transition(definition, source, target)
            {
                report.push(
                    ValidationSeverity::Error,
                    "unsupported_cross_region_transition",
                    format!(
                        "transition {:?} crosses sibling parallel regions, which v0.1 defers",
                        transition.id
                    ),
                );
            }
        }
    }

    validate_transient_cycles(definition, &mut report);
    validate_history(definition, &mut report);

    report
}

fn mark_reachable(
    definition: &StateMachineDefinition,
    region_id: RegionId,
    reachable: &mut BTreeSet<StateId>,
) {
    let Some(region) = definition.region(region_id) else {
        return;
    };
    for state_id in &region.child_states {
        if reachable.insert(*state_id) {
            let Some(state) = definition.state(*state_id) else {
                continue;
            };
            for child_region in &state.child_regions {
                mark_reachable(definition, *child_region, reachable);
            }
        }
    }
}

fn validate_history(definition: &StateMachineDefinition, report: &mut ValidationReport) {
    for state in &definition.states {
        if !matches!(state.history_mode, HistoryMode::None)
            && !matches!(state.kind, StateKind::Compound | StateKind::Parallel)
        {
            report.push(
                ValidationSeverity::Error,
                "invalid_history_mode",
                format!(
                    "state '{}' cannot use {:?} history because it has no child regions",
                    state.name, state.history_mode
                ),
            );
        }
    }
}

fn validate_transient_cycles(definition: &StateMachineDefinition, report: &mut ValidationReport) {
    let mut graph: BTreeMap<StateId, Vec<StateId>> = BTreeMap::new();
    for transition in &definition.transitions {
        let TransitionSource::State(source) = transition.source else {
            continue;
        };
        let Some(source_state) = definition.state(source) else {
            continue;
        };
        if source_state.kind != StateKind::Transient {
            continue;
        }
        let Some(target) = transition.target else {
            continue;
        };
        let Some(target_state) = definition.state(target) else {
            continue;
        };
        if target_state.kind != StateKind::Transient {
            continue;
        }
        if !matches!(transition.trigger, TransitionTrigger::Automatic) || transition.guard.is_some()
        {
            continue;
        }
        graph.entry(source).or_default().push(target);
    }

    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    for node in graph.keys().copied() {
        if detect_cycle(node, &graph, &mut temporary, &mut permanent) {
            report.push(
                ValidationSeverity::Error,
                "transient_cycle",
                "machine contains an unconditional transient-state cycle that would never quiesce",
            );
            return;
        }
    }
}

fn detect_cycle(
    node: StateId,
    graph: &BTreeMap<StateId, Vec<StateId>>,
    temporary: &mut BTreeSet<StateId>,
    permanent: &mut BTreeSet<StateId>,
) -> bool {
    if permanent.contains(&node) {
        return false;
    }
    if !temporary.insert(node) {
        return true;
    }
    if let Some(children) = graph.get(&node) {
        for child in children {
            if detect_cycle(*child, graph, temporary, permanent) {
                return true;
            }
        }
    }
    temporary.remove(&node);
    permanent.insert(node);
    false
}

fn unsupported_cross_region_transition(
    definition: &StateMachineDefinition,
    source: StateId,
    target: StateId,
) -> bool {
    let Some(source_root) = root_parallel_region(definition, source) else {
        return false;
    };
    let Some(target_root) = root_parallel_region(definition, target) else {
        return false;
    };
    source_root != target_root
}

fn root_parallel_region(
    definition: &StateMachineDefinition,
    mut state_id: StateId,
) -> Option<RegionId> {
    while let Some(state) = definition.state(state_id) {
        let parent_region = state.parent_region?;
        let parent = state.parent_state?;
        if definition.state(parent)?.kind == StateKind::Parallel {
            return Some(parent_region);
        }
        state_id = parent;
    }
    None
}

trait TransitionSourceExt {
    fn and_then_state(self) -> Option<StateId>;
}

impl TransitionSourceExt for TransitionSource {
    fn and_then_state(self) -> Option<StateId> {
        match self {
            TransitionSource::State(state_id) => Some(state_id),
            TransitionSource::AnyState => None,
        }
    }
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
