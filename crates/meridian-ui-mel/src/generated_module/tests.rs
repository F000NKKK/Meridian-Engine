//! Tests for [`GeneratedModule::render`].

use std::path::Path;

use super::*;

fn render(source: &str) -> String {
    let document = MelDocument::parse(Path::new("/scenes/demo.meui"), source).unwrap();
    GeneratedModule::render(&document).code
}

#[test]
fn names_class_and_functions_are_generated() {
    let code = render(r#"<Scene x:Class="crate::Demo"><Entity x:Name="left_arm" /></Scene>"#);
    assert!(
        code.contains(r#"include_str!("/scenes/demo.meui")"#),
        "{code}"
    );
    assert!(
        code.contains("    // @mel /scenes/demo.meui:1:30\n    LeftArm,"),
        "{code}"
    );
    assert!(
        code.contains("pub const ALL: [Self; 1] = [Self::LeftArm];"),
        "{code}"
    );
    assert!(code.contains(r#"Self::LeftArm => "left_arm","#), "{code}");
    assert!(code.contains("pub type Class = crate::Demo;"), "{code}");
    assert!(!code.contains("ViewModel"), "{code}");
    assert!(
        code.contains("::meridian_sdk::dsl::build_scene(SOURCE, registry)"),
        "{code}"
    );
}

#[test]
fn bindings_become_ui_glue() {
    let code = render(
        r#"<StackPanel x:ViewModel="crate::HudVm">
            <TextBlock x:Name="title" text="{x:Bind player?.name, Fallback='-'}" font_size="{x:Bind size, Mode=OneTime}" />
            <TextBlock x:Name="query_box" text="{x:Bind search.text, Mode=TwoWay}" />
            <Border x:Name="mute" width="{x:Bind width(), Mode=TwoWay, BindBack=set_width}" />
        </StackPanel>"#,
    );
    let ui = "::meridian_sdk::ui_core";
    assert!(
        code.contains(&format!("pub title: {ui}::NodeId,")),
        "{code}"
    );
    assert!(
        code.contains(&format!("pub query_box: {ui}::NodeId,")),
        "{code}"
    );
    assert!(
        code.contains("pub fn apply_one_time_bindings(vm: &ViewModel, tree: &mut"),
        "{code}"
    );
    assert!(
        code.contains(&format!(
            "tree.set(names.title, {ui}::properties::text_block::FONT_SIZE, ::core::convert::Into::into(::core::clone::Clone::clone(&(vm.size))))?;"
        )),
        "{code}"
    );
    assert!(
        code.contains(
            "match (|| Some(::core::clone::Clone::clone(&(vm.player.as_ref()?.name))))() {"
        ),
        "{code}"
    );
    assert!(code.contains("None => tree.set(names.title,"), "{code}");
    assert!(
        code.contains("pub fn write_back_bindings(vm: &mut ViewModel"),
        "{code}"
    );
    assert!(
        code.contains("vm.search.text = ::core::convert::Into::into(value);"),
        "{code}"
    );
    assert!(
        code.contains("vm.set_width(::core::convert::Into::into(value));"),
        "{code}"
    );
    assert!(
        code.contains(&format!(
            "change.node == names.mute && change.property == {ui}::properties::element::WIDTH.meta().id"
        )),
        "{code}"
    );
    assert!(code.contains("// @mel /scenes/demo.meui:2:"), "{code}");
    assert_eq!(
        code.matches('{').count(),
        code.matches('}').count(),
        "{code}"
    );
}

#[test]
fn ui_documents_build_their_tree() {
    let code = render(
        r#"<StackPanel width="200">
            <TextBlock x:Name="title" text="Hi" Grid.Row="1" />
            <Border x:Name="frame"><TextBlock text="@@x" /></Border>
        </StackPanel>"#,
    );
    let ui = "::meridian_sdk::ui_core";
    assert!(
        code.contains(&format!(
            "pub fn build_ui(tree: &mut {ui}::UiTree) -> Result<({ui}::NodeId, Names), {ui}::UiError> {{"
        )),
        "{code}"
    );
    let expected = [
        format!("    let n0 = tree.create({ui}::ElementKind::StackPanel);"),
        format!("    tree.set_literal(n0, {ui}::properties::element::WIDTH.meta().id, \"200\")?;"),
        format!("    let n1 = tree.create({ui}::ElementKind::TextBlock);"),
        format!("    tree.set_literal(n1, {ui}::properties::grid::ROW.meta().id, \"1\")?;"),
        "    tree.append_child(n0, n1)?;".to_string(),
        "    let n2 = tree.create(::meridian_sdk::ui_core::ElementKind::Border);".to_string(),
        "    tree.append_child(n2, n3)?;".to_string(),
        "        title: n1,".to_string(),
        "        frame: n2,".to_string(),
    ];
    for line in expected {
        assert!(code.contains(&line), "missing {line}\n{code}");
    }
    assert!(code.contains(r#""@@x")?;"#), "{code}");
    assert!(code.contains("// @mel /scenes/demo.meui:2:"), "{code}");
    assert!(!code.contains("pub fn build("), "{code}");
    assert_eq!(
        code.matches('{').count(),
        code.matches('}').count(),
        "{code}"
    );
}

#[test]
fn one_way_only_documents_have_no_write_back() {
    let code = render(
        r#"<StackPanel x:ViewModel="V"><TextBlock x:Name="t" text="{x:Bind s}" /></StackPanel>"#,
    );
    assert!(code.contains("pub fn apply_bindings("), "{code}");
    assert!(!code.contains("apply_one_time_bindings"), "{code}");
    assert!(!code.contains("write_back_bindings"), "{code}");
}

#[test]
fn document_without_names_has_an_empty_enum() {
    let code = render("<Scene />");
    assert!(code.contains("pub enum Name {\n}\n\n"), "{code}");
    assert!(
        code.contains("        match self {\n        }\n    }\n}\n"),
        "{code}"
    );
    assert_eq!(
        code.matches('{').count(),
        code.matches('}').count(),
        "{code}"
    );
    assert_eq!(code.matches("#[allow(dead_code").count(), 5, "{code}");
    assert!(!code.contains("Names"), "{code}");
    assert!(code.contains("pub const ALL: [Self; 0] = [];"), "{code}");
}

#[test]
fn event_handlers_become_dispatch() {
    let code = render(
        r#"<StackPanel x:ViewModel="crate::HudVm">
            <TextBlock x:Name="title" click="{x:Bind save}" key_pressed="{x:Bind on_key(e)}" />
        </StackPanel>"#,
    );
    assert!(code.contains("pub fn dispatch(vm: &mut ViewModel, tree: &::meridian_sdk::ui_core::UiTree, names: &Names, events: &[::meridian_sdk::ui_core::UiEvent]) {"), "{code}");
    assert!(
        code.contains("route.extend(tree.ancestors(event.target));"),
        "{code}"
    );
    assert!(
        code.contains(
            "if node == names.title && tag == ::meridian_sdk::ui_core::UiEventTag::Click {"
        ),
        "{code}"
    );
    assert!(
        code.contains(
            "IntoHandled::into_handled(vm.save()) == ::meridian_sdk::ui_core::Handled::Yes"
        ),
        "{code}"
    );
    assert!(code.contains("vm.on_key(event)"), "{code}");
}
