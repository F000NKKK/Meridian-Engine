#!/usr/bin/env python3
"""Architectural test: enforce docs/dependency-rules.md and ADR 019.

Two checks, both exact:

1. Every internal (meridian-*) edge in the workspace appears in ALLOWED or
   in PENDING below, and every edge listed there exists. Adding an edge, even
   a legitimate one, updates this file and docs/dependency-rules.md in the
   same change.
2. Dependencies point inwards (ADR 019, Craftbook ch. 7 §7.3). Each crate's
   layer comes from craftbook.toml, the one source of layers. An edge that
   points outwards (domain -> infra, app -> infra, ...) is allowed only while
   it is listed in PENDING with the wave that removes it; an edge in ALLOWED
   must point inwards. PENDING may only shrink.

Run: ./build.sh check-deps   (or: python3 scripts/check_dependency_rules.py)
"""
import glob
import os
import re
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Which layers a crate of each layer may depend on (Craftbook ch. 7 §7.3,
# the same table craftcheck deps uses).
INWARD = {
    "shared-kernel": {"shared-kernel"},
    "domain": {"shared-kernel", "domain"},
    "app": {"shared-kernel", "domain", "app"},
    "infra": {"shared-kernel", "domain", "app", "infra"},
    "interface": {"shared-kernel", "domain", "app", "infra", "interface"},
}

# crate -> direct meridian-* dependencies that follow the target direction.
ALLOWED = {
    "meridian-diagnostics": set(),
    "meridian-diagnostics-fs": {"meridian-diagnostics"},
    "meridian-numeric": {"meridian-diagnostics"},
    "meridian-gac": {"meridian-numeric"},
    "meridian-memory": set(),
    "meridian-resource": {"meridian-memory"},
    "meridian-runtime-threads": {"meridian-asset-app", "meridian-runtime-app"},
    "meridian-platform-winit": {"meridian-diagnostics"},
    "meridian-gpu-wgpu": {"meridian-diagnostics", "meridian-platform-winit"},
    "meridian-compute-wgpu": {
        "meridian-platform-winit",
        "meridian-gpu-wgpu",
        "meridian-compute-app",
    },
    "meridian-compute-app": set(),
    "meridian-gac-wgpu": {
        "meridian-gac",
        "meridian-numeric",
        "meridian-compute-app",
        "meridian-compute-wgpu",
        "meridian-gpu-wgpu",
    },
    "meridian-ecs": {"meridian-memory", "meridian-gac"},
    "meridian-asset-domain": {"meridian-diagnostics"},
    "meridian-asset-app": {"meridian-asset-domain"},
    "meridian-asset-fs": {"meridian-asset-app", "meridian-asset-domain"},
    "meridian-asset-http": {"meridian-asset-app"},
    "meridian-asset-gltf": {"meridian-asset-domain", "meridian-gac"},
    "meridian-asset-jpeg": {"meridian-asset-domain"},
    "meridian-asset-png": {"meridian-asset-domain"},
    "meridian-asset-symphonia": {"meridian-asset-domain"},
    "meridian-rendering-wgpu": {
        "meridian-diagnostics",
        "meridian-platform-winit",
        "meridian-gpu-wgpu",
        "meridian-gac",
        "meridian-rendering-domain",
        "meridian-rendering-app",
        "meridian-memory",
        "meridian-asset-domain",
    },
    "meridian-audio-cpal": {
        "meridian-diagnostics",
        "meridian-platform-winit",
        "meridian-audio-domain",
        "meridian-audio-app",
    },
    "meridian-audio-app": {"meridian-audio-domain"},
    "meridian-rendering-app": {
        "meridian-rendering-domain",
        "meridian-asset-domain",
        "meridian-memory",
    },
    "meridian-rendering-domain": {
        "meridian-diagnostics",
        "meridian-gac",
        "meridian-memory",
        "meridian-ecs",
        "meridian-resource",
    },
    "meridian-physics-wgpu": {
        "meridian-diagnostics",
        "meridian-physics-domain",
        "meridian-physics-app",
        "meridian-gac",
        "meridian-gac-wgpu",
        "meridian-compute-app",
        "meridian-compute-wgpu",
        "meridian-numeric",
        "meridian-gpu-wgpu",
    },
    "meridian-physics-domain": {
        "meridian-gac",
        "meridian-numeric",
        "meridian-resource",
    },
    "meridian-audio-domain": {
        "meridian-gac",
    },
    "meridian-runtime-app": {
        "meridian-audio-app",
        "meridian-audio-domain",
        "meridian-diagnostics",
        "meridian-ecs",
        "meridian-physics-app",
        "meridian-compute-app",
    },
    "meridian-physics-app": {"meridian-physics-domain", "meridian-gac"},
    "meridian-data-substrate": {"meridian-ecs", "meridian-gac"},
    "meridian-editor-domain": {"meridian-ecs", "meridian-data-substrate"},
    "meridian-editor-app": {
        "meridian-ecs",
        "meridian-editor-domain",
        "meridian-data-substrate",
    },
    "meridian-editor-tcp": {"meridian-editor-app", "meridian-editor-domain"},
    "meridian-markup": {"meridian-diagnostics"},
    "meridian-markup-macros": set(),
    "meridian-ui-mel": {"meridian-markup", "meridian-ui-domain"},
    "meridian-ui-domain": {"meridian-diagnostics", "meridian-memory"},
    "meridian-text-domain": set(),
    "meridian-ui-scene2d": {
        "meridian-gac",
        "meridian-rendering-domain",
        "meridian-text-domain",
        "meridian-ui-domain",
    },
    "meridian-sdk": {
        "meridian-memory",
        "meridian-resource",
        "meridian-data-substrate",
        "meridian-ecs",
        "meridian-asset-app",
        "meridian-asset-fs",
        "meridian-asset-http",
        "meridian-diagnostics",
        "meridian-gac",
        "meridian-platform-winit",
        "meridian-asset-domain",
        "meridian-asset-gltf",
        "meridian-asset-jpeg",
        "meridian-asset-png",
        "meridian-asset-symphonia",
        "meridian-rendering-wgpu",
        "meridian-rendering-domain",
        "meridian-physics-domain",
        "meridian-audio-domain",
        "meridian-audio-app",
        "meridian-audio-cpal",
        "meridian-rendering-app",
        "meridian-diagnostics-fs",
        "meridian-runtime-app",
        "meridian-physics-app",
        "meridian-physics-wgpu",
        "meridian-runtime-threads",
        "meridian-compute-wgpu",
        "meridian-compute-app",
        "meridian-markup",
        "meridian-markup-macros",
        "meridian-ui-domain",
        "meridian-ui-scene2d",
    },
}

# (crate, dependency) -> the migration wave that removes the edge.
# These edges point outwards and contradict ADR 019; they stay until that
# wave inverts them. Never add to this table.
PENDING = {
}

NAME_RE = re.compile(r'^name *= *"([^"]+)"', re.M)
DEP_RE = re.compile(r'^(meridian-[a-z0-9_-]+) *=', re.M)


def actual_deps():
    found = {}
    for toml in glob.glob(os.path.join(ROOT, "crates", "*", "Cargo.toml")):
        text = open(toml).read()
        m = NAME_RE.search(text)
        if not m:
            continue
        name = m.group(1)
        # Only the [dependencies] table, not [package]/[dev-dependencies].
        section = text.split("[dependencies]", 1)
        deps_text = section[1] if len(section) > 1 else ""
        deps_text = deps_text.split("\n[", 1)[0]
        deps = set(DEP_RE.findall(deps_text)) - {name}
        found[name] = deps
    return found


def layers():
    with open(os.path.join(ROOT, "craftbook.toml"), "rb") as fh:
        config = tomllib.load(fh)
    return {crate["name"]: crate["layer"] for crate in config.get("crate", [])}


def edge_errors(actual):
    errors = []
    for crate, deps in sorted(actual.items()):
        allowed = ALLOWED.get(crate)
        if allowed is None:
            errors.append(
                f"{crate}: no entry in ALLOWED (scripts/check_dependency_rules.py) "
                f"— add one and document it in docs/dependency-rules.md"
            )
            continue
        pending = {dep for (src, dep) in PENDING if src == crate}
        for dep in sorted(deps - allowed - pending):
            errors.append(f"{crate} -> {dep}: undocumented edge (not in ALLOWED)")
        for dep in sorted((allowed | pending) - deps):
            errors.append(
                f"{crate} -> {dep}: documented in ALLOWED/PENDING but missing from Cargo.toml"
            )
    for crate in sorted(set(ALLOWED) - set(actual)):
        errors.append(f"{crate}: listed in ALLOWED but crate no longer exists")
    return errors


def direction_errors(layer_of):
    errors = []
    edges = [(src, dep) for src, deps in ALLOWED.items() for dep in deps]
    for src, dep in sorted(edges + list(PENDING)):
        src_layer, dep_layer = layer_of.get(src), layer_of.get(dep)
        if src_layer is None or dep_layer is None:
            errors.append(f"{src} -> {dep}: crate missing from craftbook.toml's layer map")
            continue
        inward = dep_layer in INWARD[src_layer]
        if (src, dep) in PENDING and inward:
            errors.append(f"{src} -> {dep}: points inwards now; remove it from PENDING")
        elif (src, dep) not in PENDING and not inward:
            errors.append(
                f"{src} -> {dep}: {src_layer} crate depends on {dep_layer} crate; "
                f"dependencies point inwards (ADR 019)"
            )
    return errors


def main():
    errors = edge_errors(actual_deps()) + direction_errors(layers())
    if errors:
        print("dependency-rules violations:\n")
        for e in errors:
            print(f"  - {e}")
        print(f"\n{len(errors)} violation(s). See docs/dependency-rules.md.")
        return 1

    print(
        f"OK: {len(ALLOWED)} crates match docs/dependency-rules.md; "
        f"{len(PENDING)} outward edge(s) pending migration (ADR 019)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
