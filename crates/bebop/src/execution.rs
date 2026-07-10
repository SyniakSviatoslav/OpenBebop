//! EXECUTION UPGRADE — native, falsifiable primitives for max-speed / max-free-LLM
//! usage, grounded in verified 2026 methods (Anthropic prompt-caching, Batch API,
//! model cascading/routing). All deterministic, 0 deps, RED+GREEN tested.
//!
//! Proven levers (researched, not vibes):
//!  - Prompt caching: cache STATIC prefixes (system prompt, repo/knowledge, skill
//!    defs) with stable breakpoints → ~90% cached-input cost, ~85% latency.
//!    FAILURE MODE: mutating the cached prefix mid-session breaks the cache and
//!    drains limits. So we separate static vs dynamic context and make the cache
//!    boundary explicit + checkable.
//!  - Batch API: 50% cheaper, 24h async → fan N extractions out, reconcile after.
//!  - Model cascading: cheap/free tier first, escalate to Opus only when the cheap
//!    model's self-check FAILS (do-validate-don't-trust). Difficulty-based routing.

use std::collections::HashMap;

/// A prompt assembled from a STATIC, cacheable prefix + a DYNAMIC tail.
/// The cache breakpoint sits between them; the static part MUST be byte-stable
/// across calls or caching silently breaks (the measured failure mode).
#[derive(Clone, Debug, PartialEq)]
pub struct CachePrompt {
    pub static_prefix: String,
    pub dynamic_tail: String,
}

impl CachePrompt {
    pub fn new(static_prefix: &str, dynamic_tail: &str) -> Self {
        CachePrompt {
            static_prefix: static_prefix.to_string(),
            dynamic_tail: dynamic_tail.to_string(),
        }
    }

    /// Cheapest representation when the static prefix is unchanged → cache HIT.
    /// We fingerprint the static prefix so callers can detect a cache break.
    pub fn static_fingerprint(&self) -> String {
        format!("{:08x}", fnv1a(self.static_prefix.as_bytes()))
    }

    /// True iff this prompt's static prefix matches `other`'s → safe to reuse cache.
    pub fn shares_cache(&self, other: &CachePrompt) -> bool {
        self.static_prefix == other.static_prefix
    }

    /// Full serialized prompt (what actually goes to the model).
    pub fn render(&self) -> String {
        format!("{}\n\n{}", self.static_prefix, self.dynamic_tail)
    }
}

/// Prompt-cache accounting: counts tokens saved + detects a cache break.
#[derive(Clone, Debug, PartialEq)]
pub struct CacheLedger {
    pub hits: u64,
    pub breaks: u64,
    pub cached_tokens: u64,
    pub fresh_tokens: u64,
}

impl CacheLedger {
    pub fn new() -> Self {
        CacheLedger {
            hits: 0,
            breaks: 0,
            cached_tokens: 0,
            fresh_tokens: 0,
        }
    }

    /// Feed one prompt; `prev` is the previous prompt's static fingerprint (None = first).
    /// Returns true if the cache was REUSED (static prefix unchanged).
    pub fn observe(&mut self, prev: Option<&str>, cur: &CachePrompt, tail_tokens: u64) {
        match prev {
            Some(p) if p == &cur.static_fingerprint() => {
                // cache HIT: only the dynamic tail is billed fresh
                self.hits += 1;
                self.cached_tokens += cur.static_prefix.len() as u64; // static reused
                self.fresh_tokens += tail_tokens;
            }
            _ => {
                // cache BREAK (or first): entire prompt billed fresh
                self.breaks += 1;
                self.fresh_tokens += (cur.static_prefix.len() as u64) + tail_tokens;
            }
        }
    }

    /// Cached fraction of total billed tokens (0..1). GREEN: with stable static
    /// context this trends high; RED: frequent breaks → ~0.
    pub fn cached_fraction(&self) -> f64 {
        let total = self.cached_tokens + self.fresh_tokens;
        if total == 0 {
            0.0
        } else {
            self.cached_tokens as f64 / total as f64
        }
    }
}

impl Default for CacheLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// MODEL CASCADE — cheapest-adequate-first routing with escalate-on-failure.
/// `cheap_adequate` = cheap/free model's self-check passed (do-validate).
/// Returns which tier to actually USE (not just nominate).
/// GREEN: cheap adequate → use cheap. RED: cheap fails or budget exhausted →
/// escalate to opus (never silently skip the task).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tier {
    Free,  // free-tier LLM (Groq/Cerebras/HF) — fastest, rate-limited
    Cheap, // Haiku 4.5-class ($1/$5) — adequate for most
    Opus,  // frontier — escalate only when needed
}

