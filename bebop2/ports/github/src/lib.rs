//! `bebop-port-github` — inbound GitHub webhook receiver (external-adapter port).
//!
//! ## Trust boundary
//! GitHub is a **non-bebop** sender: it cannot produce a signed [`SignedFrame`],
//! so the sovereign hybrid gate does not apply to it directly. Instead the port
//! authenticates every delivery the way GitHub itself specifies — an
//! **HMAC-SHA256** over the *raw request body*, keyed by the shared webhook
//! secret, delivered in the `X-Hub-Signature-256` header as `sha256=<hex>`.
//!
//! The verification is **constant-time** (`ring::hmac::verify`) and
//! **fail-closed**: a missing, malformed, or wrong signature is rejected with
//! `401` and the sink is **never** called. The port neither scores nor ranks the
//! sender — trust here is the shared secret (a capability), not reputation.
//!
//! A verified delivery is handed to a [`WebhookSink`] as a [`GithubEvent`]
//! carrying the raw, *unparsed* JSON payload. The port does not interpret GitHub
//! payloads and never mints a `SignedFrame` on GitHub's behalf; re-minting into a
//! sovereign frame (under this port's own anchored capability) is a deliberate
//! follow-up, not something faked here.
//!
//! [`SignedFrame`]: https://docs.rs/bebop-proto-cap

use std::sync::Arc;

use ring::hmac;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// GitHub's documented maximum webhook payload size (25 MiB). Bodies larger than
/// this are rejected with `413` before the whole body is buffered.
pub const DEFAULT_MAX_BODY: usize = 25 * 1024 * 1024;

/// Header section cap — a request whose headers exceed this is refused (`431`)
/// rather than buffered without bound.
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// A verified GitHub webhook delivery. The `payload` is the exact raw body bytes
/// the HMAC was computed over — never re-serialized.
#[derive(Debug, Clone)]
pub struct GithubEvent {
    /// `X-GitHub-Event` — the event name, e.g. `"push"`, `"pull_request"`, `"ping"`.
    pub event: String,
    /// `X-GitHub-Delivery` — the unique delivery GUID (empty if absent).
    pub delivery: String,
    /// The raw, verified JSON body. Left unparsed on purpose (no `serde` dep).
    pub payload: Vec<u8>,
}

/// Sink that receives a delivery **after** its signature has been verified. The
/// port calls this only on the success path.
pub trait WebhookSink: Send + Sync {
    fn on_event(&self, event: GithubEvent);
}

/// Any `Fn(GithubEvent)` is a sink.
impl<F> WebhookSink for F
where
    F: Fn(GithubEvent) + Send + Sync,
{
    fn on_event(&self, event: GithubEvent) {
        self(event)
    }
}

/// The HTTP status the port decided for a request. Kept as a plain code so the
/// verification logic is pure and unit-testable without sockets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
}

impl Response {
    const fn new(status: u16) -> Self {
        Self { status }
    }
}

/// Configured GitHub webhook receiver.
pub struct GithubWebhook {
    secret: Vec<u8>,
    path: String,
    max_body: usize,
}

impl GithubWebhook {
    /// Build a receiver for the given shared secret. Default path `/`, default
    /// body cap [`DEFAULT_MAX_BODY`].
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
            path: "/".to_string(),
            max_body: DEFAULT_MAX_BODY,
        }
    }

    /// Only accept POSTs whose path (query stripped) equals `path`; others `404`.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Override the maximum accepted body size.
    pub fn max_body(mut self, max_body: usize) -> Self {
        self.max_body = max_body;
        self
    }

    /// Bind `addr` and serve forever, spawning a task per connection.
    pub async fn serve<S>(self, addr: &str, sink: S) -> std::io::Result<()>
    where
        S: WebhookSink + 'static,
    {
        let listener = TcpListener::bind(addr).await?;
        self.serve_listener(listener, sink).await
    }

    /// Serve on an already-bound listener (lets callers pick an ephemeral port
    /// and learn the address first — used by the integration test).
    pub async fn serve_listener<S>(self, listener: TcpListener, sink: S) -> std::io::Result<()>
    where
        S: WebhookSink + 'static,
    {
        let cfg = Arc::new(self);
        let sink = Arc::new(sink);
        loop {
            let (stream, _peer) = listener.accept().await?;
            let cfg = Arc::clone(&cfg);
            let sink = Arc::clone(&sink);
            tokio::spawn(async move {
                let _ = serve_conn(&cfg, sink.as_ref(), stream).await;
            });
        }
    }
}

