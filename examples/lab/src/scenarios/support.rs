use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions};

use crate::{LabAgent, LabDiagnostics, LabKeys, SIGNAL_STUN, StateMachineLabPane};

pub(super) fn take_control() -> Action {
    Action::Custom(Box::new(|world| {
        let mut pane = world.resource_mut::<StateMachineLabPane>();
        pane.stun_interval = 999.0;
        pane.visibility_radius = 0.0;
        pane.attack_radius = 0.0;

        let keys = *world.resource::<LabKeys>();
        let mut query =
            world.query_filtered::<&mut saddle_ai_state_machine::Blackboard, With<LabAgent>>();
        for mut bb in query.iter_mut(world) {
            let _ = bb.set(keys.target_visible, false);
            let _ = bb.set(keys.in_attack_range, false);
        }
    }))
}

pub(super) fn set_blackboard(target_visible: bool, in_attack_range: bool) -> Action {
    Action::Custom(Box::new(move |world| {
        let mut pane = world.resource_mut::<StateMachineLabPane>();
        pane.visibility_radius = if target_visible { 999.0 } else { 0.0 };
        pane.attack_radius = if in_attack_range { 999.0 } else { 0.0 };

        let keys = *world.resource::<LabKeys>();
        let mut query =
            world.query_filtered::<&mut saddle_ai_state_machine::Blackboard, With<LabAgent>>();
        for mut bb in query.iter_mut(world) {
            let _ = bb.set(keys.target_visible, target_visible);
            let _ = bb.set(keys.in_attack_range, in_attack_range);
        }
    }))
}

pub(super) fn enter_combat() -> Action {
    set_blackboard(true, false)
}

pub(super) fn enter_attack() -> Action {
    set_blackboard(true, true)
}

pub(super) fn leave_combat() -> Action {
    set_blackboard(false, false)
}

pub(super) fn send_stun() -> Action {
    Action::Custom(Box::new(|world| {
        let mut query = world.query_filtered::<
            &mut saddle_ai_state_machine::StateMachineInstance,
            With<LabAgent>,
        >();
        for mut instance in query.iter_mut(world) {
            instance.queue_signal(SIGNAL_STUN);
        }
    }))
}

pub(super) fn rapid_guard_toggle_pattern() -> Vec<Action> {
    let mut actions = Vec::with_capacity(20);

    for step in 0..10 {
        actions.push(set_blackboard(step % 2 == 0, false));
        actions.push(Action::WaitFrames(5));
    }

    actions
}

pub(super) fn wait_for_state(name: &str, max_frames: u32) -> Action {
    let name = name.to_string();
    Action::WaitUntil {
        label: format!("wait for state '{name}'"),
        condition: Box::new(move |world| world.resource::<LabDiagnostics>().active_leaf_name == name),
        max_frames,
    }
}

pub(super) fn assert_state(expected: &str) -> Action {
    let expected = expected.to_string();
    assertions::resource_satisfies::<LabDiagnostics>(&format!("active leaf is '{expected}'"), move |d| {
        d.active_leaf_name == expected
    })
}
