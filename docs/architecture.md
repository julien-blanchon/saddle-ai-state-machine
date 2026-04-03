# Architecture

`saddle-ai-state-machine` uses a split design:

- Static definition:
  reusable machine structure, hierarchy, regions, transition graph, declaration order, blackboard schema, and debug defaults.
- Runtime instance:
  entity-local active states, timers, stack, pending transitions, queued signals, blackboard contents, and recent trace entries.

This keeps authoring data reusable across many entities while runtime state stays compact and serializable.

## Update Pipeline

```text
IntakeSignals
    -> AdvanceTimers
    -> EvaluateTransitions
    -> ExecuteTransitions
    -> UpdateStates
    -> DebugVisualize
```

The pipeline is explicit on purpose:

- `IntakeSignals`
  queues `StateMachineSignal` messages onto instances, aligns runtime storage with the definition schema, and applies per-instance blackboard overrides before first initialization.
- `AdvanceTimers`
  advances active-state elapsed timers and transition cooldown timers.
- `EvaluateTransitions`
  computes deterministic winners or blocked reasons without mutating the machine.
- `ExecuteTransitions`
  applies the winning transition, runs hooks, emits messages, and appends trace entries.
- `UpdateStates`
  runs active-state `on_update` hooks after topology changes settle.
- `DebugVisualize`
  draws line-based annotations from `AiDebugAnnotations` using `AiDebugGizmos`.

## Deterministic Arbitration

For every active machine, candidate transitions are ranked by:

1. Deepest active source state first
2. Higher explicit priority first
3. Higher utility score first
4. Earlier declaration order last

Any-state transitions participate in the same deterministic policy. Runtime evaluation never relies on `HashMap` iteration order.

Blocked candidates keep a reason when debug tracing is enabled:

- guard false
- utility below threshold
- cooldown active
- debounce active
- exit not ready
- pending exit
- stack empty / stack overflow
- invalid transition semantics

## Hierarchy And Regions

- Atomic states own no child regions.
- Compound states enter their initial child region state, or restore history if configured.
- Parallel states activate all enabled child regions together.
- Child transitions preempt parent transitions because depth wins arbitration.
- Depth outranks priority and utility, so a valid deeper transition wins over a parent transition in the same frame. Parent transitions still fire when no deeper candidate wins.

Parallel-region cross transitions that would jump between sibling root regions of the same parallel state are explicitly rejected by validation in v0.1.

## Stack Semantics

`TransitionOperation` controls topology changes:

- `Replace`
  exits the affected active path and enters the new target path.
- `Push`
  saves the current active regions/history/timers on the stack, clears the active state, then enters the target.
- `Pop`
  exits the current pushed state and restores the previous frame.

The runtime enforces `max_stack_depth` during transition evaluation so overflow becomes an explicit blocked-transition reason instead of a silent runtime failure.

## History Restore

History is recorded when states exit:

- `HistoryMode::Shallow`
  restores the last direct child per region.
- `HistoryMode::Deep`
  restores recorded leaf-region activity under the exited subtree.

History only applies to compound or parallel parents. Validation rejects history configuration on atomic/final/transient states.

## Delayed And Done Transitions

- `TransitionTrigger::AfterSeconds`
  checks the source state's elapsed time.
- Leaving a state discards the active path that the timer belonged to, so the timer effectively stops applying.
- `TransitionTrigger::Done`
  becomes valid when every active child region of the source compound/parallel state reaches a final state.

Transient states are supported, and validation detects unconditional transient-only cycles that would never quiesce.

## Blackboard Model

The blackboard is intentionally simple:

- keys are declared in the machine definition
- keys resolve to stable dense IDs at build time
- values use a compact `BlackboardValue` enum for hot-path reads
- writes bump a revision counter and track dirty keys
- schema-aware writes reject mismatched value types once the schema is installed

Supported value types in v0.1:

- `f32`
- `i32`
- `bool`
- `Entity`
- `Vec2`
- `Vec3`
- `String`

## Asset Definitions

Machine definitions can now enter the runtime through `StateMachineDefinitionAssetLoader` as well as pure Rust builders. Loaded assets still register into `StateMachineLibrary`, so the runtime keeps one definition path after load time: built and asset-authored machines share the same validation, IDs, and execution pipeline.

## Callbacks

Guard, action, and scorer callbacks are registered once in `StateMachineCallbacks` and referenced by stable IDs from the definition:

- Guards:
  read world state and decide whether a transition is valid
- Actions:
  perform world-facing side effects on enter, update, exit, or transition
- Scorers:
  compute utility scores without changing machine topology

This keeps definitions data-driven without string dispatch in hot loops.

## Trace And Debug Data

Every instance carries a `StateMachineTrace` with bounded capacity. The trace records:

- entered states
- exited states
- triggered transitions
- pending transitions waiting on exit readiness
- blocked transitions

The runtime data needed for BRP/save-load stays directly on `StateMachineInstance` and `Blackboard`, so inspecting:

- active path
- active leaf states
- stack contents
- pending transition
- queued signals
- blackboard values
- recent trace entries

does not require hidden editor-only state.
