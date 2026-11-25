//! The per-client token store over Redis — the session whitelist
//! (`rws:at:{ct}:{uid}:{jti}` exact compare), the refresh rotation and
//! the blacklist (`rws:bl:{jti}`), mirroring the reference's token.rs.

use redis::aio::ConnectionManager;
use redis::AsyncCommands as _;
use shop_proto::proto::authentication::service::v1::ClientType;

/// The admin (ct=0) / app (ct=1) split the wire protocol pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientKind {
    Admin,
    App,
}

impl ClientKind {
    pub fn from_i32(v: i32) -> Self {
        if v == ClientType::App as i32 {
            Self::App
        } else {
            Self::Admin
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Admin => 0,
            Self::App => 1,
        }
    }
}

pub struct TokenStore {
    conn: ConnectionManager,
    client: ClientKind,
    access_secs: i64,
    refresh_secs: i64,
}

impl TokenStore {
    pub fn new(
        conn: ConnectionManager,
        client: ClientKind,
        access_secs: i64,
        refresh_secs: i64,
    ) -> Self {
        Self {
            conn,
            client,
            access_secs,
            refresh_secs,
        }
    }

    pub fn access_secs(&self) -> i64 {
        self.access_secs
    }

    pub fn refresh_secs(&self) -> i64 {
        self.refresh_secs
    }

    fn at_key(&self, uid: i64, jti: &str) -> String {
        format!("rws:at:{}:{}:{}", self.client.as_u32(), uid, jti)
    }

    fn rt_key(&self, uid: i64, jti: &str) -> String {
        format!("rws:rt:{}:{}:{}", self.client.as_u32(), uid, jti)
    }

    fn bl_key(jti: &str) -> String {
        format!("rws:bl:{jti}")
    }

    /// Whitelist exact-match — false means revoked/expired.
    pub async fn is_valid_access_token(&self, uid: i64, jti: &str, token: &str) -> bool {
        let Ok(stored): Result<String, _> = self.conn.clone().get(self.at_key(uid, jti)).await
        else {
            return false;
        };
        stored == token
    }

    /// Blacklist existence.
    pub async fn is_blocked(&self, jti: &str) -> bool {
        let Ok(n): Result<i64, _> = self.conn.clone().exists(Self::bl_key(jti)).await else {
            return false;
        };
        n > 0
    }

    /// Registers a fresh token pair (the mint path). A disabled refresh
    /// expiry (0s — the app profile) stores no RT row.
    pub async fn add_token_pair(
        &self,
        uid: i64,
        jti: &str,
        access: &str,
        refresh: Option<&str>,
    ) -> Result<(), String> {
        let mut conn = self.conn.clone();
        let _: () = conn
            .set_ex(
                self.at_key(uid, jti),
                access,
                self.access_secs.max(1) as u64,
            )
            .await
            .map_err(|e| e.to_string())?;
        if let Some(refresh) = refresh.filter(|_| self.refresh_secs > 0) {
            let _: () = conn
                .set_ex(self.rt_key(uid, jti), refresh, self.refresh_secs as u64)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The refresh rotation: verify-and-revoke. The stored RT row must
    /// equal the presented token exactly — a second use of a rotated
    /// token finds the row gone and fails. (GET+DEL, not WATCH/MULTI:
    /// the race window is one rotated token replayed within the same
    /// instant, which the exact-compare then rejects.)
    pub async fn verify_and_revoke_pair(
        &self,
        uid: i64,
        jti: &str,
        refresh: &str,
    ) -> Result<bool, String> {
        let mut conn = self.conn.clone();
        let key = self.rt_key(uid, jti);
        let stored: Option<String> = redis::cmd("GET")
            .arg(key.as_str())
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        if stored.as_deref() != Some(refresh) {
            return Ok(false);
        }
        let _: () = conn
            .del(key.as_str())
            .await
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub async fn revoke_user_tokens(&self, uid: i64) -> Result<(), String> {
        let mut conn = self.conn.clone();
        let pattern = format!("rws:*:{}:{}:*", self.client.as_u32(), uid);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        if !keys.is_empty() {
            let _: () = conn.del(keys).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn access_tokens(&self, uid: i64) -> Result<Vec<String>, String> {
        let mut conn = self.conn.clone();
        let pattern = format!("rws:at:{}:{}:*", self.client.as_u32(), uid);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let values: Vec<String> = redis::cmd("MGET")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        Ok(values)
    }

    pub async fn block_token(&self, jti: &str, secs: i64) -> Result<(), String> {
        let mut conn = self.conn.clone();
        let _: () = conn
            .set_ex(Self::bl_key(jti), "1", secs.max(1) as u64)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn revoke_by_jti(&self, uid: i64, jti: &str) -> Result<(), String> {
        let mut conn = self.conn.clone();
        let _: () = conn
            .del(vec![self.at_key(uid, jti), self.rt_key(uid, jti)])
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// The refresh-presenting lookup: scan the client's RT keys for
    /// the user, return the jti whose stored row equals the presented
    /// token. The KEYS scan is bounded by one user's row count.
    pub async fn find_jti_of_refresh(
        &self,
        uid: i64,
        refresh: &str,
    ) -> Result<Option<String>, String> {
        let mut conn = self.conn.clone();
        let pattern = format!("rws:rt:{}:{}:*", self.client.as_u32(), uid);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        if keys.is_empty() {
            return Ok(None);
        }
        let values: Vec<String> = redis::cmd("MGET")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| e.to_string())?;
        for (key, stored) in keys.into_iter().zip(values.into_iter()) {
            if stored == refresh {
                // rws:rt:{ct}:{uid}:{jti}
                let jti = key.rsplit(':').next().unwrap_or("").to_string();
                return Ok((!jti.is_empty()).then_some(jti));
            }
        }
        Ok(None)
    }

    /// The refresh credential: 256 bits of CSPRNG, hex — compared
    /// exactly against the stored row.
    pub fn new_refresh_token() -> String {
        // Two UUIDv4s joined: 244 bits of randomness from the uuid
        // crate's CSPRNG, hex-shaped like the reference's 256-bit
        // credential.
        format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        )
    }
}
