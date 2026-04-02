# AI State Machine Lab

Crate-local standalone lab app for manually inspecting the shared `saddle-ai-state-machine` crate in a real Bevy application.

## Purpose

- verify shared-crate integration in a real app
- inspect runtime state, trace buffers, and blackboards in a live scene
- exercise hierarchy, push interrupts, delayed transitions, and debug gizmos together

## Status

Working

## Run

```bash
cargo run -p saddle-ai-state-machine-lab
```

## Notes

- The lab intentionally keeps the scene generic and avoids any project-specific gameplay types.
