//! `Play`, `Pause`, `Step` against the simulation port.

use meridian_editor_host::SimulationControl;
use meridian_editor_protocol::{Event, EventTopic, HostMessage, PlayState, Request, Response};

use crate::rig::Rig;

fn state_events(sent: &[HostMessage]) -> Vec<PlayState> {
    sent.iter()
        .filter_map(|m| match m {
            HostMessage::Event {
                event: Event::PlayStateChanged { state },
            } => Some(*state),
            _ => None,
        })
        .collect()
}

#[test]
fn play_then_pause_toggle_the_simulation_and_announce_each_change() {
    let mut rig = Rig::greeted(&[EventTopic::PlayState]);
    let played = rig.send(Request::Play);
    assert_eq!(state_events(&played), vec![PlayState::Playing]);
    assert_eq!(rig.simulation.state(), PlayState::Playing);
    let paused = rig.send(Request::Pause);
    assert_eq!(state_events(&paused), vec![PlayState::Paused]);
    assert_eq!(rig.simulation.state(), PlayState::Paused);
}

#[test]
fn playing_twice_announces_once() {
    let mut rig = Rig::greeted(&[EventTopic::PlayState]);
    rig.send(Request::Play);
    let again = rig.send(Request::Play);
    assert_eq!(again.len(), 1);
    let paused = rig.send(Request::Pause);
    assert_eq!(state_events(&paused).len(), 1);
    assert_eq!(rig.send(Request::Pause).len(), 1);
}

#[test]
fn step_runs_one_step_and_stays_paused_without_an_event() {
    let mut rig = Rig::greeted(&[EventTopic::PlayState]);
    let sent = rig.send(Request::Step);
    assert_eq!(sent.len(), 1);
    assert_eq!(rig.simulation.steps(), 1);
    assert_eq!(rig.simulation.state(), PlayState::Paused);
}

#[test]
fn step_while_playing_pauses_afterwards() {
    let mut rig = Rig::greeted(&[EventTopic::PlayState]);
    rig.respond(Request::Play);
    let sent = rig.send(Request::Step);
    assert_eq!(state_events(&sent), vec![PlayState::Paused]);
    assert_eq!(rig.simulation.steps(), 1);
    assert_eq!(rig.simulation.state(), PlayState::Paused);
}

#[test]
fn an_unsubscribed_editor_still_gets_done_but_no_event() {
    let mut rig = Rig::greeted(&[]);
    let sent = rig.send(Request::Play);
    assert_eq!(sent.len(), 1);
    assert!(matches!(
        sent[0],
        HostMessage::Response {
            response: Response::Done,
            ..
        }
    ));
    assert_eq!(rig.simulation.state(), PlayState::Playing);
}
