use super::AuthService;
use crate::domain::auth::claims::Claims;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

impl AuthService {
    #[cfg(test)]
    pub fn generate_token(
        &self,
        user_id: Uuid,
        email: &str,
        role: &str,
        token_version: u64,
    ) -> anyhow::Result<String> {
        self.encode_claims(build_claims(
            user_id,
            email,
            role,
            self.session_ttl_seconds,
            token_version,
        ))
    }

    pub fn verify_token(&self, token: &str) -> anyhow::Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(Into::into)
    }

    pub fn token_ttl_seconds(&self, token: &str) -> anyhow::Result<u64> {
        let claims = self.verify_token(token)?;
        let now = Utc::now().timestamp();
        Ok((claims.exp - now).max(0) as u64)
    }

    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        role: &str,
        token_version: u64,
    ) -> anyhow::Result<String> {
        self.encode_claims(build_claims(
            user_id,
            email,
            role,
            Self::ACCESS_TOKEN_TTL_SECONDS,
            token_version,
        ))
    }

    pub fn generate_refresh_token(
        &self,
        user_id: Uuid,
        token_version: u64,
    ) -> anyhow::Result<String> {
        self.encode_claims(build_claims(
            user_id,
            "",
            "refresh",
            self.session_ttl_seconds,
            token_version,
        ))
    }

    #[cfg(test)]
    pub fn verify_refresh_token(&self, token: &str) -> anyhow::Result<Uuid> {
        Ok(self.verify_refresh_token_claims(token)?.sub)
    }

    pub fn verify_refresh_token_claims(&self, token: &str) -> anyhow::Result<Claims> {
        let claims = self.verify_token(token)?;
        if claims.role != "refresh" {
            return Err(anyhow::anyhow!("Not a refresh token"));
        }
        Ok(claims)
    }

    fn encode_claims(&self, claims: Claims) -> anyhow::Result<String> {
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(Into::into)
    }
}

fn build_claims(
    user_id: Uuid,
    email: &str,
    role: &str,
    ttl_seconds: u64,
    token_version: u64,
) -> Claims {
    let now = Utc::now();
    let ttl = Duration::seconds(ttl_seconds.min(i64::MAX as u64) as i64);
    let exp = now + ttl;

    Claims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        token_version,
        exp: exp.timestamp(),
        iat: now.timestamp(),
    }
}
