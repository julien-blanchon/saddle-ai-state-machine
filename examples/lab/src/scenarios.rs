use saddle_bevy_e2e::action::Action;
use saddle_bevy_e2e::actions::{assertions, inspect};
use saddle_bevy_e2e::scenario::Scenario;

use crate::{LabAgent, LabDiagnostics, LabKeys, StateMachineLabPane, SIGNAL_STUN};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_lab",
        "basic_cycling",
        "hierarchical",
        "pushdown_stun",
        "guard_transitions",
        "delayed_transitions",
        "debug_annotations",
        "history_restore",
        "trace_recording",
        "full_lifecycle",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_lab" => Some(smoke_lab()),
        "basic_cycling" => Some(basic_cycling()),
        "hierarchical" => Some(hierarchical()),
        "pushdown_stun" => Some(pushdown_stun()),
        "guard_transitions" => Some(guard_transitions()),
        "delayed_transitions" => Some(delayed_transitions()),
        "debug_annotations" => Some(debug_annotations()),
        "history_restore" => Some(history_restore()),
        "trace_recording" => Some(trace_recording()),
        "full_lifecycle" => Some(full_lifecycle()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helper actions
// ---------------------------------------------------------------------------

/// Take full control: disable auto-stun, zero out pane radii and write blackboard
/// values directly so drive_machine doesn't interfere with scenario-driven state.
fn take_control() -> Action {
    Action::Custom(Box::new(|world| {
        let mut pane = world.resource_mut::<StateMachineLabPane>();
        pane.stun_interval = 999.0;
        pane.visibility_radius = 0.0;
        pane.attack_radius = 0.0;
        // Write directly to blackboard for immediate effect
        let keys = *world.resource::<LabKeys>();
        let mut query = world.query_filtered::<&mut saddle_ai_state_machine::Blackboard, bevy::prelude::With<LabAgent>>();
        for mut bb in query.iter_mut(world) {
            let _ = bb.set(keys.target_visible, false);
            let _ = bb.set(keys.in_attack_range, false);
        }
    }))
}

/// Set blackboard values directly AND update pane radii so drive_machine stays consistent.
fn set_blackboard(target_visible: bool, in_attack_range: bool) -> Action {
    Action::Custom(Box::new(move |world| {
        // Set pane radii so drive_machine writes consistent values on subsequent frames
        let mut pane = world.resource_mut::<StateMachineLabPane>();
        pane.visibility_radius = if target_visible { 999.0 } else { 0.0 };
        pane.attack_radius = if in_attack_range { 999.0 } else { 0.0 };
        // Also write directly to blackboard for immediate effect this frame
        let keys = *world.resource::<LabKeys>();
        let mut query = world.query_filtered::<&mut saddle_ai_state_machine::Blackboard, bevy::prelude::With<LabAgent>>();
        for mut bb in query.iter_mut(world) {
            let _ = bb.set(keys.target_visible, target_visible);
            let _ = bb.set(keys.in_attack_range, in_attack_range);
        }
    }))
}

/// Queue stun signal directly on the agent's state machine instance.
fn send_stun() -> Action {
    Action::Custom(Box::new(|world| {
        let mut query = world.query_filtered::<&mut saddle_ai_state_machine::StateMachineInstance, bevy::prelude::With<LabAgent>>();
        for mut instance in query.iter_mut(world) {
            instance.queue_signal(SIGNAL_STUN);
        }
    }))
}

/// Wait until the active leaf state name matches `name`.
fn wait_for_state(name: &str, max_frames: u32) -> Action {
    let name = name.to_string();
    Action::WaitUntil {
        label: format!("wait for state '{name}'"),
        condition: Box::new(move |world| {
            world.resource::<LabDiagnostics>().active_leaf_name == name
        }),
        max_frames,
    }
}

/// Assert the active leaf state name is `expected`.
fn assert_state(expected: &str) -> Action {
    let expected = expected.to_string();
    assertions::resource_satisfies::<LabDiagnostics>(
        &format!("active leaf is '{expected}'"),
        move |d| d.active_leaf_name == expected,
    )
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// 1. Boot and verify agent spawns, machine initializes to Idle.
fn smoke_lab() -> Scenario {
    Scenario::builder("smoke_lab")
        .description("Agent spawns, machine initializes to Idle")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::entity_exists::<LabAgent>("agent spawned"))
        .then(assertions::resource_exists::<LabDiagnostics>("diagnostics present"))
        .then(assert_state("Idle"))
        .then(inspect::log_resource::<LabDiagnostics>("smoke diagnostics"))
        .then(Action::Screenshot("smoke_lab".into()))
        .then(assertions::log_summary("smoke_lab"))
        .build()
}

/// 2. Idle -> Patrol after 2.5s timer.
fn basic_cycling() -> Scenario {
    Scenario::builder("basic_cycling")
        .description("Timer-based Idle->Patrol transition fires after 2.5s")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("cycling_idle".into()))
        // Wait ~1.7s — should still be Idle
        .then(Action::WaitFrames(100))
        .then(assert_state("Idle"))
        // Now wait for Patrol (generous timeout)
        .then(wait_for_state("Patrol", 200))
        .then(assert_state("Patrol"))
        .then(inspect::log_resource::<LabDiagnostics>("after cycling"))
        .then(Action::Screenshot("cycling_patrol".into()))
        .then(assertions::log_summary("basic_cycling"))
        .build()
}

