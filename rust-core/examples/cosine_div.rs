// rust-core/examples/cosine_div.rs
// Self-asserting eqc proof for the cosine-similarity core: cos = dot / sqrt(na*nb).
// (The kernel's cosine_similarity wrappers this with clamp + degenerate->0; this proves the math.)
//   cargo run --example cosine_div
include!("../eqc-proofs/cosine_div.rs");
