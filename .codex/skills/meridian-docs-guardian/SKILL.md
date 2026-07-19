---
name: meridian-docs-guardian
description: Maintain Meridian-Engine documentation and architecture text. Use when editing README.md, AGENTS.md, CLAUDE.md, docs/**, architecture comments, ADRs, roadmap/status text, or Cargo.toml descriptions that explain crate responsibility.
---

# Meridian Docs Guardian

## Workflow

1. Read `AGENTS.md`, then `docs/dependency-rules.md` if the text touches architecture, layering, crate boundaries, or dependency direction.
2. Read the narrow subsystem doc(s) for the changed area and any referenced ADR.
3. Check `docs/roadmap.md` before writing implementation-status claims.
4. Keep docs precise and falsifiable. Avoid claims like "all crates are scaffolds" unless verified against code.
5. If a decision changes, add or update an ADR. Do not bury permanent rationale only in comments.
6. If code changes invalidate docs, update docs in the same patch.

## Validation

- Run `./build.sh check-deps` when dependency rules or crate boundaries are mentioned.
- Run `cargo test --workspace` when docs describe behavior, examples, or current implementation status.
- Check Markdown links manually when adding or moving document references.
