#!/usr/bin/env python3
"""Gate on the clippy warning backlog: counts per lint may only go down.

The workspace lint set (root Cargo.toml) is Craftbook's. Lints the tree
already satisfies are `deny`; the rest are `warn` while the migration works
them off (Craftbook adoption page, waves 1-7). This script runs clippy once
and compares the count of each warning with `.clippy-backlog`:

  python3 scripts/clippy_backlog.py           # fail if any count went up
  python3 scripts/clippy_backlog.py --write   # record the current counts

A migration change runs `--write` after its fixes, so the file only shrinks.
"""
import collections
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = os.path.join(ROOT, ".clippy-backlog")
COMMAND = [
    "cargo", "clippy", "--workspace", "--all-targets", "--all-features",
    "--message-format=json", "--quiet",
]


def current_counts():
    run = subprocess.run(COMMAND, cwd=ROOT, capture_output=True, text=True)
    counts = collections.Counter()
    seen = set()
    errors = 0
    for line in run.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message["message"]
        if diagnostic["level"] == "error":
            errors += 1
            sys.stderr.write(diagnostic.get("rendered") or diagnostic["message"])
            continue
        if not diagnostic.get("code"):
            continue
        span = diagnostic["spans"][0] if diagnostic["spans"] else {}
        key = (diagnostic["code"]["code"], span.get("file_name"),
               span.get("line_start"), span.get("column_start"))
        if key not in seen:
            seen.add(key)
            counts[diagnostic["code"]["code"]] += 1
    if run.returncode != 0 and errors == 0:
        sys.stderr.write(run.stderr)
        errors = 1
    return counts, errors


def read_baseline():
    counts = {}
    if os.path.exists(BASELINE):
        for line in open(BASELINE):
            line = line.strip()
            if line and not line.startswith("#"):
                lint, count = line.split("\t")
                counts[lint] = int(count)
    return counts


def write_baseline(counts):
    with open(BASELINE, "w") as fh:
        fh.write("# clippy warnings per lint; may only shrink (scripts/clippy_backlog.py)\n")
        for lint, count in sorted(counts.items()):
            fh.write(f"{lint}\t{count}\n")


def main():
    counts, errors = current_counts()
    if errors:
        print(f"clippy failed with {errors} error(s)")
        return 1
    if "--write" in sys.argv[1:]:
        write_baseline(counts)
        print(f"wrote {BASELINE}: {sum(counts.values())} warning(s)")
        return 0
    baseline = read_baseline()
    worse = [(lint, count, baseline.get(lint, 0)) for lint, count in sorted(counts.items())
             if count > baseline.get(lint, 0)]
    for lint, count, allowed in worse:
        print(f"{lint}: {count} warning(s), backlog allows {allowed}")
    if worse:
        return 1
    print(f"OK: {sum(counts.values())} warning(s), backlog {sum(baseline.values())}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
