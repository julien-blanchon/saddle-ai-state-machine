use super::*;

#[test]
fn trace_respects_capacity() {
    let mut trace = StateMachineTrace::new(DebugTraceConfig {
        capacity: 2,
        record_blocked: true,
    });

    trace.push(StateMachineTraceEntry {
        frame_revision: 1,
        runtime_revision: 1,
        kind: TraceKind::EnteredState(crate::definition::StateId(0)),
    });
    trace.push(StateMachineTraceEntry {
        frame_revision: 2,
        runtime_revision: 2,
        kind: TraceKind::EnteredState(crate::definition::StateId(1)),
    });
    trace.push(StateMachineTraceEntry {
        frame_revision: 3,
        runtime_revision: 3,
        kind: TraceKind::EnteredState(crate::definition::StateId(2)),
    });

    assert_eq!(trace.entries.len(), 2);
    assert_eq!(trace.entries[0].frame_revision, 2);
    assert_eq!(trace.entries[1].frame_revision, 3);
}
