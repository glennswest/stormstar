//! Host inventory CRUD + registration.

use axum::{
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, AppError};
use crate::db::models::{Host, ActivationKey, HostFact};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hosts", get(list).post(create))
        .route("/hosts/{id}", get(show).put(update).delete(delete))
        .route("/hosts/register", axum::routing::post(register))
}

#[derive(Deserialize)]
struct CreateHost {
    org_id: String,
    hostname: String,
    #[serde(default = "default_arch")]
    arch: String,
    #[serde(default)]
    os: String,
}

fn default_arch() -> String { "x86_64".to_string() }

#[derive(Deserialize)]
struct UpdateHost {
    hostname: Option<String>,
    os: Option<String>,
    env_id: Option<String>,
    cv_id: Option<String>,
    #[serde(default)]
    facts: Option<Vec<HostFact>>,
    #[serde(default)]
    installed_packages: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RegisterRequest {
    activation_key: String,
    hostname: String,
    #[serde(default = "default_arch")]
    arch: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    facts: Vec<HostFact>,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Host>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<Host> = r.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(items))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Host>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Host = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("host not found"))?;
    Ok(Json(item))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateHost>,
) -> Result<(StatusCode, Json<Host>), AppError> {
    let host = Host {
        id: Uuid::new_v4().to_string(),
        org_id: body.org_id,
        hostname: body.hostname,
        arch: body.arch,
        os: body.os,
        env_id: None,
        cv_id: None,
        activation_key_id: None,
        facts: Vec::new(),
        installed_packages: Vec::new(),
        applicable_errata: Vec::new(),
        last_checkin: None,
        registered_at: chrono::Utc::now().to_rfc3339(),
    };

    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    rw.insert(host.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(host)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateHost>,
) -> Result<Json<Host>, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let old: Host = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("host not found"))?;

    let mut updated = old.clone();
    if let Some(hostname) = body.hostname { updated.hostname = hostname; }
    if let Some(os) = body.os { updated.os = os; }
    if let Some(env_id) = body.env_id { updated.env_id = Some(env_id); }
    if let Some(cv_id) = body.cv_id { updated.cv_id = Some(cv_id); }
    if let Some(facts) = body.facts { updated.facts = facts; }
    if let Some(pkgs) = body.installed_packages { updated.installed_packages = pkgs; }
    updated.last_checkin = Some(chrono::Utc::now().to_rfc3339());

    rw.update(old, updated.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(updated))
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Host = rw.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("host not found"))?;

    rw.remove(item).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<Host>), AppError> {
    let rw = state.db.rw_transaction().map_err(|e| AppError::internal(e.to_string()))?;

    // Find activation key by key string
    let keys: Vec<ActivationKey> = rw.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let old_key = keys.into_iter()
        .find(|k| k.key == body.activation_key)
        .ok_or_else(|| AppError::bad_request("invalid activation key"))?;

    // Check max hosts
    if let Some(max) = old_key.max_hosts {
        if old_key.usage_count >= max {
            return Err(AppError::bad_request("activation key usage limit reached"));
        }
    }

    let host = Host {
        id: Uuid::new_v4().to_string(),
        org_id: old_key.org_id.clone(),
        hostname: body.hostname,
        arch: body.arch,
        os: body.os,
        env_id: Some(old_key.env_id.clone()),
        cv_id: Some(old_key.cv_id.clone()),
        activation_key_id: Some(old_key.id.clone()),
        facts: body.facts,
        installed_packages: Vec::new(),
        applicable_errata: Vec::new(),
        last_checkin: Some(chrono::Utc::now().to_rfc3339()),
        registered_at: chrono::Utc::now().to_rfc3339(),
    };

    // Increment key usage
    let mut updated_key = old_key.clone();
    updated_key.usage_count += 1;

    rw.insert(host.clone()).map_err(|e| AppError::internal(e.to_string()))?;
    rw.update(old_key, updated_key).map_err(|e| AppError::internal(e.to_string()))?;
    rw.commit().map_err(|e| AppError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(host)))
}
