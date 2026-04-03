# Configuration

This document lists the main tuning points exposed by `saddle-ai-state-machine` in v0.1.

## Plugin Schedules

Use `AiStateMachinePlugin::always_on(update_schedule)` when the machine should stay active for the entire app lifetime. Use `AiStateMachinePlugin::new(activate, deactivate, update)` when the machine should be tied to explicit state entry/exit schedules.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `activate_schedule` | `Interned<dyn ScheduleLabel>` | none | Schedule where inactive instances can be reactivated |
| `deactivate_schedule` | `Interned<dyn ScheduleLabel>` | none | Schedule where instances are marked inactive |
| `update_schedule` | `Interned<dyn ScheduleLabel>` | none | Schedule that runs the full machine pipeline |

## `StateMachineInstanceConfig`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `evaluation_mode` | `StateMachineEvaluationMode` | `EveryFrame` | `OnSignalOrBlackboardChange` skips transition evaluation until a queued signal or blackboard revision change wakes the instance |
| `enabled_regions` | `Vec<RegionId>` | empty | Empty means all regions are enabled. Otherwise only listed regions enter/update. |
| `trace_config.capacity` | `usize` | `32` | Maximum number of trace entries stored per instance |
| `trace_config.record_blocked` | `bool` | `true` | Controls whether blocked transitions are recorded in the trace |
| `max_internal_steps` | `usize` | `16` | Maximum chained internal transitions allowed in one update before the runtime aborts the chain |
| `max_stack_depth` | `usize` | `8` | Maximum push depth for stack-based interrupts |
| `blackboard_overrides` | `Vec<InstanceBlackboardOverride>` | empty | Per-instance startup values applied after schema defaults and before first transition evaluation |
| `utility_threshold_overrides` | `Vec<InstanceThresholdOverride>` | empty | Per-instance transition score thresholds overriding the definition policy |

## Blackboard Schema

Blackboard keys are declared by the builder:

| Builder Input | Meaning |
| --- | --- |
| `name` | Human-readable debug name |
| `value_type` | Required runtime value type |
| `required` | Documents that the key should exist for this machine |
| `default_value` | Optional initial value installed when the schema is applied |

Schema effects in v0.1:

- key IDs are stable within the built definition
- `Blackboard::ensure_schema` installs defaults and declared types
- `Blackboard::set` rejects writes with the wrong value type once the schema is known
- revision counters and dirty keys update only when the stored value actually changes

## State-Level Tuning

| Builder API | Default | Effect |
| --- | --- | --- |
| `set_state_history_mode` | `HistoryMode::None` | Enables shallow or deep history restore for compound/parallel states |
| `set_state_min_active_seconds` | `0.0` | Minimum dwell time before the state may exit |
| `set_state_exit_guard` | `None` | Guard callback that must allow exiting the state |
| `set_state_always_tick` | `false` | Reserved for future sleep/wake work; currently documentation-level only |
| `add_state_tag` | none | Adds generic tags for consumer-side categorization/debugging |
| `add_on_enter` / `add_on_update` / `add_on_exit` | none | Registers action callbacks for lifecycle hooks |

## Transition-Level Tuning

| Builder API | Default | Effect |
| --- | --- | --- |
| `with_trigger` | `Automatic` | Sets `Automatic`, `AfterSeconds`, `Signal`, or `Done` trigger behavior |
| `with_signal` | none | Convenience wrapper for `TransitionTrigger::Signal` |
| `with_guard` | none | Guard callback that must succeed before the transition can fire |
| `with_action` | none | Transition-side effect executed after exits and before enters |
| `with_priority` | `0` | Higher numbers win before lower numbers |
| `with_scorer` | none | Enables utility-scored arbitration for the transition |
| `with_mode` | `TransitionMode::Immediate` | `Pending` waits for the source to become exit-ready; `Force` bypasses pending semantics |
| `with_cooldown` | `0.0` | Blocks refiring until the cooldown elapses |
| `with_debounce` | `0.0` | Requires the guard to stay true for long enough before the transition is allowed |

## Utility Policy

| Variant | Effect |
| --- | --- |
| `Disabled` | No scorer threshold is applied |
| `BestScore` | Higher score wins among otherwise valid candidates |
| `BestScoreAbove { minimum_score_milli }` | Higher score wins, but only if it clears the threshold |

Per-instance overrides via `InstanceThresholdOverride` let one entity require stricter or looser thresholds than the shared definition.

## Debug Visualization

`AiDebugAnnotations` carries optional line-based geometry:

| Field | Effect |
| --- | --- |
| `circles` | Detection / influence / range discs |
| `lines` | Target links, aim vectors, or one-off direction hints |
| `paths` | Waypoint or route polylines |

The crate registers a custom `GizmoConfigGroup` named `AiDebugGizmos`, so consumers can tune visibility, width, and depth bias independently from their default gizmos.

## Message Surface

| Message | When it fires |
| --- | --- |
| `StateMachineSignal` | Consumer-written inbox message that queues a signal on a target entity during `IntakeSignals` |
| `StateEntered` | After a state is entered during transition execution |
| `StateExited` | After a state exits during transition execution |
| `TransitionTriggered` | When a transition is successfully applied |
| `TransitionBlocked` | When the best deterministic candidate is blocked; trace config only controls whether the blocked result is stored in the instance trace |

## Asset Loading

`StateMachineDefinitionAssetLoader` registers the `.fsm.ron` extension and produces `StateMachineDefinitionAsset`.

| Type | Effect |
| --- | --- |
| `StateMachineDefinitionAsset::definition` | Serializable machine definition payload |
| `StateMachineDefinitionAsset::register(...)` | Validates and inserts the loaded definition into `StateMachineLibrary` |

## Deferred Tuning Areas

These are intentionally deferred in v0.1 and therefore not configurable yet:

- sleep / wake scheduling
- scoped blackboards
- marker mirroring of active states
- score hysteresis helpers
- text-based debug overlays as crate runtime features
