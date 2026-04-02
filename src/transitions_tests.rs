use super::*;

#[test]
fn choose_best_candidate_prefers_depth_then_priority_then_score() {
    let candidates = vec![
        EvaluatedTransition {
            transition_id: crate::definition::TransitionId(0),
            source_depth: 1,
            priority: 0,
            score: 0.8,
            declaration_order: 0,
            reason_if_blocked: None,
        },
        EvaluatedTransition {
            transition_id: crate::definition::TransitionId(1),
            source_depth: 2,
            priority: 0,
            score: 0.1,
            declaration_order: 1,
            reason_if_blocked: None,
        },
        EvaluatedTransition {
            transition_id: crate::definition::TransitionId(2),
            source_depth: 2,
            priority: 3,
            score: 0.0,
            declaration_order: 2,
            reason_if_blocked: None,
        },
    ];

    assert_eq!(
        choose_best_candidate(&candidates).unwrap().transition_id,
        crate::definition::TransitionId(2)
    );
}

#[test]
fn threshold_override_wins_over_definition_policy() {
    let mut instance =
        crate::instance::StateMachineInstance::new(crate::definition::StateMachineDefinitionId(1));
    instance
        .config
        .utility_threshold_overrides
        .push(crate::instance::InstanceThresholdOverride {
            transition_id: crate::definition::TransitionId(3),
            minimum_score: 0.9,
        });

    assert_eq!(
        threshold_for(
            &instance,
            crate::definition::TransitionId(3),
            crate::definition::UtilityPolicy::best_score_above(0.2),
        ),
        0.9
    );
}
