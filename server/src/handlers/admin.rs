//! Admin handlers

use axum::{
    extract::{Path, State},
    Json,
};
use crate::{
    crypto,
    error::{AppError, Result},
    models::*,
    AppState,
};

/// Admin authentication middleware check
fn verify_admin_key(provided: &str, expected: &str) -> Result<()> {
    if provided != expected {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Create a new user (admin only)
pub async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>> {
    verify_admin_key(&req.admin_key, &state.config.admin.master_key)?;

    let user_id = req.user_id.unwrap_or_else(|| crypto::generate_user_id());
    let access_key = crypto::generate_access_key();
    let key_hash = crypto::hash_access_key(&access_key);

    // Check if user already exists
    if state.storage.get_user(&user_id).await?.is_some() {
        return Err(AppError::UserAlreadyExists);
    }

    state.storage.create_user(&user_id, &key_hash).await?;

    tracing::info!("Admin created user: {}", user_id);

    Ok(Json(CreateUserResponse {
        user_id,
        access_key,
    }))
}

/// Delete a user (admin only)
pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Json(req): Json<AdminKeyRequest>,
) -> Result<Json<serde_json::Value>> {
    verify_admin_key(&req.admin_key, &state.config.admin.master_key)?;

    // Disconnect user if online
    state.ws_manager.send_to_user(
        &user_id,
        WsServerMessage::Error {
            code: "ACCOUNT_DELETED".to_string(),
            message: "Your account has been deleted".to_string(),
        },
    );

    // Delete user and cascade
    state.storage.delete_user(&user_id).await?;

    tracing::info!("Admin deleted user: {}", user_id);

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Get server statistics (admin only)
pub async fn get_stats(
    State(state): State<AppState>,
    Json(req): Json<AdminKeyRequest>,
) -> Result<Json<ServerStats>> {
    verify_admin_key(&req.admin_key, &state.config.admin.master_key)?;

    let mut stats = state.storage.get_stats().await?;
    stats.online_users = state.ws_manager.online_user_count() as i64;

    Ok(Json(stats))
}

#[derive(Debug, serde::Deserialize)]
pub struct AdminKeyRequest {
    pub admin_key: String,
}
