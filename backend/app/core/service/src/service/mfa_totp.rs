//! The TOTP support face for the MFA service — RFC 6238 over
//! HMAC-SHA1 (30 s period, 6 digits, ±1 window skew, the reference's
//! `pquerna/otp` defaults), the base32 codec the secrets ride, the
//! otpauth URL, and the QR PNG data URI the authenticator apps scan.
//! Hand-rolled: the whole face is ~100 lines and stays free of a
//! totp-crate's API surface.

use hmac::{Hmac, Mac};
use sha1::Sha1;

/// The otpauth issuer stamp (the reference's mfaTotpIssuer).
pub const TOTP_ISSUER: &str = "GoWindAdmin";
/// Allowed counter drift (±1 period) against small clock skew.
pub const TOTP_SKEW: u64 = 1;
/// The challenge TTL mirrors the reference's five minutes.
pub const MFA_CHALLENGE_TTL_SECS: i64 = 300;
/// The enroll cooldown (the reference's mfaEnrollCooldown).
pub const MFA_ENROLL_COOLDOWN_SECS: i64 = 30;
/// Per-challenge verify failures before the challenge is voided.
pub const MAX_LOGIN_CHALLENGE_FAILURES: i64 = 3;

// ---------- base32 (RFC 4648, no padding — the otp secret spelling) ----------

const B32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn base32_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    for chunk in data.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let bits = (u64::from(buf[0]) << 32)
            | (u64::from(buf[1]) << 24)
            | (u64::from(buf[2]) << 16)
            | (u64::from(buf[3]) << 8)
            | u64::from(buf[4]);
        // A partial chunk only emits its significant quintets.
        let significant = match chunk.len() {
            5 => 8,
            4 => 7,
            3 => 5,
            2 => 4,
            _ => 2,
        };
        for i in 0..significant {
            out.push(B32_ALPHABET[(bits >> (35 - i * 5) & 0x1f) as usize] as char);
        }
    }
    out
}

