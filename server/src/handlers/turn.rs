//! TURN credentials handler for WebRTC

use axum::{extract::State, Json};
use crate::{
    crypto,
    error::Result,
    models::TurnCredentialsResponse,
    AppState,
};

use super::AuthUser;

/// Get TURN server credentials for WebRTC
pub async fn get_credentials(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<TurnCredentialsResponse>> {
    let turn_config = &state.config.turn;

    if !turn_config.enabled {
        return Ok(Json(TurnCredentialsResponse {
            urls: vec![],
            username: String::new(),
            credential: String::new(),
            credential_type: String::new(),
            ttl: 0,
        }));
    }

    // Generate time-limited credentials
    let (username, credential) = crypto::generate_turn_credentials(
        &turn_config.username,
        &turn_config.credential,
        turn_config.ttl_seconds,
    );

    Ok(Json(TurnCredentialsResponse {
        urls: turn_config.urls.clone(),
        username,
        credential,
        credential_type: turn_config.credential_type.clone(),
        ttl: turn_config.ttl_seconds,
    }))
}
