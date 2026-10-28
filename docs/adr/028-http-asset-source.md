# ADR 028: HTTP asset source without TLS

## Status

Accepted. Follows [ADR 027](027-background-asset-loading.md) D4, which expected
an HTTP source to use `tokio` inside the adapter; it does not need to.

## Context

ADR 027 left a network `ByteSource` open. `ByteSource::read` is blocking and
runs on a `BackgroundPool` thread, so the adapter is a blocking HTTP client.
`tokio` would add a runtime handle and a `block_on` on a pool thread for
nothing, and `async` stays reserved for the frame-side I/O it was adopted for
(ADR 009).

TLS is the hard part. `ureq`'s `rustls` feature pulls in `ring`, whose licence
(`ISC AND MIT AND OpenSSL`) is not in `deny.toml`'s allow list, and `deny.toml`
says a licence is added by a Craftbook PR, not locally. `cargo deny` runs with
`all-features = true`, so an optional `tls` feature would fail the gate as well.

## Decision

**D1 — `meridian-asset-http`.** An infrastructure crate in the assets context,
depending only on `meridian-asset-app` (plus `ureq` without TLS,
`percent-encoding`, `thiserror`). `HttpSource` implements `ByteSource` for
`http://host[:port][/prefix]`.

**D2 — plain `http` only, and loud about it.** `HttpSource::new` returns
`HttpSourceError::UnsupportedScheme` for `https://` instead of downgrading. It
is for a LAN, a loopback dev server or an asset proxy that terminates TLS. Real
`https` waits for a decision on the `ring` licence (or a crypto provider with an
allowed one); then only `Cargo.toml` and the scheme check change.

**D3 — a hostile server cannot hold a thread or fill memory.** Redirects are
not followed, a request has a global timeout (30 s), a body over a cap (64 MiB)
is refused, and the status maps to the port's errors: 404 and 410 are
`NotFound`, anything else outside 2xx is `Unreadable`.

**D4 — the path cannot leave the base.** Like `FsSource`: an empty path, an
empty segment or `.` / `..` is `Rejected` without a request; every other byte
outside RFC 3986's unreserved set is percent-encoded.

## Consequences

- No asset served over `https` yet; the error says so at construction.
- The crate is tested against a one-thread local server in its own tests
  (no external network).
- `sdk` re-exports `HttpSource`; a game builds
  `AssetLoader::new(Arc::new(HttpSource::new(url)?), Arc::new(BackgroundPool::new(n)))`.
