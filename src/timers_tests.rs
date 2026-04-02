use super::*;

#[test]
fn tick_and_decay_helpers_work() {
    let mut values = vec![0.0, 0.0, 0.0];
    tick_active(&mut values, [0_usize, 2], 0.5);
    assert_eq!(values, vec![0.5, 0.0, 0.5]);

    decay_toward_zero(&mut values, 0.25);
    assert_eq!(values, vec![0.25, 0.0, 0.25]);
}
