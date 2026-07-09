//! Knowledge — the §0·GP living-knowledge retriever (ported from `src/knowledge.ts`).
//! Deterministic sparse retrieval: cosine over hashed concept vectors.
//! No RNG. Returns REAL payloads; a noise floor is excluded honestly.

use crate::memory::{LivingMemory, MemoryNode};

pub struct Hit {
    pub id: String,
    pub concept: String,
    pub text: String,
    pub score: f64,
}

/// Retrieve the top-k nodes nearest `query` by hashed-bag-of-bytes cosine.
/// `note` explains the result (incl. an honest noise floor).
pub fn recall(mm: &LivingMemory, query: &str, k: usize) -> RecallOut {
    let qv = bag_vec(query.as_bytes());
    let mut scored: Vec<(f64, &MemoryNode)> = mm
        .nodes()
        .values()
        .map(|n| (cosine(&qv, &bag_vec(n.concept.as_bytes())), n))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    // Noise floor: below this cosine the match is indistinguishable from chance
    // for short strings, so we exclude it honestly (no manufactured hits).
    const NOISE_FLOOR: f64 = 0.35;
    let mut hits = Vec::new();
    for (s, n) in scored.into_iter().take(k) {
        if s < NOISE_FLOOR {
            continue;
        }
        hits.push(Hit {
            id: n.id.clone(),
            concept: n.concept.clone(),
            text: n.payload.clone(),
            score: s,
        });
    }
    let note = if hits.is_empty() {
        format!("no real hit above noise floor ({NOISE_FLOOR})")
    } else {
        "retrieved real payloads".into()
    };
    RecallOut { hits, note }
}

pub struct RecallOut {
    pub hits: Vec<Hit>,
    pub note: String,
}

/// Bag-of-bytes vector: counts of each byte value (256-dim, deterministic).
pub fn bag_vec(b: &[u8]) -> Vec<f64> {
    let mut v = vec![0f64; 256];
    for &x in b {
        v[x as usize] += 1.0;
    }
    v
}

pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::LivingMemory;

    #[test]
    fn recall_returns_real_payload() {
        // GREEN: a stored concept is retrievable, with its concept + payload.
        let mut m = LivingMemory::new();
        m.remember("copilot", "native doer/checker seam");
        m.remember("vault", "encrypted-at-rest identity");
        let r = recall(&m, "copilot", 2);
        assert!(!r.hits.is_empty(), "no hit for a stored concept");
        assert_eq!(r.hits[0].concept, "copilot");
        assert!(r.hits[0].text.contains("doer/checker"));
    }

    #[test]
    fn recall_excludes_noise_floor() {
        // RED: a query with ZERO letter overlap must NOT manufacture a fake hit.
        let mut m = LivingMemory::new();
        m.remember("copilot", "native doer/checker seam");
        m.remember("vault", "encrypted-at-rest identity");
        // "qzxjwk" shares no letters with either stored concept → cosine 0.
        let r = recall(&m, "qzxjwk", 2);
        assert!(r.hits.is_empty(), "noise floor leaked a fake hit");
        assert!(r.note.contains("noise"));
    }
}
