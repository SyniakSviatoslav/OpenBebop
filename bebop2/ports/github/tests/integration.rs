//! End-to-end proof over a real TCP socket: a correctly-signed POST is accepted
//! and reaches the sink; a tampered POST is rejected and never does.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bebop_port_github::{GithubEvent, GithubWebhook};
use ring::hmac;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SECRET: &[u8] = b"integration-secret";

fn sign(secret: &[u8], body: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let tag = hmac::sign(&key, body);
    let mut hex = String::from("sha256=");
    for b in tag.as_ref() {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

fn post(headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut req = format!("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n", body.len());
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    let mut raw = req.into_bytes();
    raw.extend_from_slice(body);
    raw
}

async fn send(addr: std::net::SocketAddr, raw: &[u8]) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(raw).await.unwrap();
    stream.flush().await.unwrap();
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).await.unwrap();
    String::from_utf8_lossy(&resp).into_owned()
}

#[tokio::test]
async fn signed_delivery_accepted_tampered_rejected_over_tcp() {
    let count = Arc::new(AtomicUsize::new(0));
    let last = Arc::new(Mutex::new(String::new()));
    let (c, l) = (Arc::clone(&count), Arc::clone(&last));
    let sink = move |ev: GithubEvent| {
        c.fetch_add(1, Ordering::SeqCst);
        *l.lock().unwrap() = ev.event;
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(GithubWebhook::new(SECRET).serve_listener(listener, sink));

    // 1. Correctly-signed delivery → 204, sink reached with the right event.
    let body = br#"{"zen":"Anything added dilutes everything else."}"#;
    let sig = sign(SECRET, body);
    let ok = send(addr, &post(&[("X-GitHub-Event", "push"), ("X-Hub-Signature-256", &sig)], body)).await;
    assert!(ok.starts_with("HTTP/1.1 204"), "expected 204, got: {ok}");
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(*last.lock().unwrap(), "push");

    // 2. Tampered body under the same signature → 401, sink NOT reached again.
    let tampered = send(
        addr,
        &post(&[("X-Hub-Signature-256", &sig)], br#"{"zen":"tampered"}"#),
    )
    .await;
    assert!(tampered.starts_with("HTTP/1.1 401"), "expected 401, got: {tampered}");
    assert_eq!(count.load(Ordering::SeqCst), 1, "tampered delivery must not reach the sink");
}