/// Verify GitHub's `X-Hub-Signature-256` over `body` with `secret`.
///
/// Returns `true` only if the header is present, exactly `sha256=<hex>`, decodes
/// to a valid MAC, and matches — checked in constant time. Everything else is
/// `false` (fail-closed).
pub fn verify_signature(secret: &[u8], body: &[u8], signature_header: Option<&str>) -> bool {
    let Some(header) = signature_header else {
        return false;
    };
    let Some(hex) = header.strip_prefix("sha256=") else {
        return false;
    };
    let Some(expected) = decode_hex(hex) else {
        return false;
    };
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    hmac::verify(&key, body, &expected).is_ok()
}

/// The pure decision: parse `raw`, and either reject (fail-closed) or verify and
/// dispatch to `sink`. No I/O — the socket layer only feeds bytes in and writes
/// the status out. `sink` is invoked **iff** the signature verifies.
pub fn handle_request(
    secret: &[u8],
    expected_path: &str,
    raw: &[u8],
    sink: &dyn WebhookSink,
) -> Response {
    let Ok(req) = parse_request(raw) else {
        return Response::new(400);
    };
    if !req.method.eq_ignore_ascii_case("POST") {
        return Response::new(405);
    }
    if path_without_query(&req.path) != expected_path {
        return Response::new(404);
    }
    if !verify_signature(secret, &req.body, req.header("x-hub-signature-256")) {
        // Fail-closed: sink is NOT reached.
        return Response::new(401);
    }
    sink.on_event(GithubEvent {
        event: req.header("x-github-event").unwrap_or("").to_string(),
        delivery: req.header("x-github-delivery").unwrap_or("").to_string(),
        payload: req.body,
    });
    Response::new(204)
}

// ── HTTP/1.1 (minimal, one POST) ─────────────────────────────────────────────

struct ParsedRequest {
    method: String,
    path: String,
    /// Header names are lowercased; values trimmed.
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ParsedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

struct ParseError;

/// Parse a full request buffer (headers + body already read). Fail-closed on any
/// structural problem.
fn parse_request(raw: &[u8]) -> Result<ParsedRequest, ParseError> {
    let sep = find_subslice(raw, b"\r\n\r\n").ok_or(ParseError)?;
    let head = &raw[..sep];
    let body = raw[sep + 4..].to_vec();

    let head = std::str::from_utf8(head).map_err(|_| ParseError)?;
    let mut lines = head.split("\r\n");

    let request_line = lines.next().ok_or(ParseError)?;
    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or(ParseError)?.to_string();
    let path = parts.next().ok_or(ParseError)?.to_string();
    // parts.next() would be the HTTP version — unused.

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let colon = line.find(':').ok_or(ParseError)?;
        let name = line[..colon].trim().to_ascii_lowercase();
        let value = line[colon + 1..].trim().to_string();
        headers.push((name, value));
    }

    Ok(ParsedRequest {
        method,
        path,
        headers,
        body,
    })
}

