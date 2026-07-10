//! Ledger — deterministic double-entry money/resource boundary.
//!
//! Port of the bleeding-edge-EV "TigerBeetle" tier (tier-2: operational ledger
//! that backs the zkVM verify boundary). Grounded in TigerBeetle's hard invariant:
//! every transfer is double-entry, so the SUM OF ALL BALANCES IS ALWAYS ZERO.
//! That conservation law is the falsifiable proof surface — any positive or
//! negative drift means a transfer did not balance, and the ledger fails closed.
//!
//! The crate is the sovereign math core: NO rng, NO wall-clock. Transfers are
//! content-addressed by a deterministic id (`H(from||to||amount||nonce)`) so the
//! same transfer applied twice is a NO-OP (idempotency) — replay-safety for the
//! event-sourced kernel. Real TB would be a clustered datastore; this is the
//! invariant kernel that any substrate must satisfy. Honest: in-process only.
//!
//! Verified-by-Math: conservation holds on every green path; a tampered/imbalanced
//! transfer is rejected (RED). Idempotent: replaying a transfer does not change
//! the sum or any balance twice (RED).

use sha2::{Digest, Sha256};

/// A single account in the ledger.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub id: String,
    pub balance: i128, // signed; can be negative (debt) per policy
}

/// A transfer between two accounts.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    pub amount: i128, // MUST be > 0 (direction carried by from/to)
    pub nonce: u64,   // for idempotency / replay protection
}

/// The deterministic id of a transfer = H(from||to||amount||nonce), hex.
pub fn transfer_id(t: &Transfer) -> String {
    let mut h = Sha256::new();
    h.update(t.from.as_bytes());
    h.update(b"|");
    h.update(t.to.as_bytes());
    h.update(b"|");
    h.update(t.amount.to_le_bytes());
    h.update(b"|");
    h.update(t.nonce.to_le_bytes());
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone)]
pub struct Ledger {
    accounts: Vec<Account>,
    /// The set of transfer ids already applied (idempotency guard).
    applied: std::collections::HashSet<String>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, id: &str, initial: i128) {
        if self.accounts.iter().any(|a| a.id == id) {
            return; // opening an existing account is a no-op (idempotent)
        }
        self.accounts.push(Account {
            id: id.to_string(),
            balance: initial,
        });
    }

    pub fn balance(&self, id: &str) -> Option<i128> {
        self.accounts.iter().find(|a| a.id == id).map(|a| a.balance)
    }

    /// Conservation invariant: Σ balance == 0 across all accounts.
    /// This is the TigerBeetle law; if it ever drifts the ledger is corrupt.
    pub fn conserved(&self) -> bool {
        self.accounts.iter().map(|a| a.balance).sum::<i128>() == 0
    }

    /// Apply a transfer. Returns false (rejected) on any invariant violation:
    ///   - amount <= 0 (no zero/negative transfers)
    ///   - unknown account
    ///   - insufficient funds at `from`
    ///   - already-applied id (idempotency: replay == no-op, NOT a double-spend)
    /// A successful transfer keeps `conserved()` true.
    pub fn transfer(&mut self, t: &Transfer) -> bool {
        if t.amount <= 0 {
            return false;
        }
        let id = transfer_id(t);
        if self.applied.contains(&id) {
            return true; // idempotent: replay is a clean no-op, not a failure
        }
        let from_idx = match self.accounts.iter().position(|a| a.id == t.from) {
            Some(i) => i,
            None => return false,
        };
        let to_idx = match self.accounts.iter().position(|a| a.id == t.to) {
            Some(i) => i,
            None => return false,
        };
        if self.accounts[from_idx].balance < t.amount {
            return false; // insufficient funds — fail closed
        }
        // double-entry: debit one, credit the other, sum preserved by construction
        self.accounts[from_idx].balance -= t.amount;
        self.accounts[to_idx].balance += t.amount;
        self.applied.insert(id);
        self.conserved() // must hold; if not, this is a kernel bug
    }

    pub fn is_applied(&self, t: &Transfer) -> bool {
        self.applied.contains(&transfer_id(t))
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Ledger {
            accounts: Vec::new(),
            applied: std::collections::HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Ledger {
        let mut l = Ledger::new();
        // GENESIS: mint 1000 to "mint", which credits the system; conservation
        // is kept by giving "sink" the balancing −1000? No — in a closed ledger
        // the mint itself must be balanced. We model "mint" as the issuer that
        // starts at +1000 and seeds "alice" with −1000 (alice owes the issuer),
        // so Σ = 0. This is the canonical TB "issuance is also double entry".
        l.open("mint", 1000);
        l.open("alice", -1000);
        // sanity: genesis is conserved
        assert!(l.conserved(), "genesis must be conserved");
        l
    }

    #[test]
    fn green_transfer_preserves_conservation() {
        // GREEN: a valid transfer moves value but Σ stays 0.
        let mut l = setup();
        let ok = l.transfer(&Transfer {
            from: "mint".into(),
            to: "alice".into(),
            amount: 300,
            nonce: 1,
        });
        assert!(ok, "valid transfer rejected");
        assert_eq!(l.balance("mint"), Some(700));
        assert_eq!(l.balance("alice"), Some(-700));
        assert!(l.conserved(), "conservation broken after transfer");
    }

    #[test]
    fn red_insufficient_funds_rejected() {
        // RED: alice cannot send more than she holds.
        let mut l = setup();
        let ok = l.transfer(&Transfer {
            from: "alice".into(),
            to: "mint".into(),
            amount: 5000, // alice only has −1000 (can't pay out)
            nonce: 2,
        });
        assert!(!ok, "overdraft transfer wrongly accepted");
        assert_eq!(l.balance("alice"), Some(-1000), "balance mutated on reject");
        assert!(l.conserved());
    }

    #[test]
    fn red_zero_or_negative_amount_rejected() {
        // RED: amount must be strictly positive.
        let mut l = setup();
        assert!(!l.transfer(&Transfer {
            from: "mint".into(),
            to: "alice".into(),
            amount: 0,
            nonce: 3,
        }));
        assert!(!l.transfer(&Transfer {
            from: "mint".into(),
            to: "alice".into(),
            amount: -50,
            nonce: 4,
        }));
        assert!(l.conserved());
    }

    #[test]
    fn red_unknown_account_rejected() {
        // RED: transferring to/from a non-existent account fails closed.
        let mut l = setup();
        assert!(!l.transfer(&Transfer {
            from: "ghost".into(),
            to: "alice".into(),
            amount: 10,
            nonce: 5,
        }));
        assert!(!l.transfer(&Transfer {
            from: "mint".into(),
            to: "ghost".into(),
            amount: 10,
            nonce: 6,
        }));
        assert!(l.conserved());
    }

    #[test]
    fn idempotent_replay_is_noop() {
        // RED+GREEN: replaying a transfer id does not double-spend.
        let mut l = setup();
        let t = Transfer {
            from: "mint".into(),
            to: "alice".into(),
            amount: 200,
            nonce: 7,
        };
        assert!(l.transfer(&t));
        let after_first = l.balance("alice");
        // replay
        assert!(l.transfer(&t), "replay should be a clean no-op (true)");
        assert_eq!(
            l.balance("alice"),
            after_first,
            "balance changed on replay — double spend!"
        );
        assert!(l.is_applied(&t));
        assert!(l.conserved());
    }
}
