use super::*;

#[test]
fn stack_enforces_max_depth() {
    let mut stack = StateStack::new(1);
    assert!(stack.push(StateStackFrame::default()));
    assert!(!stack.push(StateStackFrame::default()));
    assert_eq!(stack.len(), 1);
}

#[test]
fn pop_restores_last_frame() {
    let mut stack = StateStack::new(2);
    let frame = StateStackFrame {
        pending_transition: Some(crate::definition::TransitionId(9)),
        ..default()
    };
    stack.push(frame.clone());

    assert_eq!(stack.pop(), Some(frame));
    assert!(stack.is_empty());
}
