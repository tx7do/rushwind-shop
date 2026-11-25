//! The timing-side-channel guard: a dummy Argon2id verification run on
//! every "user not found" branch, so a missing row and a wrong password
//! cost the same.

/// Runs one throwaway Argon2id verification against a fixed dummy hash.
pub fn dummy_verify() {
    use rushwind_password::PasswordHasher as _;
    let dummy = rushwind_password_argon2::Argon2Hasher::new()
        .hash("timing-equalizer")
        .unwrap_or_default();
    let _ = rushwind_password_argon2::Argon2Hasher::new().verify("x", &dummy);
}
