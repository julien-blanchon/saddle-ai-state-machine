# Saddle AI State Machine

Reusable hierarchical finite state machine / statechart runtime for Bevy.

The crate is generic on purpose. It can drive enemy AI, companion behaviors, locomotion layers, interaction flows, dialogue/cutscene sequencing, UI workflows, and other entity-level orchestration without importing any project-specific types.

For simple apps that keep machines live for the full app lifetime, prefer `AiStateMachinePlugin::always_on(Update)`. Use `AiStateMachinePlugin::new(...)` when you need explicit activation/deactivation schedules such as `OnEnter` / `OnExit`.

## Quick Start

```toml
[dependencies]
saddle-ai-state-machine = { git = "https://github.com/julien-blanchon/saddle-ai-state-machine" }
```

```rust
use saddle_ai_state_machine::*;
use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Running,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .add_plugins(AiStateMachinePlugin::new(
            OnEnter(DemoState::Running),
            OnExit(DemoState::Running),
            Update,
        ))
        .add_systems(Startup, setup_machine)
        .run();
}

fn setup_machine(mut commands: Commands, mut library: ResMut<StateMachineLibrary>) {
    let mut builder = StateMachineBuilder::new("basic");
    let root = builder.root_region("root");
    let idle = builder.atomic_state("Idle");
    let move_state = builder.atomic_state("Move");
    builder
        .add_state_to_region(idle, root)
        .add_state_to_region(move_state, root)
        .set_region_initial(root, idle)
        .add_transition(
            TransitionDefinition::replace(idle, move_state)
                .with_trigger(TransitionTrigger::after_seconds(1.0)),
        );

    let definition_id = library.register(builder.build().unwrap()).unwrap();
    commands.spawn(StateMachineInstance::new(definition_id));
}
```

## Public API

- Plugin: `AiStateMachinePlugin`
- Components: `StateMachineInstance`, `Blackboard`, `AiDebugAnnotations`
- Resources: `StateMachineLibrary`, `StateMachineCallbacks`
- Builder / definition types: `StateMachineBuilder`, `StateMachineDefinition`, `TransitionDefinition`
- Instance config helpers: `InstanceBlackboardOverride`, `InstanceThresholdOverride`
- System sets: `AiStateMachineSystems::{IntakeSignals, AdvanceTimers, EvaluateTransitions, ExecuteTransitions, UpdateStates, DebugVisualize}`
- Messages: `StateMachineSignal`, `StateEntered`, `StateExited`, `TransitionTriggered`, `TransitionBlocked`
- Debug types: `StateMachineTrace`, `DebugTraceConfig`, `AiDebugGizmos`

## Semantic Guarantees

- Deterministic transition arbitration:
  deepest active child first, then parents, then higher priority, then higher utility score, then declaration order.
- Stable IDs:
  state, region, transition, signal, action, guard, and scorer names resolve to dense IDs at build time.
- Explicit update pipeline:
  `IntakeSignals -> AdvanceTimers -> EvaluateTransitions -> ExecuteTransitions -> UpdateStates -> DebugVisualize`.
- External signal inbox:
  consumers can queue `StateMachineSignal` messages instead of mutating instances directly.
- Exit/action ordering:
  state exit hooks run first, transition actions run next, state enter hooks run last.
- Pending transitions:
  `TransitionMode::Pending` reports a blocked transition as waiting for the source state to become exit-ready instead of force-breaking the state immediately.
- History behavior:
  compound and parallel parents may restore shallow or deep history for child regions when re-entered.

## What This Crate Is

- A data-defined HFSM / statechart runtime with reusable static machine definitions
- A Bevy-native runtime that exposes reflected state for BRP/debugging/save-load
- A shared crate with injectable schedules and no project-specific state or schedule dependency

## What This Crate Is Not

- A project-specific enemy AI crate
- A behavior tree, GOAP, or navmesh solution
- A thin wrapper over a third-party gameplay runtime
- A text-rendering debug inspector; gizmos stay line-based, and text overlays live in examples/UI

