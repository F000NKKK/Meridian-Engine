#!/usr/bin/env python3
"""Architectural test: enforce docs/dependency-rules.md by whitelist.

Every internal (meridian-*) edge in the workspace must appear in ALLOWED
below. This is deliberately an *exact* match, not a subset check: adding a
new edge — even a legitimate one — must update this file and
docs/dependency-rules.md in the same change, so the graph documented there
never drifts from the graph that actually compiles.

Run: ./build.sh check-deps   (or: python3 scripts/check_dependency_rules.py)
"""
import re
import sys
import glob
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# crate -> set of allowed direct meridian-* dependencies.
# Keep in sync with the diagram + rules in docs/dependency-rules.md.
ALLOWED = {
    "meridian-foundation": set(),
    "meridian-numeric-core": {"meridian-foundation"},
    "meridian-gac-core": {"meridian-numeric-core"},
    "meridian-memory-core": set(),
    "meridian-resource-core": {"meridian-memory-core"},
    "meridian-task-core": set(),
    "meridian-platform-core": set(),
    "meridian-compute-driver": {"meridian-platform-core"},
    "meridian-compute-runtime": {
        "meridian-compute-driver",
        "meridian-task-core",
        "meridian-memory-core",
    },
    "meridian-gac-compute": {
        "meridian-gac-core",
        "meridian-compute-runtime",
    },
    "meridian-ecs-core": {"meridian-memory-core", "meridian-gac-core"},
    "meridian-asset-core": {"meridian-platform-core"},
    "meridian-graphics-driver": {"meridian-platform-core"},
    "meridian-audio-driver": {"meridian-platform-core"},
    "meridian-physics-driver": {"meridian-platform-core"},
    "meridian-graphics-core": {
        "meridian-gac-core",
        "meridian-gac-compute",
        "meridian-memory-core",
        "meridian-ecs-core",
        "meridian-asset-core",
        "meridian-graphics-driver",
        "meridian-compute-runtime",
        "meridian-resource-core",
    },
    "meridian-physics-core": {
        "meridian-gac-core",
        "meridian-gac-compute",
        "meridian-ecs-core",
        "meridian-compute-runtime",
        "meridian-physics-driver",
        "meridian-resource-core",
    },
    "meridian-audio-core": {
        "meridian-gac-core",
        "meridian-audio-driver",
        "meridian-resource-core",
    },
    "meridian-engine-core": {
        "meridian-gac-core",
        "meridian-memory-core",
        "meridian-task-core",
        "meridian-platform-core",
        "meridian-ecs-core",
        "meridian-graphics-core",
        "meridian-physics-core",
        "meridian-audio-core",
        "meridian-asset-core",
        "meridian-compute-runtime",
    },
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


def main():
    actual = actual_deps()
    errors = []

    for crate, deps in sorted(actual.items()):
        allowed = ALLOWED.get(crate)
        if allowed is None:
            errors.append(
                f"{crate}: no entry in ALLOWED (scripts/check_dependency_rules.py) "
                f"— add one and document it in docs/dependency-rules.md"
            )
            continue
        extra = deps - allowed
        missing = allowed - deps
        for dep in sorted(extra):
            errors.append(f"{crate} -> {dep}: undocumented edge (not in ALLOWED)")
        for dep in sorted(missing):
            errors.append(
                f"{crate} -> {dep}: documented in ALLOWED but missing from Cargo.toml"
            )

    known = set(ALLOWED.keys())
    stale = known - set(actual.keys())
    for crate in sorted(stale):
        errors.append(f"{crate}: listed in ALLOWED but crate no longer exists")

    if errors:
        print("dependency-rules violations:\n")
        for e in errors:
            print(f"  - {e}")
        print(f"\n{len(errors)} violation(s). See docs/dependency-rules.md.")
        return 1

    print(f"OK: {len(actual)} crates match docs/dependency-rules.md.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
