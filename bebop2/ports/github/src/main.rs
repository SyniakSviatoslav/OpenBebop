//! `bebop-github-webhook` — runnable inbound GitHub webhook daemon.
//!
//! Usage:
//!   GITHUB_WEBHOOK_SECRET=… [GITHUB_WEBHOOK_PATH=/hook] bebop-github-webhook [BIND_ADDR]
//!
//! Refuses to start without a secret — the HMAC gate is the only thing standing
//! between GitHub and the sink, so an empty secret is a configuration error, not
//! a default.

use bebop_port_github::{GithubEvent, GithubWebhook};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let secret = match std::env::var("GITHUB_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            eprintln!("fatal: set GITHUB_WEBHOOK_SECRET (non-empty) — the webhook is fail-closed");
            std::process::exit(2);
        }
    };
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let path = std::env::var("GITHUB_WEBHOOK_PATH").unwrap_or_else(|_| "/".to_string());

    eprintln!("bebop-github-webhook: listening on {addr}{path} (fail-closed HMAC-SHA256)");
    GithubWebhook::new(secret.into_bytes())
        .path(path)
        .serve(&addr, |ev: GithubEvent| {
            println!(
                "event={} delivery={} bytes={}",
                ev.event,
                ev.delivery,
                ev.payload.len()
            );
        })
        .await
}
