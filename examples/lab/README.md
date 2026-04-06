# AI State Machine Lab

Crate-local standalone lab app for manually inspecting the shared `saddle-ai-state-machine` crate in a real Bevy application.

## Purpose

- verify shared-crate integration in a real app
- inspect runtime state, trace buffers, and blackboards in a live scene
- exercise hierarchy, push interrupts, delayed transitions, and debug gizmos together
- run automated E2E scenarios to validate every feature area

## Status

Working

## Run

```bash
cargo run -p saddle-ai-state-machine-lab
```

## E2E Scenarios

Run any scenario:

```bash
cargo run -p saddle-ai-state-machine-lab --features e2e -- <scenario_name>
```

Handoff mode (keeps the game running after scenario completes):

```bash
cargo run -p saddle-ai-state-machine-lab --features e2e -- <scenario_name> --handoff
```

Output lands in `e2e_output/<scenario_name>/` with screenshots and `log.txt`.

### Available Scenarios

| Scenario | Feature Area | Assertions | Description |
|---|---|---|---|
| `smoke_lab` | Boot | 3 | Agent spawns, machine initializes to Idle |
| `basic_cycling` | Timers | 3 | Idle->Patrol after 2.5s timer, not before |
| `hierarchical` | Compound states | 5 | Enter/exit Combat, Chase as nested leaf |
| `pushdown_stun` | Push/pop | 5 | SIGNAL_STUN pushes Stunned, pops after 3s |
| `guard_transitions` | Guards | 3 | Visibility/attack guards drive Chase<->Attack |
| `delayed_transitions` | Timing | 3 | Timer-gated transition fires at correct time |
| `debug_annotations` | Debug gizmos | 4 | AiDebugAnnotations present with correct counts |
| `history_restore` | Deep history | 4 | Re-entering Combat restores Attack (not Chase) |
| `trace_recording` | Trace buffer | 2 | Trace entries recorded for transitions |
| `full_lifecycle` | Integration | 8 | Drive through ALL states end-to-end (7 screenshots) |

### Run All Scenarios

```bash
for s in smoke_lab basic_cycling hierarchical pushdown_stun guard_transitions \
         delayed_transitions debug_annotations history_restore trace_recording \
         full_lifecycle; do
    echo "=== $s ==="
    cargo run -p saddle-ai-state-machine-lab --features e2e -- "$s" 2>&1 \
        | grep "\[assertions\]" | head -1
done
```

## Notes

- The lab intentionally keeps the scene generic and avoids any project-specific gameplay types.
- E2E scenarios use `saddle-bevy-e2e` (feature-gated behind `--features e2e`).
- Normal `cargo run` without the `e2e` feature runs the interactive lab as usual.