/// 3. Enter compound Combat state, verify Chase as nested leaf.
fn hierarchical() -> Scenario {
    Scenario::builder("hierarchical")
        .description("Enter/exit compound Combat state with nested Chase")
        .then(take_control())
        // Wait for Idle->Patrol
        .then(wait_for_state("Patrol", 300))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("hierarchical_patrol".into()))
        // Enter combat by setting target_visible
        .then(set_blackboard(true, false))
        .then(wait_for_state("Chase", 60))
        .then(assert_state("Chase"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "in compound state",
            |d| d.is_in_compound_state,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "path includes Combat",
            |d| d.active_path_names.iter().any(|n| n == "Combat"),
        ))
        .then(Action::Screenshot("hierarchical_combat".into()))
        // Exit combat
        .then(set_blackboard(false, false))
        .then(wait_for_state("Patrol", 60))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("hierarchical_patrol_resume".into()))
        .then(assertions::log_summary("hierarchical"))
        .build()
}

/// 4. Push Stunned via signal, verify stack, wait for pop after 3s.
fn pushdown_stun() -> Scenario {
    Scenario::builder("pushdown_stun")
        .description("SIGNAL_STUN pushes Stunned, pops after 3s")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack empty before stun",
            |d| d.stack_depth == 0,
        ))
        .then(Action::Screenshot("pushdown_before".into()))
        // Ensure target not visible so we stay in Patrol, then stun
        .then(set_blackboard(false, false))
        .then(Action::WaitFrames(5))
        .then(send_stun())
        .then(wait_for_state("Stunned", 30))
        .then(assert_state("Stunned"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack has 1 frame",
            |d| d.stack_depth >= 1,
        ))
        .then(Action::Screenshot("pushdown_stunned".into()))
        // Wait for pop (3.0s = 180 frames + generous buffer)
        .then(Action::WaitUntil {
            label: "wait for stun pop".into(),
            condition: Box::new(|world| {
                world.resource::<LabDiagnostics>().active_leaf_name != "Stunned"
            }),
            max_frames: 300,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "no longer stunned",
            |d| d.active_leaf_name != "Stunned",
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack empty after pop",
            |d| d.stack_depth == 0,
        ))
        .then(Action::Screenshot("pushdown_popped".into()))
        .then(inspect::log_resource::<LabDiagnostics>("after pop"))
        .then(assertions::log_summary("pushdown_stun"))
        .build()
}

/// 5. Guard-gated Chase <-> Attack transitions.
fn guard_transitions() -> Scenario {
    Scenario::builder("guard_transitions")
        .description("Visibility and attack range guards drive Chase<->Attack")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        // Enter combat -> Chase
        .then(set_blackboard(true, false))
        .then(wait_for_state("Chase", 60))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("guard_chase".into()))
        // Enter attack range -> Attack
        .then(set_blackboard(true, true))
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("guard_attack".into()))
        // Leave attack range — Attack has min_active 1.0s + Pending mode
        .then(set_blackboard(true, false))
        .then(wait_for_state("Chase", 120))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("guard_back_to_chase".into()))
        .then(assertions::log_summary("guard_transitions"))
        .build()
}

