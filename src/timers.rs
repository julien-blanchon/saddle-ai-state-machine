pub fn tick_active(
    values: &mut [f32],
    active_indices: impl IntoIterator<Item = usize>,
    delta: f32,
) {
    for index in active_indices {
        if let Some(value) = values.get_mut(index) {
            *value += delta;
        }
    }
}

pub fn decay_toward_zero(values: &mut [f32], delta: f32) {
    for value in values {
        *value = (*value - delta).max(0.0);
    }
}

#[cfg(test)]
#[path = "timers_tests.rs"]
mod tests;
