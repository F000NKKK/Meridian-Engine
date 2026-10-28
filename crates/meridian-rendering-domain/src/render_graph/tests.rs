//! Lifetimes and slot sharing of transient render-graph resources.

use std::collections::HashMap;

use crate::{GraphResourceId, RenderGraph, RenderPass, ResourceLifetime};

const SHADOW: GraphResourceId = GraphResourceId(0);
const HDR: GraphResourceId = GraphResourceId(1);
const LDR: GraphResourceId = GraphResourceId(2);
const FINAL: GraphResourceId = GraphResourceId(3);
const SWAPCHAIN: GraphResourceId = GraphResourceId(9);

/// shadow -> lighting -> tonemap -> present, each reading the previous output.
fn chain() -> RenderGraph {
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("shadow").writing(SHADOW));
    graph.add_pass(RenderPass::new("lighting").reading(SHADOW).writing(HDR));
    graph.add_pass(RenderPass::new("tonemap").reading(HDR).writing(LDR));
    graph.add_pass(
        RenderPass::new("present")
            .reading(LDR)
            .reading(SWAPCHAIN)
            .writing(FINAL),
    );
    graph
}

fn keyed(key: u32) -> HashMap<GraphResourceId, u32> {
    [SHADOW, HDR, LDR, FINAL]
        .into_iter()
        .map(|resource| (resource, key))
        .collect()
}

#[test]
fn a_resource_lives_from_its_writer_to_its_last_reader() {
    let lifetimes = chain().resource_lifetimes().unwrap();
    assert_eq!(lifetimes[&SHADOW], ResourceLifetime { first: 0, last: 1 });
    assert_eq!(lifetimes[&HDR], ResourceLifetime { first: 1, last: 2 });
    assert_eq!(lifetimes[&FINAL], ResourceLifetime { first: 3, last: 3 });
}

#[test]
fn an_externally_supplied_resource_has_no_lifetime() {
    assert!(
        !chain()
            .resource_lifetimes()
            .unwrap()
            .contains_key(&SWAPCHAIN)
    );
}

#[test]
fn resources_touching_at_one_pass_overlap() {
    let a = ResourceLifetime { first: 0, last: 1 };
    let b = ResourceLifetime { first: 1, last: 2 };
    let c = ResourceLifetime { first: 2, last: 3 };
    assert!(a.overlaps(b));
    assert!(!a.overlaps(c));
}

#[test]
fn compatible_resources_with_disjoint_lifetimes_share_a_slot() {
    let plan = chain().plan_allocations(&keyed(0)).unwrap();
    assert_eq!(plan.slot_count(), 2);
    assert_eq!(plan.slot(SHADOW), plan.slot(LDR));
    assert_ne!(plan.slot(HDR), plan.slot(SHADOW));
    assert_eq!(plan.slot(FINAL), plan.slot(HDR));
}

#[test]
fn resources_with_different_keys_never_share() {
    let mut keys = keyed(0);
    keys.insert(LDR, 1);
    keys.insert(FINAL, 2);
    let plan = chain().plan_allocations(&keys).unwrap();
    assert_eq!(plan.slot_count(), 4);
    assert_ne!(plan.slot(LDR), plan.slot(SHADOW));
}

#[test]
fn resources_missing_from_the_keys_are_not_planned() {
    let mut keys = keyed(0);
    keys.remove(&HDR);
    let plan = chain().plan_allocations(&keys).unwrap();
    assert_eq!(plan.slot(HDR), None);
    assert_eq!(plan.slot(SWAPCHAIN), None);
}

#[test]
fn an_invalid_graph_plans_nothing() {
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("a").reading(HDR).writing(LDR));
    graph.add_pass(RenderPass::new("b").reading(LDR).writing(HDR));
    assert!(graph.plan_allocations(&keyed(0)).is_err());
}

#[test]
fn an_empty_graph_needs_no_slots() {
    let plan = RenderGraph::new().plan_allocations(&keyed(0)).unwrap();
    assert_eq!(plan.slot_count(), 0);
}
