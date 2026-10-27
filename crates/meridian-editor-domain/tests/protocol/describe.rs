//! `Describe` carries the registry's description as it is.

use meridian_data_substrate::{ComponentName, ComponentRegistry, TransformCodec};
use meridian_ecs::Transform;
use meridian_editor_domain::{HostMessage, RequestId, Response, read_message, write_message};

fn registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    let name = ComponentName::new("transform").expect("a valid name");
    registry
        .register_with::<Transform, TransformCodec>(name)
        .expect("registers once");
    registry
}

#[test]
fn the_describe_response_equals_the_registry_description() {
    let registry = registry();
    let sent = HostMessage::Response {
        id: RequestId::new(1),
        response: Response::Described {
            components: registry.describe(),
        },
    };
    let mut stream = Vec::new();
    write_message(&mut stream, &sent).expect("a Vec accepts writes");
    let received = read_message::<HostMessage>(&mut stream.as_slice()).expect("reads back");
    let Some(HostMessage::Response {
        response: Response::Described { components },
        ..
    }) = received
    else {
        panic!("a Described response was sent");
    };
    assert_eq!(components, registry.describe());
    assert_eq!(components.len(), 1);
}
