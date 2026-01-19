//! User management handlers

use axum::{
    extract::{Path, State},
    Json,
};
use crate::{
    error::{AppError, Result},
    models::*,
    AppState,
};

use super::AuthUser;

/// Get current user's profile
pub async fn get_current_user(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserProfile>> {
    let user = state
        .storage
        .get_user(&auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

/// Get another user's profile by ID
pub async fn get_user(
    State(state): State<AppState>,
    _auth: AuthUser, // Must be authenticated
    Path(user_id): Path<String>,
) -> Result<Json<UserProfile>> {
    let user = state
        .storage
        .get_user(&user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    if !user.is_active {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    Ok(Json(user.into()))
}

/// Update current user's profile
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>> {
    state
        .storage
        .update_user_profile(
            &auth.user_id,
            req.display_name.as_deref(),
            req.avatar_file_id.as_deref(),
            req.public_key.as_deref(),
        )
        .await?;

    let user = state
        .storage
        .get_user(&auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

/// List current user's devices
pub async fn list_devices(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Device>>> {
    let devices = state.storage.list_user_devices(&auth.user_id).await?;
    Ok(Json(devices))
}

/// Remove a device
pub async fn remove_device(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    // Verify device belongs to user
    let device = state
        .storage
        .get_device(&device_id)
        .await?
        .ok_or(AppError::NotFound("Device not found".to_string()))?;

    if device.user_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    // Can't remove current device
    if device_id == auth.device_id {
        return Err(AppError::BadRequest("Cannot remove current device".to_string()));
    }

    state.storage.delete_device(&device_id).await?;

    // Disconnect if online
    state.ws_manager.unregister(&device_id);

    Ok(Json(serde_json::json!({ "success": true })))
}