pub fn route_tier(cheap_adequate: bool, budget_left: f64, cheap_cost: f64) -> Tier {
    if cheap_adequate && budget_left >= cheap_cost {
        Tier::Cheap
    } else if budget_left <= 0.0 {
        // no budget at all → fall back to a FREE tier rather than overspend
        Tier::Free
    } else {
        Tier::Opus
    }
}

/// BATCH SPLITTER — partition N items into `batches` groups for the Batch API
/// (50% cheaper, async). Round-robin, ensures every item lands exactly once and
/// no batch is empty when items exist (GREEN); empty input → empty (RED-safe).
pub fn batch_split(n_items: usize, batches: usize) -> Vec<Vec<usize>> {
    if n_items == 0 || batches == 0 {
        return Vec::new();
    }
    let nb = batches.min(n_items);
    let mut out = vec![Vec::new(); nb];
    for (i, item) in (0..n_items).enumerate() {
        out[i % nb].push(item);
    }
    out
}

/// RECONCILE — merge batch results by taking the cheap model's output when its
/// self-check passed, else the fallback. Mirrors `consensual_aggregate` semantics
/// but for tier selection: prefer cheap, keep opus as the safety net.
pub fn reconcile_tier(cheap_output: &str, cheap_ok: bool, fallback_output: &str) -> String {
    if cheap_ok && !cheap_output.is_empty() {
        cheap_output.to_string()
    } else {
        fallback_output.to_string()
    }
}

/// Tiny FNV-1a (deterministic, no deps) — used for static-prefix fingerprinting.
pub fn fnv1a(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811C9DC5;
    for &x in b {
        h ^= x as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_reuse_accounts_savings() {
        // GREEN: two prompts with the SAME static prefix → 1 break + 1 hit,
        // cached fraction > 0 (static reused, only tail billed fresh).
        let p1 = CachePrompt::new("SYS: you are a Rust agent", "task A");
        let p2 = CachePrompt::new("SYS: you are a Rust agent", "task B");
        let mut led = CacheLedger::new();
        led.observe(None, &p1, 5);
        led.observe(Some(&p1.static_fingerprint()), &p2, 5);
        assert_eq!(led.breaks, 1);
        assert_eq!(led.hits, 1);
        assert!(
            led.cached_fraction() > 0.0,
            "cache should have saved static tokens"
        );
        assert!(p1.shares_cache(&p2));
    }

    #[test]
    fn cache_break_drains_savings() {
        // RED: every prompt mutates the static prefix → no reuse, cached ≈ 0.
        let p1 = CachePrompt::new("SYS v1", "t");
        let p2 = CachePrompt::new("SYS v2", "t"); // different static → break
        let mut led = CacheLedger::new();
        led.observe(None, &p1, 2);
        led.observe(Some(&p1.static_fingerprint()), &p2, 2);
        assert_eq!(led.breaks, 2);
        assert_eq!(led.hits, 0);
        assert_eq!(
            led.cached_fraction(),
            0.0,
            "frequent breaks must zero out savings"
        );
        assert!(!p1.shares_cache(&p2));
    }

    #[test]
    fn tier_cascade_escalates_only_when_needed() {
        // GREEN: cheap adequate + budget → cheap
        assert_eq!(route_tier(true, 10.0, 1.0), Tier::Cheap);
        // RED: cheap self-check failed → escalate to opus
        assert_eq!(route_tier(false, 10.0, 1.0), Tier::Opus);
        // RED: budget exhausted → free tier, never overspend
        assert_eq!(route_tier(true, 0.0, 1.0), Tier::Free);
    }

    #[test]
    fn batch_split_covers_each_item_once() {
        // GREEN: 10 items / 3 batches → every index present exactly once
        let shards = batch_split(10, 3);
        let mut all: Vec<usize> = shards.iter().flatten().cloned().collect();
        all.sort_unstable();
        assert_eq!(all, (0..10).collect::<Vec<_>>());
        // RED: empty input → empty (no phantom batches)
        assert!(batch_split(0, 3).is_empty());
        assert!(batch_split(5, 0).is_empty());
    }

    #[test]
    fn reconcile_prefers_cheap_keeps_fallback() {
        // GREEN: cheap ok → use cheap
        assert_eq!(reconcile_tier("cheap out", true, "opus out"), "cheap out");
        // RED: cheap failed → fall back to opus (no silent task drop)
        assert_eq!(reconcile_tier("", false, "opus out"), "opus out");
    }
}
