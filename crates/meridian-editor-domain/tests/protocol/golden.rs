//! Golden bytes: one encoded frame per variant, pinned. Each frame is a
//! little-endian `u32` length and then the self-describing binary form
//! (`SDAT`), so a renamed variant or field, a changed field type or a
//! changed wire layout fails here. Reordering is pinned by `variant_order`.
//!
//! When a variant is *appended*, add its row; no existing row may change.

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
        "2c0000005344415401070205696403000f7265717565737407020974797065050b68656c6c6f0f76657273696f6e0302",
    ),
    (
        "describe",
        "250000005344415401070205696403020f726571756573740701097479706505116465736372696265",
    ),
    (
        "list_entities",
        "2a0000005344415401070205696403040f7265717565737407010974797065051b6c6973745f656e746974696573",
    ),
    (
        "get_component",
        "5e0000005344415401070205696403060f7265717565737407030974797065051b6765745f636f6d706f6e656e740d656e7469747907020b696e646578030e1567656e65726174696f6e030413636f6d706f6e656e74051572696769645f626f6479",
    ),
    (
        "set_component",
        "660000005344415401070205696403080f7265717565737407040974797065051b7365745f636f6d706f6e656e740d656e7469747907020b696e646578030e1567656e65726174696f6e030413636f6d706f6e656e74051572696769645f626f64790b76616c75650354",
    ),
    (
        "spawn_entity",
        "2900000053444154010702056964030a0f72657175657374070109747970650519737061776e5f656e74697479",
    ),
    (
        "despawn_entity",
        "4900000053444154010702056964030c0f7265717565737407020974797065051d6465737061776e5f656e746974790d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
    (
        "play",
        "2100000053444154010702056964030e0f72657175657374070109747970650509706c6179",
    ),
    (
        "pause",
        "220000005344415401070205696403100f7265717565737407010974797065050b7061757365",
    ),
    (
        "step",
        "210000005344415401070205696403120f7265717565737407010974797065050973746570",
    ),
    (
        "subscribe",
        "310000005344415401070205696403140f726571756573740702097479706505137375627363726962650b746f70696305076c6f67",
    ),
    (
        "unsubscribe",
        "330000005344415401070205696403160f72657175657374070209747970650517756e7375627363726962650b746f70696305076c6f67",
    ),
];

const RESPONSES: [(&str, &str); 7] = [
    (
        "welcome",
        "1f000000534441540107020974797065050f77656c636f6d650f76657273696f6e0302",
    ),
    (
        "described",
        "24000000534441540107020974797065051364657363726962656415636f6d706f6e656e74730600",
    ),
    (
        "entities",
        "300000005344415401070209747970650511656e74697469657302060107020b696e646578030e1567656e65726174696f6e0304",
    ),
    (
        "component",
        "1f0000005344415401070209747970650513636f6d706f6e656e740b76616c75650354",
    ),
    (
        "spawned",
        "33000000534441540107020974797065050f737061776e65640d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
    ("done", "120000005344415401070109747970650509646f6e65"),
    (
        "failed",
        "2a000000534441540107020974797065050d6661696c65640b6572726f7207010005176e6f745f67726565746564",
    ),
];

const ERRORS: [(&str, &str); 7] = [
    (
        "version_mismatch",
        "2e000000534441540107030974797065052176657273696f6e5f6d69736d6174636809686f737403020d636c69656e740312",
    ),
    (
        "not_greeted",
        "1900000053444154010701097479706505176e6f745f67726565746564",
    ),
    (
        "no_such_entity",
        "3a000000534441540107020974797065051d6e6f5f737563685f656e746974790d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
    (
        "no_such_component",
        "3500000053444154010702097479706505236e6f5f737563685f636f6d706f6e656e7413636f6d706f6e656e74051572696769645f626f6479",
    ),
    (
        "component_missing",
        "530000005344415401070309747970650523636f6d706f6e656e745f6d697373696e670d656e7469747907020b696e646578030e1567656e65726174696f6e030413636f6d706f6e656e74051572696769645f626f6479",
    ),
    (
        "invalid_value",
        "5f000000534441540107020974797065051b696e76616c69645f76616c756517646961676e6f737469637306010704117365766572697479050b6572726f720f6d65737361676505196e6f742061206e756d626572097061746806000b6f6e4b657901",
    ),
    (
        "unsupported",
        "190000005344415401070109747970650517756e737570706f72746564",
    ),
];

const EVENTS: [(&str, &str); 5] = [
    (
        "entity_added",
        "380000005344415401070209747970650519656e746974795f61646465640d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
    (
        "entity_removed",
        "3a000000534441540107020974797065051d656e746974795f72656d6f7665640d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
    (
        "component_changed",
        "5b0000005344415401070409747970650523636f6d706f6e656e745f6368616e6765640d656e7469747907020b696e646578030e1567656e65726174696f6e030413636f6d706f6e656e74051572696769645f626f64790b76616c75650354",
    ),
    (
        "log",
        "3700000053444154010703097479706505076c6f670b6c6576656c050f7761726e696e670f6d657373616765051b6c6f77206f6e206d656d6f7279",
    ),
    (
        "play_state_changed",
        "2e0000005344415401070209747970650525706c61795f73746174655f6368616e6765640b7374617465050d706175736564",
    ),
];

const HOST_MESSAGES: [(&str, &str); 2] = [
    (
        "response",
        "250000005344415401070309747970650511726573706f6e73650569640306020701000509646f6e65",
    ),
    (
        "event",
        "43000000534441540107020974797065050b6576656e74020702000519656e746974795f61646465640d656e7469747907020b696e646578030e1567656e65726174696f6e0304",
    ),
];

const TOPICS: [(&str, &str); 4] = [
    ("entities", "0f00000053444154010511656e746974696573"),
    ("components", "1100000053444154010515636f6d706f6e656e7473"),
    ("log", "0a000000534441540105076c6f67"),
    ("play_state", "1100000053444154010515706c61795f7374617465"),
];

const LEVELS: [(&str, &str); 4] = [
    ("debug", "0c0000005344415401050b6465627567"),
    ("info", "0b00000053444154010509696e666f"),
    ("warning", "0e0000005344415401050f7761726e696e67"),
    ("error", "0c0000005344415401050b6572726f72"),
];

const STATES: [(&str, &str); 2] = [
    ("playing", "0e0000005344415401050f706c6179696e67"),
    ("paused", "0d0000005344415401050d706175736564"),
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
