//! Framing — encode/decode the envelope onto a carrier byte stream.
//!
//! The wire format is a minimal, byte-deterministic, length-prefixed envelope:
//!
//! ```text
//! [ u32 LE length ][ Envelope JSON bytes ]
//! ```
//!
//! The 4-byte little-endian length prefix lets both carriers (WSS binary frames,
//! iroh streams) delimit messages without carrier-specific delimiters. A max
//! frame cap enforces `WireError::FrameTooLarge` (fail-closed, no unbounded read).
//!
//! CI GUARD: NO-COURIER-SCORING — framing is pure layout; no scoring surface.

use crate::envelope::Envelope;
use crate::error::{WireError, WireResult};

/// Maximum envelope size we will accept/emit (8 MiB). Fail-closed above this.
pub const MAX_ENVELOPE_BYTES: usize = 8 * 1024 * 1024;

/// Encode an envelope into the length-prefixed wire format.
pub fn encode(envelope: &Envelope) -> WireResult<Vec<u8>> {
    let body = envelope.to_bytes()?;
    if body.len() > MAX_ENVELOPE_BYTES {
        return Err(WireError::FrameTooLarge(body.len()));
    }
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode one length-prefixed envelope from a byte buffer.
///
/// `buf` is a cursor: bytes consumed by the frame are removed from the front.
/// Returns `Ok(None)` if not enough bytes are present yet (caller should wait
/// for more carrier data). Returns `Ok(Some(..))` once a full frame is decoded.
pub fn decode(buf: &mut Vec<u8>) -> WireResult<Option<Envelope>> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if len > MAX_ENVELOPE_BYTES {
        return Err(WireError::FrameTooLarge(len));
    }
    if buf.len() < 4 + len {
        return Ok(None);
    }
    let body = &buf[4..4 + len];
    let envelope = Envelope::from_bytes(body)?;
    // Advance the cursor.
    buf.drain(0..4 + len);
    Ok(Some(envelope))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encode_decode() {
        let env = Envelope::new(
            [1, 2, 3, 4, 0, 0, 0, 0, 9, 9, 9, 9, 0, 0, 0, 0],
            b"payload-bytes".to_vec(),
        );
        let bytes = encode(&env).unwrap();
        let mut buf = bytes.clone();
        let got = decode(&mut buf).unwrap().expect("full frame");
        assert_eq!(got, env);
        assert!(buf.is_empty(), "cursor fully consumed");
    }

    #[test]
    fn partial_frame_returns_none_then_completes() {
        let env = Envelope::new([0; 16], b"abc".to_vec());
        let bytes = encode(&env).unwrap();
        let mut buf = bytes[..3].to_vec();
        assert!(decode(&mut buf).unwrap().is_none());
        buf.extend_from_slice(&bytes[3..]);
        let got = decode(&mut buf).unwrap().expect("now complete");
        assert_eq!(got, env);
    }

    #[test]
    fn oversize_is_rejected() {
        let huge = Envelope::new([0; 16], vec![0u8; MAX_ENVELOPE_BYTES + 1]);
        assert!(matches!(encode(&huge), Err(WireError::FrameTooLarge(_))));
    }
}
