//! The source against a local server.

use std::time::Duration;

use meridian_asset_app::{AssetPath, ByteSource, SourceError};

use crate::test_server::serve;
use crate::{HttpSource, HttpSourceError};

fn path(text: &str) -> AssetPath {
    AssetPath::new(text)
}

#[test]
fn reads_the_body_of_a_200() {
    let (base, _) = serve(vec![("/tex/a.bin", "200 OK", "", vec![1, 2, 3])]);
    let source = HttpSource::new(&base).expect("http url");
    assert_eq!(source.read(&path("tex/a.bin")), Ok(vec![1, 2, 3]));
}

#[test]
fn a_base_url_prefix_and_trailing_slash_are_joined_once() {
    let (base, seen) = serve(vec![("/assets/a.bin", "200 OK", "", vec![9])]);
    let source = HttpSource::new(&format!("{base}/assets/")).expect("http url");
    assert_eq!(source.read(&path("a.bin")), Ok(vec![9]));
    assert_eq!(*seen.lock().expect("log"), ["/assets/a.bin"]);
}

#[test]
fn unusual_bytes_in_a_path_are_percent_encoded() {
    let (base, seen) = serve(vec![("/my%20pic%3F.png", "200 OK", "", vec![7])]);
    let source = HttpSource::new(&base).expect("http url");
    assert_eq!(source.read(&path("my pic?.png")), Ok(vec![7]));
    assert_eq!(*seen.lock().expect("log"), ["/my%20pic%3F.png"]);
}

#[test]
fn a_404_is_not_found_and_other_statuses_are_unreadable() {
    let (base, _) = serve(vec![("/boom", "500 Internal Server Error", "", Vec::new())]);
    let source = HttpSource::new(&base).expect("http url");
    assert_eq!(
        source.read(&path("absent")),
        Err(SourceError::NotFound(path("absent")))
    );
    let Err(SourceError::Unreadable { reason, .. }) = source.read(&path("boom")) else {
        panic!("a 500 must be unreadable");
    };
    assert!(reason.contains("500"), "{reason}");
}

#[test]
fn a_redirect_is_not_followed() {
    let (base, seen) = serve(vec![(
        "/moved",
        "302 Found",
        "Location: /elsewhere\r\n",
        Vec::new(),
    )]);
    let source = HttpSource::new(&base).expect("http url");
    let Err(SourceError::Unreadable { reason, .. }) = source.read(&path("moved")) else {
        panic!("a redirect must not be followed");
    };
    assert!(reason.contains("302"), "{reason}");
    assert_eq!(*seen.lock().expect("log"), ["/moved"]);
}

#[test]
fn a_body_over_the_cap_is_refused() {
    let (base, _) = serve(vec![("/big", "200 OK", "", vec![0; 1000])]);
    let source = HttpSource::new(&base)
        .expect("http url")
        .with_max_bytes(100);
    assert!(matches!(
        source.read(&path("big")),
        Err(SourceError::Unreadable { .. })
    ));
}

#[test]
fn a_path_that_could_leave_the_base_is_rejected_without_a_request() {
    let (base, seen) = serve(Vec::new());
    let source = HttpSource::new(&base).expect("http url");
    for bad in ["", "../x", "a/../x", "a//b", "/abs", "a/", "./a"] {
        assert_eq!(
            source.read(&path(bad)),
            Err(SourceError::Rejected(path(bad))),
            "{bad:?}"
        );
    }
    assert!(seen.lock().expect("log").is_empty());
}

#[test]
fn an_unreachable_server_is_unreadable_not_a_panic() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("address"));
    drop(listener);
    let source = HttpSource::new(&base)
        .expect("http url")
        .with_timeout(Duration::from_secs(5));
    assert!(matches!(
        source.read(&path("a")),
        Err(SourceError::Unreadable { .. })
    ));
}

#[test]
fn only_plain_http_base_urls_are_accepted() {
    assert_eq!(
        HttpSource::new("https://example.com").unwrap_err(),
        HttpSourceError::UnsupportedScheme("https://example.com".to_owned())
    );
    assert!(matches!(
        HttpSource::new("ftp://x").unwrap_err(),
        HttpSourceError::UnsupportedScheme(_)
    ));
    assert!(matches!(
        HttpSource::new("http://").unwrap_err(),
        HttpSourceError::MissingHost(_)
    ));
    assert!(matches!(
        HttpSource::new("http:///assets").unwrap_err(),
        HttpSourceError::MissingHost(_)
    ));
}
