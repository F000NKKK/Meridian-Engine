# ADR 028: HTTP asset source

## Status

Accepted. Follows [ADR 027](027-background-asset-loading.md) D4, which expected
an HTTP source to use `tokio` inside the adapter; it does not need to.
Amended 2026-10-28: `https` is supported (D2), after the licence allow-list grew.

## Context

ADR 027 left a network `ByteSource` open. `ByteSource::read` is blocking and
runs on a `BackgroundPool` thread, so the adapter is a blocking HTTP client.
`tokio` would add a runtime handle and a `block_on` on a pool thread for
nothing, and `async` stays reserved for the frame-side I/O it was adopted for
(ADR 009).

TLS was the hard part. `ureq`'s `rustls` feature pulls in `ring` and the
Mozilla root bundle (`webpki-roots`). The first version shipped without TLS
because `deny.toml` did not allow the licences involved (OpenSSL on older `ring`
releases, `CDLA-Permissive-2.0` for the roots) and a licence is added by a
Craftbook PR, not locally. The owner then allowed every attribution-only
permissive licence (Craftbook `templates/deny.toml`, 2026-10-28), and the latest
`ring` (0.17.14) is `Apache-2.0 AND ISC` anyway. `cargo deny` runs with
`all-features = true`, so the feature set cannot hide a dependency from the gate.

## Decision

**D1 — `meridian-asset-http`.** An infrastructure crate in the assets context,
depending only on `meridian-asset-app` (plus `ureq` with `rustls`,
`percent-encoding`, `thiserror`). `HttpSource` implements `ByteSource` for
`http[s]://host[:port][/prefix]`.

**D2 — `http` and `https`; nothing else.** `HttpSource::new` accepts both
schemes and returns `HttpSourceError::UnsupportedScheme` for any other. Over
`https` the server certificate is checked against the Mozilla roots; a server
with a private or self-signed certificate is trusted explicitly with
`with_root_certificate(der)`, which replaces the public roots for that source.
There is no switch that disables verification, and no downgrade from `https` to
`http`.

**D3 — a hostile server cannot hold a thread or fill memory.** Redirects are
not followed, a request has a global timeout (30 s), a body over a cap (64 MiB)
is refused, and the status maps to the port's errors: 404 and 410 are
`NotFound`, anything else outside 2xx is `Unreadable`.

**D4 — the path cannot leave the base.** Like `FsSource`: an empty path, an
empty segment or `.` / `..` is `Rejected` without a request; every other byte
outside RFC 3986's unreserved set is percent-encoded.

## Consequences

- The TLS stack (`rustls`, `ring`, `webpki-roots`) is now in the build of every
  binary that depends on `sdk`. The roots are compiled in, so they age with the
  release: a long-lived game updates them by rebuilding.
- The crate is tested against one-thread loopback servers in its own tests
  (no external network): plain, and TLS with a 100-year self-signed fixture
  certificate in `testdata/` (a public test key on purpose).
- `sdk` re-exports `HttpSource`; a game builds
  `AssetLoader::new(Arc::new(HttpSource::new(url)?), Arc::new(BackgroundPool::new(n)))`.
