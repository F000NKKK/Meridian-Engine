# Test fixtures

`cert.der` is a self-signed server certificate for `localhost` / `127.0.0.1`
(RSA 2048, valid 100 years, not a CA) and `key.der` its PKCS#8 private key.
They only secure the loopback server in `src/test_tls_server.rs`; the key is public on purpose.
Regenerate with `openssl req -x509 -newkey rsa:2048 -nodes -days 36500 -subj /CN=localhost -addext subjectAltName=DNS:localhost,IP:127.0.0.1 -addext basicConstraints=critical,CA:FALSE -addext keyUsage=critical,digitalSignature,keyEncipherment -addext extendedKeyUsage=serverAuth`.