pub fn base32_decode(text: &str) -> Option<Vec<u8>> {
    let mut acc: u64 = 0;
    let mut bits: u32 = 0;
    let mut out = Vec::with_capacity(text.len() * 5 / 8);
    for c in text.chars() {
        if c == '=' || c.is_whitespace() {
            continue;
        }
        let v = B32_ALPHABET
            .iter()
            .position(|&b| b == c.to_ascii_uppercase() as u8)? as u64;
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

// ---------- the RFC 4226 HOTP truncation + RFC 6238 clock ----------

fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("hmac accepts any key length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let code = u32::from_be_bytes([
        digest[offset] & 0x7f,
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    (code % 1_000_000) as u32
}

fn counter_at(now_secs: i64) -> u64 {
    (now_secs.max(0) as u64) / 30
}

/// The six-digit code the authenticator shows at `now_secs`.
pub fn totp_at(secret: &[u8], now_secs: i64) -> String {
    format!("{:06}", hotp(secret, counter_at(now_secs)))
}

/// The ±skew window check (the reference's ValidateCustom defaults).
pub fn totp_verify(code: &str, secret: &[u8], now_secs: i64) -> bool {
    let submitted: u32 = match code.trim().parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let counter = counter_at(now_secs);
    (counter.saturating_sub(TOTP_SKEW)..=counter + TOTP_SKEW).any(|c| hotp(secret, c) == submitted)
}

// ---------- the enroll key: 20 random bytes, the otpauth URL, the QR ----------

/// A fresh base32 secret (160 bits, the pquerna/otp default size).
pub fn generate_totp_secret() -> String {
    use uuid::Uuid;
    // Two UUIDv4s (26 random bytes after the fixed nibbles) truncated to
    // 20 — a random-enough draw without pulling another rand backend.
    let mut bytes = [0u8; 20];
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut raw = [0u8; 32];
    raw[..16].copy_from_slice(a.as_bytes());
    raw[16..].copy_from_slice(b.as_bytes());
    bytes.copy_from_slice(&raw[..20]);
    base32_encode(&bytes)
}

/// The otpauth:// URI the authenticator apps import (percent-escaped
/// label per the key URI format).
pub fn otpauth_url(secret: &str, user_id: i64) -> String {
    format!(
        "otpauth://totp/{issuer}:uid%3A{uid}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30",
        issuer = TOTP_ISSUER,
        uid = user_id,
        secret = secret,
    )
}

/// The QR PNG rendered from the otpauth URI as a data URI (240 px, the
/// reference's key.Image(240, 240)). Gray-scale 8-bit, 4 px per module
/// plus a 4-module quiet zone.
pub fn qr_png_data_uri(text: &str) -> Option<String> {
    use base64::Engine as _;
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let colors = code.to_colors();
    let width = code.width();
    let scale = 4usize;
    let quiet = 4usize;
    let dim = (width + quiet * 2) * scale;
    let mut px = vec![255u8; dim * dim];
    for (i, c) in colors.iter().enumerate() {
        if *c == qrcode::Color::Dark {
            let x = (i % width) as usize + quiet;
            let y = i / width as usize + quiet;
            for dy in 0..scale {
                for dx in 0..scale {
                    px[(y * scale + dy) * dim + x * scale + dx] = 0;
                }
            }
        }
    }
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, dim as u32, dim as u32);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&px).ok()?;
    }
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png_bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238's SHA-1 test vector set ("12345678901234567890" seed),
    /// quoted at the 30 s period.
    #[test]
    fn rfc6238_reference_vectors() {
        let secret = b"12345678901234567890";
        for (t, code) in [
            (59_i64, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ] {
            assert_eq!(totp_at(secret, t), code, "t={t}");
            assert!(totp_verify(code, secret, t), "verify t={t}");
        }
    }

    #[test]
    fn the_skew_window_accepts_neighbors_and_rejects_far_codes() {
        let secret = b"12345678901234567890";
        let t = 1_111_111_111;
        // ±1 period lands inside the window: the neighbors' codes verify.
        assert!(totp_verify(&totp_at(secret, t - 30), secret, t));
        assert!(totp_verify(&totp_at(secret, t + 30), secret, t));
        // Two periods out: rejected.
        assert!(!totp_verify(&totp_at(secret, t + 90), secret, t));
        // Garbage never verifies.
        assert!(!totp_verify("abcdef", secret, t));
        assert!(!totp_verify("", secret, t));
    }

    #[test]
    fn base32_round_trips_and_spells_the_otp_shape() {
        for len in 0..=25usize {
            let data: Vec<u8> = (0..len as u8).collect();
            let text = base32_encode(&data);
            assert!(text
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
            assert_eq!(
                base32_decode(&text).as_deref(),
                Some(data.as_slice()),
                "{text}"
            );
        }
        // The RFC 4648 vectors, unpadded — the otp secret spelling.
        assert_eq!(base32_encode(b"f"), "MY");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
        // The decoder tolerates padding and lower case.
        assert_eq!(
            base32_decode("mzxw6ytboi======").as_deref(),
            Some(b"foobar".as_slice())
        );
    }

    #[test]
    fn the_generated_secret_round_trips_through_the_verifier() {
        let secret_text = generate_totp_secret();
        assert_eq!(secret_text.len(), 32, "160 bits → 32 base32 chars");
        let secret = base32_decode(&secret_text).unwrap();
        let now = chrono::Utc::now().timestamp();
        let code = totp_at(&secret, now);
        assert_eq!(code.len(), 6);
        assert!(totp_verify(&code, &secret, now));
    }

    #[test]
    fn the_url_and_qr_carry_the_scannable_shapes() {
        let url = otpauth_url("JBSWY3DPEBPDE", 42);
        assert!(
            url.starts_with("otpauth://totp/GoWindAdmin:uid%3A42?secret="),
            "{url}"
        );
        assert!(url.contains("&issuer=GoWindAdmin&algorithm=SHA1&digits=6&period=30"));
        let uri = qr_png_data_uri(&url).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"), "{uri:?}");
        // The payload decodes as a real PNG (8-byte signature) — the
        // renderer produced bytes, not a stub.
        let b64 = uri.trim_start_matches("data:image/png;base64,");
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }
}