## Supported v0.1 Features

- Hierarchy and compound states
- Parallel/orthogonal regions
- Push / replace / pop stack semantics
- Delayed `after` transitions
- Any-state transitions
- Utility-scored transitions
- RON asset loading through `StateMachineDefinitionAsset` and `StateMachineDefinitionAssetLoader`
- Typed blackboards with schema-aware writes and revision tracking
- Per-instance blackboard default overrides
- Event-driven evaluation with `StateMachineEvaluationMode::OnSignalOrBlackboardChange`
- Reflection-backed runtime inspection and serialization
- Trace buffers and blocked-transition reporting

Deferred in v0.1:

- Sleep / wake scheduling
- Marker-component mirroring of active states
- Scoped blackboards
- Score hysteresis helpers

## Examples

| Example | Description | Run |
| --- | --- | --- |
| `basic` | Minimal two-state machine with a delayed transition | `cargo run -p saddle-ai-state-machine-example-basic` |
| `hierarchical` | Parent/child hierarchy with compound-state entry | `cargo run -p saddle-ai-state-machine-example-hierarchical` |
| `pushdown` | Push interrupt + timed pop resume | `cargo run -p saddle-ai-state-machine-example-pushdown` |
| `utility` | Utility-scored transition arbitration | `cargo run -p saddle-ai-state-machine-example-utility` |
| `orthogonal_regions` | Two regions active under a parallel parent | `cargo run -p saddle-ai-state-machine-example-orthogonal-regions` |
| `delayed_transitions` | Explicit `after` timing and cancellation semantics | `cargo run -p saddle-ai-state-machine-example-delayed-transitions` |
| `debug_gizmos` | Custom gizmo group and line-based debug annotations | `cargo run -p saddle-ai-state-machine-example-debug-gizmos` |
| `debug_overlay` | Rich showcase with UI overlay, debug traces, push interrupts, and a moving target | `cargo run -p saddle-ai-state-machine-example-debug-overlay` |
| `save_load` | Reflection-backed instance + blackboard round-trip | `cargo run -p saddle-ai-state-machine-example-save-load` |
| `stress_10k` | Large-instance stress smoke for runtime stability | `cargo run -p saddle-ai-state-machine-example-stress-10k` |
| `layered_ai` | Batch-level integration demo: state machine + behavior tree + utility AI + GOAP in one sandbox | `cargo run -p saddle-ai-state-machine-example-layered-ai` |
| `lab` | Integration lab with E2E scenarios for automated feature validation | `cargo run -p saddle-ai-state-machine-lab` |

All windowed examples now expose live tuning through `saddle-pane`.

### E2E Testing

The lab includes 10 automated E2E scenarios that validate every feature area with screenshots and assertions. Run them with:

```bash
cargo run -p saddle-ai-state-machine-lab --features e2e -- <scenario_name>
```

Available: `smoke_lab`, `basic_cycling`, `hierarchical`, `pushdown_stun`, `guard_transitions`, `delayed_transitions`, `debug_annotations`, `history_restore`, `trace_recording`, `full_lifecycle`. See [lab/README.md](examples/lab/README.md) for details.

## Asset Loading

Definitions can be authored as RON assets and registered at runtime:

```rust
use bevy::prelude::*;
use saddle_ai_state_machine::{
    AiStateMachinePlugin, StateMachineDefinitionAsset, StateMachineLibrary,
};

fn register_loaded_machine(
    assets: Res<Assets<StateMachineDefinitionAsset>>,
    handle: Res<Handle<StateMachineDefinitionAsset>>,
    mut library: ResMut<StateMachineLibrary>,
) {
    if let Some(asset) = assets.get(handle.as_ref()) {
        let _definition_id = asset.register(&mut library).unwrap();
    }
}
```

## More Docs

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/configuration.md`](docs/configuration.md)
