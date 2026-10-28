//! The source over TLS, against a loopback server with the fixture certificate.

use meridian_asset_app::{AssetPath, ByteSource, SourceError};

use crate::HttpSource;
use crate::test_tls_server::{CERTIFICATE, serve_tls};

fn path(text: &str) -> AssetPath {
    AssetPath::new(text)
}

#[test]
fn reads_over_tls_from_a_server_whose_certificate_is_trusted() {
    let (base, seen) = serve_tls(vec![("/tex/a.bin", "200 OK", "", vec![4, 5, 6])]);
    let source = HttpSource::new(&base)
        .expect("https url")
        .with_root_certificate(CERTIFICATE);
    assert_eq!(source.read(&path("tex/a.bin")), Ok(vec![4, 5, 6]));
    assert_eq!(*seen.lock().expect("log"), ["/tex/a.bin"]);
}

#[test]
fn a_certificate_the_client_does_not_trust_is_unreadable_and_nothing_is_sent() {
    let (base, seen) = serve_tls(vec![("/a", "200 OK", "", vec![1])]);
    let source = HttpSource::new(&base).expect("https url");
    let Err(SourceError::Unreadable { reason, .. }) = source.read(&path("a")) else {
        panic!("an untrusted certificate must fail the request");
    };
    assert!(!reason.is_empty());
    assert!(seen.lock().expect("log").is_empty());
}

#[test]
fn a_timeout_set_after_a_certificate_keeps_the_certificate() {
    let (base, _) = serve_tls(vec![("/a", "200 OK", "", vec![2])]);
    let source = HttpSource::new(&base)
        .expect("https url")
        .with_root_certificate(CERTIFICATE)
        .with_timeout(std::time::Duration::from_secs(10));
    assert_eq!(source.read(&path("a")), Ok(vec![2]));
}
