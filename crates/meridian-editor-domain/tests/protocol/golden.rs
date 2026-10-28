//! Golden bytes: one encoded frame per variant, pinned. Each frame is a
//! little-endian `u32` length and then the self-describing binary form
//! (`SDAT`), so a renamed variant or field, a changed field type or a
//! changed wire layout fails here. Reordering is pinned by `variant_order`.
//!
//! When a variant is *appended*, add its row; no existing row may change.
//! (Substrate Data 0.3 writes SDAT version 2, which re-spelled every row once:
//! the byte after `SDAT` is `02` and keys, small ints and repeated strings are
//! shorter. Readers still accept version 1; a 0.2 reader cannot read these.)

use meridian_editor_domain::{RequestEnvelope, RequestId, write_message};
use substrate_data::Schema;

use crate::samples;

fn hex(message: &impl Schema) -> String {
    let mut frame = Vec::new();
    write_message(&mut frame, message).expect("a Vec accepts writes");
    frame.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn assert_golden(label: &str, golden: &[(&str, &str)], actual: &[String]) {
    assert_eq!(
        golden.len(),
        actual.len(),
        "{label}: a variant has no golden frame"
    );
    for ((name, expected), actual) in golden.iter().zip(actual) {
        assert_eq!(expected, actual, "{label}::{name} changed on the wire");
    }
}

const REQUESTS: [(&str, &str); 12] = [
    (
        "hello",
        "2a00000053444154020f050569640f72657175657374900f0509747970650f76657273696f6e050b68656c6c6f91",
    ),
    (
        "describe",
        "2400000053444154020f050569640f72657175657374910f03097479706505116465736372696265",
    ),
    (
        "list_entities",
        "2900000053444154020f050569640f72657175657374920f030974797065051b6c6973745f656e746974696573",
    ),
    (
        "get_component",
        "5b00000053444154020f050569640f72657175657374930f0709747970650d656e7469747913636f6d706f6e656e74051b6765745f636f6d706f6e656e740f050b696e6465781567656e65726174696f6e9792051572696769645f626f6479",
    ),
    (
        "set_component",
        "6200000053444154020f050569640f72657175657374940f0909747970650d656e7469747913636f6d706f6e656e740b76616c7565051b7365745f636f6d706f6e656e740f050b696e6465781567656e65726174696f6e9792051572696769645f626f6479ba",
    ),
    (
        "spawn_entity",
        "2800000053444154020f050569640f72657175657374950f0309747970650519737061776e5f656e74697479",
    ),
    (
        "despawn_entity",
        "4600000053444154020f050569640f72657175657374960f0509747970650d656e74697479051d6465737061776e5f656e746974790f050b696e6465781567656e65726174696f6e9792",
    ),
    (
        "play",
        "2000000053444154020f050569640f72657175657374970f0309747970650509706c6179",
    ),
    (
        "pause",
        "2100000053444154020f050569640f72657175657374980f030974797065050b7061757365",
    ),
    (
        "step",
        "2000000053444154020f050569640f72657175657374990f030974797065050973746570",
    ),
    (
        "subscribe",
        "3000000053444154020f050569640f726571756573749a0f0509747970650b746f706963051373756273637269626505076c6f67",
    ),
    (
        "unsubscribe",
        "3200000053444154020f050569640f726571756573749b0f0509747970650b746f7069630517756e73756273637269626505076c6f67",
    ),
];

const RESPONSES: [(&str, &str); 7] = [
    (
        "welcome",
        "1e00000053444154020f0509747970650f76657273696f6e050f77656c636f6d6591",
    ),
    (
        "described",
        "2400000053444154020f05097479706515636f6d706f6e656e747305136465736372696265640600",
    ),
    (
        "entities",
        "2c00000053444154020f05097479706511656e746974696573211001050b696e6465781567656e65726174696f6e9792",
    ),
    (
        "component",
        "1e00000053444154020f0509747970650b76616c75650513636f6d706f6e656e74ba",
    ),
    (
        "spawned",
        "3100000053444154020f0509747970650d656e74697479050f737061776e65640f050b696e6465781567656e65726174696f6e9792",
    ),
    ("done", "1200000053444154020f0309747970650509646f6e65"),
    (
        "failed",
        "2a00000053444154020f0509747970650b6572726f72050d6661696c65640f030005176e6f745f67726565746564",
    ),
];

const ERRORS: [(&str, &str); 7] = [
    (
        "version_mismatch",
        "2c00000053444154020f07097479706509686f73740d636c69656e74052176657273696f6e5f6d69736d617463689199",
    ),
    (
        "not_greeted",
        "1900000053444154020f03097479706505176e6f745f67726565746564",
    ),
    (
        "no_such_entity",
        "3800000053444154020f0509747970650d656e74697479051d6e6f5f737563685f656e746974790f050b696e6465781567656e65726174696f6e9792",
    ),
    (
        "no_such_component",
        "3500000053444154020f05097479706513636f6d706f6e656e7405236e6f5f737563685f636f6d706f6e656e74051572696769645f626f6479",
    ),
    (
        "component_missing",
        "5100000053444154020f0709747970650d656e7469747913636f6d706f6e656e740523636f6d706f6e656e745f6d697373696e670f050b696e6465781567656e65726174696f6e9792051572696769645f626f6479",
    ),
    (
        "invalid_value",
        "5e00000053444154020f05097479706517646961676e6f7374696373051b696e76616c69645f76616c75651001091173657665726974790f6d65737361676509706174680b6f6e4b6579050b6572726f7205196e6f742061206e756d626572060001",
    ),
    (
        "unsupported",
        "1900000053444154020f0309747970650517756e737570706f72746564",
    ),
];

const EVENTS: [(&str, &str); 5] = [
    (
        "entity_added",
        "3600000053444154020f0509747970650d656e746974790519656e746974795f61646465640f050b696e6465781567656e65726174696f6e9792",
    ),
    (
        "entity_removed",
        "3800000053444154020f0509747970650d656e74697479051d656e746974795f72656d6f7665640f050b696e6465781567656e65726174696f6e9792",
    ),
    (
        "component_changed",
        "5800000053444154020f0909747970650d656e7469747913636f6d706f6e656e740b76616c75650523636f6d706f6e656e745f6368616e6765640f050b696e6465781567656e65726174696f6e9792051572696769645f626f6479ba",
    ),
    (
        "log",
        "3700000053444154020f0709747970650b6c6576656c0f6d65737361676505076c6f67050f7761726e696e67051b6c6f77206f6e206d656d6f7279",
    ),
    (
        "play_state_changed",
        "2e00000053444154020f0509747970650b73746174650525706c61795f73746174655f6368616e676564050d706175736564",
    ),
];

const HOST_MESSAGES: [(&str, &str); 2] = [
    (
        "response",
        "2300000053444154020f07097479706505696411726573706f6e736522930f03000509646f6e65",
    ),
    (
        "event",
        "4000000053444154020f0509747970650b6576656e74210f05000d656e746974790519656e746974795f61646465640f050b696e6465781567656e65726174696f6e9792",
    ),
];

const TOPICS: [(&str, &str); 4] = [
    ("entities", "0f00000053444154020511656e746974696573"),
    ("components", "1100000053444154020515636f6d706f6e656e7473"),
    ("log", "0a000000534441540205076c6f67"),
    ("play_state", "1100000053444154020515706c61795f7374617465"),
];

const LEVELS: [(&str, &str); 4] = [
    ("debug", "0c0000005344415402050b6465627567"),
    ("info", "0b00000053444154020509696e666f"),
    ("warning", "0e0000005344415402050f7761726e696e67"),
    ("error", "0c0000005344415402050b6572726f72"),
];

const STATES: [(&str, &str); 2] = [
    ("playing", "0e0000005344415402050f706c6179696e67"),
    ("paused", "0d0000005344415402050d706175736564"),
];

#[test]
fn every_request_has_its_golden_frame() {
    let actual: Vec<String> = samples::requests()
        .into_iter()
        .enumerate()
        .map(|(i, request)| {
            let id = RequestId::new(u64::try_from(i).expect("a small index"));
            hex(&RequestEnvelope { id, request })
        })
        .collect();
    assert_golden("Request", &REQUESTS, &actual);
}

#[test]
fn every_response_has_its_golden_frame() {
    let actual: Vec<String> = samples::responses().iter().map(hex).collect();
    assert_golden("Response", &RESPONSES, &actual);
}

#[test]
fn every_error_has_its_golden_frame() {
    let actual: Vec<String> = samples::errors().iter().map(hex).collect();
    assert_golden("RequestError", &ERRORS, &actual);
}

#[test]
fn every_event_has_its_golden_frame() {
    let actual: Vec<String> = samples::events().iter().map(hex).collect();
    assert_golden("Event", &EVENTS, &actual);
}

#[test]
fn every_host_message_has_its_golden_frame() {
    let actual: Vec<String> = samples::host_messages().iter().map(hex).collect();
    assert_golden("HostMessage", &HOST_MESSAGES, &actual);
}

#[test]
fn topics_levels_and_states_have_golden_frames() {
    let topics: Vec<String> = samples::TOPICS.iter().map(hex).collect();
    assert_golden("EventTopic", &TOPICS, &topics);
    let levels: Vec<String> = samples::LEVELS.iter().map(hex).collect();
    assert_golden("LogLevel", &LEVELS, &levels);
    let states: Vec<String> = samples::STATES.iter().map(hex).collect();
    assert_golden("PlayState", &STATES, &states);
}
