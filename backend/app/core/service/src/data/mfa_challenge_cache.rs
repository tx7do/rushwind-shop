//! The MFA challenge cache — the Redis-backed operation contexts (the
//! reference's mfa_challenge_cache.go): the enroll secret ride, the
//! login challenge (the token payload awaiting the second factor), the
//! per-challenge failure counter, the enroll cooldown, and the atomic
//! take that adjudicates the concurrent verify double-spend.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::service::mfa_totp::{
    MAX_LOGIN_CHALLENGE_FAILURES, MFA_CHALLENGE_TTL_SECS, MFA_ENROLL_COOLDOWN_SECS,
};

/// The login challenge context: the token payload (the proto JSON, the
/// wire's portable spelling) plus the client type that issued it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginChallenge {
    /// The protojson envelope of `UserTokenPayload` (uid/tid/username/...).
    pub payload: serde_json::Value,
    pub client_type: i32,
}

/// The enroll challenge context: the freshly minted secret awaiting the
/// first code's confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollChallenge {
    pub secret: String,
    pub tenant_id: i64,
    pub user_id: i64,
}

/// An unpredictable operation id (the same shape class as the refresh
/// token's random draw).
fn new_operation_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn login_key(op_id: &str) -> String {
    format!("mfa:login:{op_id}")
}

fn fail_key(op_id: &str) -> String {
    format!("mfa:loginfail:{op_id}")
}

fn enroll_key(op_id: &str) -> String {
    format!("mfa:enroll:{op_id}")
}

fn enroll_cooldown_key(tenant_id: i64, user_id: i64) -> String {
    format!("mfa:enrollcd:{tenant_id}:{user_id}")
}

pub struct MfaChallengeCache {
    pub redis: ConnectionManager,
}

impl MfaChallengeCache {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Writes the login challenge (the password face calls this when the
    /// user rides a TOTP factor) and returns the operation id.
    pub async fn set_login_challenge(
        &self,
        payload: serde_json::Value,
        client_type: i32,
    ) -> Result<String, String> {
        let envelope = serde_json::to_vec(&LoginChallenge {
            payload,
            client_type,
        })
        .map_err(|e| e.to_string())?;
        let op_id = new_operation_id();
        let _: () = self
            .redis
            .clone()
            .set_ex(login_key(&op_id), envelope, MFA_CHALLENGE_TTL_SECS as u64)
            .await
            .map_err(|e| e.to_string())?;
        Ok(op_id)
    }

    /// The read-only peek (the verify face retries on wrong codes).
    pub async fn peek_login_challenge(&self, op_id: &str) -> Option<LoginChallenge> {
        let raw: Option<Vec<u8>> = self.redis.clone().get(login_key(op_id)).await.ok()?;
        raw.and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    /// Records one verify failure; `true` once the cap voided the
    /// challenge (the fail-closed answer for a Redis error too).
    pub async fn record_login_failure(&self, op_id: &str) -> bool {
        let mut con = self.redis.clone();
        let n: Result<i64, _> = redis::cmd("INCR")
            .arg(fail_key(op_id))
            .query_async(&mut con)
            .await;
        let n = match n {
            Ok(n) => n,
            Err(_) => return true,
        };
        let _: Result<(), _> = con
            .expire(fail_key(op_id), MFA_CHALLENGE_TTL_SECS as i64)
            .await;
        if n >= MAX_LOGIN_CHALLENGE_FAILURES {
            let _: Result<(), _> = con.del(login_key(op_id)).await;
            return true;
        }
        false
    }

    /// Consumes the challenge without adjudication (factor-missing and
    /// cap-reached paths).
    pub async fn consume_login_challenge(&self, op_id: &str) {
        let mut con = self.redis.clone();
        let _: Result<i64, _> = con.del(login_key(op_id)).await;
        let _: Result<i64, _> = con.del(fail_key(op_id)).await;
    }

    /// The atomic GET+DEL — only the caller that gets the value back
    /// holds the adjudication right (the concurrent double-spend fence).
    pub async fn take_login_challenge_atomic(&self, op_id: &str) -> bool {
        let mut con = self.redis.clone();
        let script = redis::Script::new(
            "local stored = redis.call('GET', KEYS[1]) \
             if not stored then return false end \
             redis.call('DEL', KEYS[1]) return stored",
        );
        let taken: Result<Option<Vec<u8>>, _> =
            script.key(login_key(op_id)).invoke_async(&mut con).await;
        match taken {
            Ok(Some(_)) => {
                let _: Result<(), _> = con.del(fail_key(op_id)).await;
                true
            }
            _ => false,
        }
    }

    /// The enroll anti-flood: one Start per cooldown per user.
    pub async fn try_acquire_enroll_cooldown(&self, tenant_id: i64, user_id: i64) -> bool {
        let mut con = self.redis.clone();
        let ok: Result<bool, _> = redis::cmd("SET")
            .arg(enroll_cooldown_key(tenant_id, user_id))
            .arg(1)
            .arg("NX")
            .arg("EX")
            .arg(MFA_ENROLL_COOLDOWN_SECS)
            .query_async(&mut con)
            .await;
        ok.unwrap_or(true)
    }

    /// Writes the enroll context (the minted secret) and returns the
    /// operation id.
    pub async fn set_enroll_challenge(
        &self,
        secret: &str,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<String, String> {
        let envelope = serde_json::to_vec(&EnrollChallenge {
            secret: secret.to_string(),
            tenant_id,
            user_id,
        })
        .map_err(|e| e.to_string())?;
        let op_id = new_operation_id();
        let _: () = self
            .redis
            .clone()
            .set_ex(enroll_key(&op_id), envelope, MFA_CHALLENGE_TTL_SECS as u64)
            .await
            .map_err(|e| e.to_string())?;
        Ok(op_id)
    }

    /// The read-only enroll peek (the first code may retry).
    pub async fn peek_enroll_challenge(&self, op_id: &str) -> Option<EnrollChallenge> {
        let raw: Option<Vec<u8>> = self.redis.clone().get(enroll_key(op_id)).await.ok()?;
        raw.and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    /// Consumes the enroll context (the factor row landed).
    pub async fn delete_enroll_challenge(&self, op_id: &str) {
        let mut con = self.redis.clone();
        let _: Result<(), _> = con.del(enroll_key(op_id)).await;
    }
}
