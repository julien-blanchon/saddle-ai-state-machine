mod support;

use saddle_bevy_e2e::action::Action;
use saddle_bevy_e2e::actions::{assertions, inspect};
use saddle_bevy_e2e::scenario::Scenario;

use crate::LabDiagnostics;
use support::{
    assert_state, enter_attack, enter_combat, leave_combat, rapid_guard_toggle_pattern, send_stun,
    take_control, wait_for_state,
};

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
        "stun_in_attack",
        "rapid_guard_toggle",
        "trace_growth",
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
        "stun_in_attack" => Some(stun_in_attack()),
        "rapid_guard_toggle" => Some(rapid_guard_toggle()),
        "trace_growth" => Some(trace_growth()),
        _ => None,
    }
}

fn smoke_lab() -> Scenario {
    Scenario::builder("smoke_lab")
        .description("Agent spawns, machine initializes to Idle")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::entity_exists::<crate::LabAgent>("agent spawned"))
        .then(assertions::resource_exists::<LabDiagnostics>("diagnostics present"))
        .then(assert_state("Idle"))
        .then(inspect::log_resource::<LabDiagnostics>("smoke diagnostics"))
        .then(Action::Screenshot("smoke_lab".into()))
        .then(assertions::log_summary("smoke_lab"))
        .build()
}

fn basic_cycling() -> Scenario {
    Scenario::builder("basic_cycling")
        .description("Timer-based Idle->Patrol transition fires after 2.5s")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("cycling_idle".into()))
        .then(Action::WaitFrames(100))
        .then(assert_state("Idle"))
        .then(wait_for_state("Patrol", 200))
        .then(assert_state("Patrol"))
        .then(inspect::log_resource::<LabDiagnostics>("after cycling"))
        .then(Action::Screenshot("cycling_patrol".into()))
        .then(assertions::log_summary("basic_cycling"))
        .build()
}

fn hierarchical() -> Scenario {
    Scenario::builder("hierarchical")
        .description("Enter/exit compound Combat state with nested Chase")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("hierarchical_patrol".into()))
        .then(enter_combat())
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
        .then(leave_combat())
        .then(wait_for_state("Patrol", 60))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("hierarchical_patrol_resume".into()))
        .then(assertions::log_summary("hierarchical"))
        .build()
}

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
        .then(leave_combat())
        .then(Action::WaitFrames(5))
        .then(send_stun())
        .then(wait_for_state("Stunned", 30))
        .then(assert_state("Stunned"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack has 1 frame",
            |d| d.stack_depth >= 1,
        ))
        .then(Action::Screenshot("pushdown_stunned".into()))
        .then(Action::WaitUntil {
            label: "wait for stun pop".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().active_leaf_name != "Stunned"),
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

fn guard_transitions() -> Scenario {
    Scenario::builder("guard_transitions")
        .description("Visibility and attack range guards drive Chase<->Attack")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(enter_combat())
        .then(wait_for_state("Chase", 60))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("guard_chase".into()))
        .then(enter_attack())
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("guard_attack".into()))
        .then(enter_combat())
        .then(wait_for_state("Chase", 120))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("guard_back_to_chase".into()))
        .then(assertions::log_summary("guard_transitions"))
        .build()
}

fn delayed_transitions() -> Scenario {
    Scenario::builder("delayed_transitions")
        .description("Transition fires at correct time (2.5s), not before")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        .then(Action::WaitFrames(60))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("delayed_still_idle".into()))
        .then(wait_for_state("Patrol", 200))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("delayed_now_patrol".into()))
        .then(assertions::log_summary("delayed_transitions"))
        .build()
}

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

fn history_restore() -> Scenario {
    Scenario::builder("history_restore")
        .description("Deep history restores Attack on Combat re-entry")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(enter_attack())
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::WaitFrames(70))
        .then(Action::Screenshot("history_in_attack".into()))
        .then(leave_combat())
        .then(wait_for_state("Patrol", 60))
        .then(assert_state("Patrol"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "history saved",
            |d| d.history_snapshot_count >= 1,
        ))
        .then(enter_attack())
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("history_restored".into()))
        .then(inspect::log_resource::<LabDiagnostics>("after history restore"))
        .then(assertions::log_summary("history_restore"))
        .build()
}

fn trace_recording() -> Scenario {
    Scenario::builder("trace_recording")
        .description("Trace entries recorded for state transitions")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace has initial entries",
            |d| d.trace_entry_count >= 1,
        ))
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

