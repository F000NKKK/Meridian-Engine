//! A loopback `https` server for the tests, with the fixture certificate.

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use crate::test_server::{Route, Seen, answer};

/// The fixture certificate (DER) a client must trust to talk to [`serve_tls`].
pub(crate) const CERTIFICATE: &[u8] = include_bytes!("../testdata/cert.der");
const KEY: &[u8] = include_bytes!("../testdata/key.der");

/// Serves `routes` over TLS on a free local port until the process ends.
/// Returns the `https://` base URL and the request paths it saw.
pub(crate) fn serve_tls(routes: Vec<Route>) -> (String, Seen) {
    let config = Arc::new(server_config());
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let base = format!("https://{}", listener.local_addr().expect("local address"));
    let seen = Seen::default();
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for tcp in listener.incoming().flatten() {
            let Ok(connection) = ServerConnection::new(Arc::clone(&config)) else {
                continue;
            };
            answer(StreamOwned::new(connection, tcp), &routes, &log);
        }
    });
    (base, seen)
}

fn server_config() -> ServerConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let chain = vec![CertificateDer::from(CERTIFICATE)];
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(KEY));
    ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .expect("fixture certificate and key")
}
