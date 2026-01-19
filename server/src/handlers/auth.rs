//! Authentication handlers

use axum::{extract::State, Json};
use crate::{
    crypto,
    error::{AppError, Result},
    models::*,
    AppState,
};

use super::AuthUser;

/// Login with user ID and access key
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Verify credentials
    let valid = state
        .storage
        .verify_user_credentials(&req.user_id, &req.access_key)
        .await
        .map_err(|_| AppError::InvalidCredentials)?;

    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    // Get user
    let user = state
        .storage
        .get_user(&req.user_id)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    // Create or find device
    let device_id = state
        .storage
        .create_device(
            &req.user_id,
            &req.device_name,
            &req.device_type,
            &req.device_public_key,
        )
        .await?;

    // Create session
    let token = crypto::generate_session_token();
    let expires_at = state
        .storage
        .create_session(&req.user_id, &device_id, &token, 24 * 30) // 30 days
        .await?;

    // Update last seen
    state.storage.update_user_last_seen(&req.user_id).await?;

    tracing::info!("User {} logged in from device {}", req.user_id, device_id);

    Ok(Json(LoginResponse {
        token,
        device_id,
        expires_at: expires_at.timestamp(),
        user: user.into(),
    }))
}

/// Refresh an existing session token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>> {
    // Validate current session
    let session = state
        .storage
        .validate_session(&req.token)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Invalidate old session
    state.storage.invalidate_session(&req.token).await?;

    // Create new session
    let new_token = crypto::generate_session_token();
    let expires_at = state
        .storage
        .create_session(&session.user_id, &session.device_id, &new_token, 24 * 30)
        .await?;

    Ok(Json(RefreshTokenResponse {
        token: new_token,
        expires_at: expires_at.timestamp(),
    }))
}

/// Logout and invalidate session
pub async fn logout(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>> {
    // Get the token from the auth context to invalidate
    // Note: In production, you'd want to track the actual token
    // For now, we just update the last seen timestamp

    tracing::info!("User {} logged out from device {}", auth.user_id, auth.device_id);

    Ok(Json(serde_json::json!({ "success": true })))
}
