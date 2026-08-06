//! Platform JWT gate over the venue routes.
//!
//! Claims are `sub`/`exp`/`role` signed HS256 with the shared platform secret
//! in `PLATFORM_JWT_SECRET`, the shape tiletopia mints and geodukt, collecta
//! and ptolemy validate, so one platform token works here too.
//!
//! The secret is mandatory: there is no unauthenticated mode, matching
//! collecta. `/health` is the only open route.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, http::HeaderMap};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

/// Env var holding the shared HS256 secret.
pub const SECRET_ENV: &str = "PLATFORM_JWT_SECRET";

/// Shortest HS256 secret we accept, matching tiletopia, collecta and ptolemy.
pub const MIN_SECRET_LEN: usize = 32;

/// JWT claims. `exp` is required and checked by [`Validation`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    /// Role name. Read it through [`Role::parse`], never by comparing the
    /// string, so an unknown role is refused instead of landing in a tier.
    pub role: String,
}

/// What a caller may do. Copied from the `role` claim.
///
/// A string that is not one of these does not parse, so a typo or a role from
/// some future service grants nothing rather than defaulting to read access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "admin" => Some(Role::Admin),
            "editor" => Some(Role::Editor),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    /// May upload or delete a venue. Viewers are read-only.
    pub fn can_write(self) -> bool {
        matches!(self, Role::Admin | Role::Editor)
    }
}

/// The verified caller.
///
/// Handlers and [`require_write`] read this from request extensions. It is
/// inserted only after the signature, `exp` and `role` all checked out, so
/// holding one is proof of an authenticated caller with a role we know.
#[derive(Debug, Clone, Copy)]
pub struct Caller {
    pub role: Role,
}

/// The signing secret the gate validates against.
#[derive(Clone)]
pub struct AuthConfig {
    secret: Arc<str>,
}

/// Redacted so a stray `{:?}` cannot put the secret in a log line.
impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig").finish_non_exhaustive()
    }
}

impl AuthConfig {
    /// Reject a missing or short secret: HS256 with a short secret is
    /// brute-forceable, and this is the only way to build the gate.
    pub fn new(secret: &str) -> Result<Self, String> {
        if secret.is_empty() {
            return Err(format!(
                "{SECRET_ENV} is not set. Set it to 32+ random bytes shared with the other \
                 platform services."
            ));
        }
        if secret.len() < MIN_SECRET_LEN {
            // the length, never the secret
            return Err(format!(
                "{SECRET_ENV} is {} bytes, need at least {MIN_SECRET_LEN}",
                secret.len()
            ));
        }
        Ok(Self {
            secret: Arc::from(secret),
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Self::new(&std::env::var(SECRET_ENV).unwrap_or_default())
    }
}

fn deny(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|t| !t.is_empty())
}

/// Require a valid platform token carrying a role we know, and expose the
/// [`Caller`] to the handlers and to [`require_write`].
///
/// `Validation::default()` pins HS256 and requires `exp`, so a token signed
/// with another algorithm, an `alg: none` header, an expired token and a
/// wrong-secret signature all fail here.
pub async fn require_auth(
    State(config): State<AuthConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(request.headers()) else {
        return deny(StatusCode::UNAUTHORIZED, "missing bearer token");
    };

    // the decode error is not echoed back: it separates "expired" from "bad
    // signature", which helps an attacker more than a caller
    let key = DecodingKey::from_secret(config.secret.as_bytes());
    let Ok(data) = decode::<Claims>(token, &key, &Validation::default()) else {
        return deny(StatusCode::UNAUTHORIZED, "invalid or expired token");
    };

    let Some(role) = Role::parse(&data.claims.role) else {
        return deny(StatusCode::FORBIDDEN, "unknown role");
    };

    request.extensions_mut().insert(Caller { role });
    next.run(request).await
}

/// Narrow a route to editors and admins. Must sit inside [`require_auth`],
/// which is what puts the [`Caller`] in the extensions.
pub async fn require_write(
    Extension(caller): Extension<Caller>,
    request: Request,
    next: Next,
) -> Response {
    if !caller.role.can_write() {
        return deny(StatusCode::FORBIDDEN, "editor or admin role required");
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_parse_is_exact() {
        assert_eq!(Role::parse("admin"), Some(Role::Admin));
        assert_eq!(Role::parse("editor"), Some(Role::Editor));
        assert_eq!(Role::parse("viewer"), Some(Role::Viewer));
        for role in ["", "Admin", "ADMIN", " admin", "admin ", "root", "owner"] {
            assert_eq!(Role::parse(role), None, "{role:?}");
        }
    }

    #[test]
    fn only_editor_and_admin_write() {
        assert!(Role::Admin.can_write());
        assert!(Role::Editor.can_write());
        assert!(!Role::Viewer.can_write());
    }

    #[test]
    fn weak_secrets_are_refused() {
        assert!(AuthConfig::new("").unwrap_err().contains("is not set"));
        let err = AuthConfig::new("short-secret").unwrap_err();
        assert!(err.contains("need at least 32"));
        assert!(!err.contains("short-secret"));
        assert!(AuthConfig::new("0123456789abcdef0123456789abcdef").is_ok());
    }

    #[test]
    fn debug_does_not_print_the_secret() {
        let config = AuthConfig::new("0123456789abcdef-s3cr3t-0123456789").unwrap();
        assert!(!format!("{config:?}").contains("s3cr3t"));
    }
}
