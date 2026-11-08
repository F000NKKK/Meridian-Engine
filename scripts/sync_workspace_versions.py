#!/usr/bin/env python3
"""Synchronize path dependencies in [workspace.dependencies] with member versions.

The release script invokes --write after bumping member crates. CI invokes --check.
Cargo version requirements in this workspace are simple numeric 0.x or 1.x
requirements; for example 0.13 and 0.13.0 mean the same minimum version.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
import tomllib


NUMERIC_VERSION = re.compile(r"^(\d+)\.(\d+)(?:\.(\d+))?$")


def version_tuple(value: str) -> tuple[int, int, int]:
    match = NUMERIC_VERSION.fullmatch(value)
    if match is None:
        raise ValueError(f"Unsupported workspace version requirement: {value!r}")
    return int(match[1]), int(match[2]), int(match[3] or 0)


def synchronize(root: Path, write: bool) -> int:
    manifest_path = root / "Cargo.toml"
    manifest = manifest_path.read_text(encoding="utf-8")
    data = tomllib.loads(manifest)
    deps = data["workspace"]["dependencies"]
    updates: dict[str, tuple[str, str]] = {}

    for name, dependency in deps.items():
        if not isinstance(dependency, dict) or "path" not in dependency:
            continue
        if "version" not in dependency:
            raise ValueError(f"{name}: local workspace dependency must have a version")
        member_path = (root / dependency["path"] / "Cargo.toml").resolve()
        if not member_path.is_relative_to(root.resolve()) or not member_path.is_file():
            raise ValueError(f"{name}: invalid workspace path {dependency['path']}")
        member = tomllib.loads(member_path.read_text(encoding="utf-8"))
        if member["package"]["name"] != name:
            raise ValueError(f"{name}: package name differs from path member")
        actual = member["package"]["version"]
        expected = dependency["version"]
        if version_tuple(actual) != version_tuple(expected):
            updates[name] = (expected, actual)

    if not updates:
        print("OK: workspace dependency versions agree with member manifests")
        return 0

    for name, (old, new) in sorted(updates.items()):
        print(f"{name}: workspace requirement {old} -> {new}", file=sys.stderr)

    if not write:
        print("ERROR: run scripts/sync_workspace_versions.py --write", file=sys.stderr)
        return 1

    # Only touch [workspace.dependencies], preserving formatting and comments.
    head, sep, rest = manifest.partition("[workspace.dependencies]")
    if not sep:
        raise ValueError("Missing [workspace.dependencies]")
    section, next_section, tail = rest.partition("\n[")
    for name, (old, new) in updates.items():
        pattern = re.compile(
            rf'(?m)^(\s*{re.escape(name)}\s*=\s*\{{[^\n]*\bversion\s*=\s*")'
            rf'{re.escape(old)}(")'
        )
        section, count = pattern.subn(lambda m: m[1] + new + m[2], section)
        if count != 1:
            raise ValueError(f"{name}: expected one dependency entry, found {count}")
    manifest_path.write_text(
        head + sep + section + ("\n[" + tail if next_section else ""),
        encoding="utf-8",
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="repair outdated requirements")
    mode.add_argument("--check", action="store_true", help="verify without modifying")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    try:
        return synchronize(args.root.resolve(), args.write)
    except (ValueError, KeyError, OSError, tomllib.TOMLDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
