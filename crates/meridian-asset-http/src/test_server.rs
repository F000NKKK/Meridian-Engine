//! A one-thread HTTP server for the tests: fixed answers by request path.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

/// One route: path, status line, extra header lines, body.
pub(crate) type Route = (&'static str, &'static str, &'static str, Vec<u8>);

/// The request paths a server saw, in order.
pub(crate) type Seen = Arc<Mutex<Vec<String>>>;

/// Serves `routes` on a free local port until the process ends. Returns the
/// base URL and the request paths it saw.
pub(crate) fn serve(routes: Vec<Route>) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let base = format!("http://{}", listener.local_addr().expect("local address"));
    let seen = Seen::default();
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            answer(stream, &routes, &log);
        }
    });
    (base, seen)
}

/// Reads one request from `stream` and writes the matching route's answer.
pub(crate) fn answer(mut stream: impl Read + Write, routes: &[Route], log: &Seen) {
    let mut request = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        if reader.read_line(&mut request).is_err() {
            return;
        }
        // Drain the headers so the client sees an orderly close.
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) if line == "\r\n" => break,
                Ok(_) => {}
            }
        }
    }
    let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
    log.lock().expect("log lock").push(path.clone());
    let (status, headers, body) = routes
        .iter()
        .find(|route| route.0 == path)
        .map_or(("404 Not Found", "", Vec::new()), |route| {
            (route.1, route.2, route.3.clone())
        });
    let head = format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    // A client that hung up early is the test's business, not the server's.
    let sent = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(&body))
        .and_then(|()| stream.flush());
    drop(sent);
}