fn path_without_query(path: &str) -> &str {
    match path.find('?') {
        Some(i) => &path[..i],
        None => path,
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Decode a lowercase/uppercase hex string. `None` on odd length or non-hex.
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    Some(out)
}

// ── Socket layer ─────────────────────────────────────────────────────────────

enum ReadError {
    BodyTooLarge,
    HeaderTooLarge,
    Malformed,
}

async fn serve_conn(
    cfg: &GithubWebhook,
    sink: &dyn WebhookSink,
    mut stream: TcpStream,
) -> std::io::Result<()> {
    let raw = match read_request(&mut stream, cfg.max_body).await {
        Ok(raw) => raw,
        Err(ReadError::BodyTooLarge) => return write_status(&mut stream, 413).await,
        Err(ReadError::HeaderTooLarge) => return write_status(&mut stream, 431).await,
        Err(ReadError::Malformed) => return write_status(&mut stream, 400).await,
    };
    let resp = handle_request(&cfg.secret, &cfg.path, &raw, sink);
    write_status(&mut stream, resp.status).await
}

/// Read exactly one HTTP request: headers up to the blank line, then
/// `Content-Length` body bytes — both bounded so a hostile client cannot make us
/// buffer without limit.
async fn read_request(stream: &mut TcpStream, max_body: usize) -> Result<Vec<u8>, ReadError> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];

    let header_end = loop {
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err(ReadError::HeaderTooLarge);
        }
        let n = stream.read(&mut tmp).await.map_err(|_| ReadError::Malformed)?;
        if n == 0 {
            return Err(ReadError::Malformed);
        }
        buf.extend_from_slice(&tmp[..n]);
    };

    let content_length = content_length(&buf[..header_end]).unwrap_or(0);
    if content_length > max_body {
        return Err(ReadError::BodyTooLarge);
    }
    let total = header_end + content_length;

    while buf.len() < total {
        let n = stream.read(&mut tmp).await.map_err(|_| ReadError::Malformed)?;
        if n == 0 {
            break; // client closed early — parse_request will fail-close.
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    buf.truncate(total.min(buf.len()));
    Ok(buf)
}

fn content_length(head: &[u8]) -> Option<usize> {
    let head = std::str::from_utf8(head).ok()?;
    for line in head.split("\r\n") {
        if let Some(colon) = line.find(':') {
            if line[..colon].trim().eq_ignore_ascii_case("content-length") {
                return line[colon + 1..].trim().parse().ok();
            }
        }
    }
    None
}

async fn write_status(stream: &mut TcpStream, status: u16) -> std::io::Result<()> {
    let reason = reason_phrase(status);
    let response =
        format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    let _ = stream.shutdown().await;
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records deliveries so a test can assert whether the sink was reached.
    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<GithubEvent>>,
    }
    impl WebhookSink for RecordingSink {
        fn on_event(&self, event: GithubEvent) {
            self.events.lock().unwrap().push(event);
        }
    }
    impl RecordingSink {
        fn count(&self) -> usize {
            self.events.lock().unwrap().len()
        }
    }

    const SECRET: &[u8] = b"It's a Secret to Everybody";

    /// The real GitHub scheme: `sha256=` + hex(HMAC-SHA256(secret, body)).
    fn sign(secret: &[u8], body: &[u8]) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let tag = hmac::sign(&key, body);
        let mut hex = String::from("sha256=");
        for b in tag.as_ref() {
            hex.push_str(&format!("{b:02x}"));
        }
        hex
    }

    fn post(path: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
        let mut req = format!("POST {path} HTTP/1.1\r\nHost: localhost\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        let mut raw = req.into_bytes();
        raw.extend_from_slice(body);
        raw
    }

    // ── The security property: only a correct signature is accepted ──────────

    #[test]
    fn valid_signature_accepted_and_sink_called() {
        let body = br#"{"zen":"Keep it logically awesome."}"#;
        let sig = sign(SECRET, body);
        let raw = post("/", &[("X-GitHub-Event", "push"), ("X-Hub-Signature-256", &sig)], body);
        let sink = RecordingSink::default();

        let resp = handle_request(SECRET, "/", &raw, &sink);

        assert_eq!(resp.status, 204);
        assert_eq!(sink.count(), 1);
        assert_eq!(sink.events.lock().unwrap()[0].event, "push");
        assert_eq!(sink.events.lock().unwrap()[0].payload, body);
    }

    #[test]
    fn tampered_body_rejected_and_sink_not_called() {
        let signed_body = br#"{"action":"opened"}"#;
        let sig = sign(SECRET, signed_body); // signature is for signed_body …
        let delivered_body = br#"{"action":"closed"}"#; // … but a different body arrives
        let raw = post("/", &[("X-Hub-Signature-256", &sig)], delivered_body);
        let sink = RecordingSink::default();

        let resp = handle_request(SECRET, "/", &raw, &sink);

        assert_eq!(resp.status, 401);
        assert_eq!(sink.count(), 0, "tampered delivery must never reach the sink");
    }

    #[test]
    fn wrong_secret_rejected() {
        let body = br#"{"ok":true}"#;
        let sig = sign(b"the-attacker-secret", body);
        let raw = post("/", &[("X-Hub-Signature-256", &sig)], body);
        let sink = RecordingSink::default();

        let resp = handle_request(SECRET, "/", &raw, &sink);

        assert_eq!(resp.status, 401);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn missing_signature_header_rejected() {
        let body = br#"{"ok":true}"#;
        let raw = post("/", &[("X-GitHub-Event", "push")], body);
        let sink = RecordingSink::default();

        let resp = handle_request(SECRET, "/", &raw, &sink);

        assert_eq!(resp.status, 401);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn malformed_signature_scheme_rejected() {
        let body = br#"{"ok":true}"#;
        // GitHub's deprecated SHA-1 header is not accepted — only sha256=.
        let raw = post("/", &[("X-Hub-Signature", "sha1=deadbeef")], body);
        let sink = RecordingSink::default();

        assert_eq!(handle_request(SECRET, "/", &raw, &sink).status, 401);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn ping_event_accepted() {
        let body = br#"{"zen":"Non-blocking is better than blocking.","hook_id":42}"#;
        let sig = sign(SECRET, body);
        let raw = post("/", &[("X-GitHub-Event", "ping"), ("X-Hub-Signature-256", &sig)], body);
        let sink = RecordingSink::default();

        assert_eq!(handle_request(SECRET, "/", &raw, &sink).status, 204);
        assert_eq!(sink.events.lock().unwrap()[0].event, "ping");
    }

    #[test]
    fn non_post_method_rejected() {
        let sink = RecordingSink::default();
        let raw = b"GET / HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(handle_request(SECRET, "/", raw, &sink).status, 405);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn wrong_path_rejected() {
        let body = br#"{"ok":true}"#;
        let sig = sign(SECRET, body);
        let raw = post("/other", &[("X-Hub-Signature-256", &sig)], body);
        let sink = RecordingSink::default();
        assert_eq!(handle_request(SECRET, "/webhook", &raw, &sink).status, 404);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn configured_path_with_query_matches() {
        let body = br#"{"ok":true}"#;
        let sig = sign(SECRET, body);
        let raw = post("/webhook?x=1", &[("X-Hub-Signature-256", &sig)], body);
        let sink = RecordingSink::default();
        assert_eq!(handle_request(SECRET, "/webhook", &raw, &sink).status, 204);
    }

    #[test]
    fn malformed_http_rejected() {
        let sink = RecordingSink::default();
        let raw = b"not http at all";
        assert_eq!(handle_request(SECRET, "/", raw, &sink).status, 400);
        assert_eq!(sink.count(), 0);
    }

    #[test]
    fn verify_signature_unit() {
        let body = b"payload";
        let good = sign(SECRET, body);
        assert!(verify_signature(SECRET, body, Some(&good)));
        assert!(!verify_signature(SECRET, b"other", Some(&good)));
        assert!(!verify_signature(SECRET, body, None));
        assert!(!verify_signature(SECRET, body, Some("sha256=zz")));
        assert!(!verify_signature(SECRET, body, Some("nothex")));
    }

    #[test]
    fn decode_hex_rejects_bad_input() {
        assert_eq!(decode_hex("00ff"), Some(vec![0x00, 0xff]));
        assert!(decode_hex("abc").is_none()); // odd length
        assert!(decode_hex("gg").is_none()); // non-hex
    }
}