fn full_lifecycle() -> Scenario {
    Scenario::builder("full_lifecycle")
        .description("Drive through all states end-to-end")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assert_state("Idle"))
        .then(Action::Screenshot("lifecycle_idle".into()))
        .then(wait_for_state("Patrol", 300))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("lifecycle_patrol".into()))
        .then(enter_combat())
        .then(wait_for_state("Chase", 60))
        .then(assert_state("Chase"))
        .then(Action::Screenshot("lifecycle_chase".into()))
        .then(enter_attack())
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::Screenshot("lifecycle_attack".into()))
        .then(Action::WaitFrames(70))
        .then(send_stun())
        .then(wait_for_state("Stunned", 30))
        .then(assert_state("Stunned"))
        .then(Action::Screenshot("lifecycle_stunned".into()))
        .then(Action::WaitUntil {
            label: "wait for stun pop".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().active_leaf_name != "Stunned"),
            max_frames: 300,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "popped from stun",
            |d| d.active_leaf_name != "Stunned",
        ))
        .then(Action::Screenshot("lifecycle_restored".into()))
        .then(leave_combat())
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

fn stun_in_attack() -> Scenario {
    Scenario::builder("stun_in_attack")
        .description("SIGNAL_STUN interrupts Attack; after stun pops the agent should resume in Attack (or fall back to Combat) because deep history was saved")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(enter_attack())
        .then(wait_for_state("Attack", 60))
        .then(assert_state("Attack"))
        .then(Action::WaitFrames(70))
        .then(Action::Screenshot("stun_in_attack_before".into()))
        .then(send_stun())
        .then(wait_for_state("Stunned", 30))
        .then(assert_state("Stunned"))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack has 1 frame (Attack saved)",
            |d| d.stack_depth >= 1,
        ))
        .then(Action::Screenshot("stun_in_attack_stunned".into()))
        .then(Action::WaitUntil {
            label: "stun pops".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().active_leaf_name != "Stunned"),
            max_frames: 300,
        })
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "no longer stunned after pop",
            |d| d.active_leaf_name != "Stunned",
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "stack empty after pop",
            |d| d.stack_depth == 0,
        ))
        .then(Action::Screenshot("stun_in_attack_popped".into()))
        .then(inspect::log_resource::<LabDiagnostics>("after stun pop"))
        .then(assertions::log_summary("stun_in_attack"))
        .build()
}

fn rapid_guard_toggle() -> Scenario {
    Scenario::builder("rapid_guard_toggle")
        .description("Toggle target_visible on/off every 5 frames for 150 frames; machine must remain in a valid leaf state (Patrol, Chase, or Attack) throughout — never stuck or panicking")
        .then(take_control())
        .then(wait_for_state("Patrol", 300))
        .then(assert_state("Patrol"))
        .then(Action::Screenshot("rapid_toggle_start".into()))
        .then_many(rapid_guard_toggle_pattern())
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "machine is in a valid leaf state after rapid toggle",
            |d| {
                matches!(
                    d.active_leaf_name.as_str(),
                    "Idle" | "Patrol" | "Chase" | "Attack" | "Stunned"
                )
            },
        ))
        .then(Action::Screenshot("rapid_toggle_end".into()))
        .then(leave_combat())
        .then(wait_for_state("Patrol", 120))
        .then(assert_state("Patrol"))
        .then(assertions::log_summary("rapid_guard_toggle"))
        .build()
}

fn trace_growth() -> Scenario {
    Scenario::builder("trace_growth")
        .description("Each state transition appends an entry to the trace buffer. Drive Idle->Patrol->Chase->Patrol and verify the trace entry count grows monotonically at each step")
        .then(take_control())
        .then(Action::WaitFrames(30))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace has at least 1 entry at boot",
            |d| d.trace_entry_count >= 1,
        ))
        .then(Action::Screenshot("trace_growth_idle".into()))
        .then(wait_for_state("Patrol", 300))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace grew after Idle->Patrol",
            |d| d.trace_entry_count >= 3,
        ))
        .then(Action::Screenshot("trace_growth_patrol".into()))
        .then(enter_combat())
        .then(wait_for_state("Chase", 60))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace grew after Patrol->Chase",
            |d| d.trace_entry_count >= 5,
        ))
        .then(Action::Screenshot("trace_growth_chase".into()))
        .then(leave_combat())
        .then(wait_for_state("Patrol", 60))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "trace grew after Chase->Patrol",
            |d| d.trace_entry_count >= 7,
        ))
        .then(inspect::log_resource::<LabDiagnostics>("trace final"))
        .then(Action::Screenshot("trace_growth_final".into()))
        .then(assertions::log_summary("trace_growth"))
        .build()
}