/// 6. Timer-gated Idle->Patrol fires at correct time, not early.
fn delayed_transitions() -> Scenario {
    Scenario::builder("delayed_transitions")
        .description("Transition fires at correct time (2.5s), not before")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        // At ~1.5s — still Idle
        .then(Action::WaitFrames(60))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("delayed_still_idle".into()))
        // Now wait for Patrol with generous timeout
        .then(wait_for_state("Patrol", 200))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("delayed_now_patrol".into()))
        .then(assertions::log_summary("delayed_transitions"))
        .build()
}

/// 7. Debug annotations present on agent with correct counts.
fn debug_annotations() -> Scenario {
    Scenario::builder("debug_annotations")
        .description("AiDebugAnnotations present with correct circle/line/path counts")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "has debug annotations",
            |d| d.has_debug_annotations,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "2 circles",
            |d| d.annotation_circle_count == 2,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "1 line",
            |d| d.annotation_line_count == 1,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "1 path",
            |d| d.annotation_path_count == 1,
        ))
        .then(Action::Screenshot("debug_annotations".into()))
        .then(assertions::log_summary("debug_annotations"))
        .build()
}

/// 8. Deep history: re-entering Combat restores Attack (not Chase).
fn history_restore() -> Scenario {
    Scenario::builder("history_restore")
        .description("Deep history restores Attack on Combat re-entry")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        // Enter combat + attack range -> Attack
        .then(set_blackboard(true, true))
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        // Wait past min_active to stabilize
        .then(Action::WaitFrames(70))
        .then(Action::Screenshot("history_in_attack".into()))
        // Leave combat entirely
        .then(set_blackboard(false, false))
        .then(wait_for_state("Patrol", 60))
        .then(assert_state("Patrol"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "history saved",
            |d| d.history_snapshot_count >= 1,
        ))
        // Re-enter combat — history should restore Attack, not Chase
        .then(set_blackboard(true, true))
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("history_restored".into()))
        .then(inspect::log_resource::<LabDiagnostics>("after history restore"))
        .then(assertions::log_summary("history_restore"))
        .build()
}

/// 9. Trace buffer records transition events.
fn trace_recording() -> Scenario {
    Scenario::builder("trace_recording")
        .description("Trace entries recorded for state transitions")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace has initial entries",
            |d| d.trace_entry_count >= 1,
        ))
        // Wait for Idle->Patrol transition
        .then(wait_for_state("Patrol", 300))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace grew after transition",
            |d| d.trace_entry_count >= 3,
        ))
        .then(inspect::log_resource::<LabDiagnostics>("trace state"))
        .then(Action::Screenshot("trace_after_transitions".into()))
        .then(assertions::log_summary("trace_recording"))
        .build()
}

/// 10. Drive through ALL states: Idle -> Patrol -> Chase -> Attack -> Stunned -> pop -> Patrol.
fn full_lifecycle() -> Scenario {
    Scenario::builder("full_lifecycle")
        .description("Drive through all states end-to-end")
        .then(take_control())
        // Phase 1: Idle
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("lifecycle_idle".into()))
        // Phase 2: Idle -> Patrol
        .then(wait_for_state("Patrol", 300))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("lifecycle_patrol".into()))
        // Phase 3: Patrol -> Combat/Chase
        .then(set_blackboard(true, false))
        .then(wait_for_state("Chase", 60))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("lifecycle_chase".into()))
        // Phase 4: Chase -> Attack
        .then(set_blackboard(true, true))
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("lifecycle_attack".into()))
        // Phase 5: Push Stunned (wait past min_active first)
        .then(Action::WaitFrames(70))
        .then(send_stun())
        .then(wait_for_state("Stunned", 30))
        .then(assert_state("Stunned"))
        .then(Action::Screenshot("lifecycle_stunned".into()))
        // Phase 6: Pop after 3s
        .then(Action::WaitUntil {
            label: "wait for stun pop".into(),
            condition: Box::new(|world| {
                world.resource::<LabDiagnostics>().active_leaf_name != "Stunned"
            }),
            max_frames: 300,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "popped from stun",
            |d| d.active_leaf_name != "Stunned",
        ))
        .then(Action::Screenshot("lifecycle_restored".into()))
        // Phase 7: Leave combat
        .then(set_blackboard(false, false))
        .then(wait_for_state("Patrol", 60))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("lifecycle_back_to_patrol".into()))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "many transitions occurred",
            |d| d.runtime_revision >= 6,
        ))
        .then(inspect::log_resource::<LabDiagnostics>("final state"))
        .then(assertions::log_summary("full_lifecycle"))
        .build()
}
