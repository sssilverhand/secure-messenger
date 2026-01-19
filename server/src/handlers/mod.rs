//! HTTP request handlers for PrivMsg Server

pub mod admin;
pub mod auth;
pub mod files;
pub mod health;
pub mod messages;
pub mod turn;
pub mod users;
pub mod websocket;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use crate::{error::AppError, AppState};

/// Authenticated user context extracted from request
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub device_id: String,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        // Parse "Bearer <token>"
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        // Validate session
        let session = state
            .storage
            .validate_session(token)
            .await
            .map_err(|_| AppError::Unauthorized)?
            .ok_or(AppError::Unauthorized)?;

        // Update device activity
        let _ = state.storage.update_device_activity(&session.device_id).await;
        let _ = state.storage.update_user_last_seen(&session.user_id).await;

        Ok(AuthUser {
            user_id: session.user_id,
            device_id: session.device_id,
        })
    }
}
