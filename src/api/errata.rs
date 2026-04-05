//! Errata listing (read-only, populated by sync).

use axum::{
    Router, Json,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;

use super::{AppState, AppError};
use crate::db::models::Erratum;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/errata", get(list))
        .route("/errata/{id}", get(show))
}

#[derive(Deserialize, Default)]
struct ErrataFilter {
    repo_id: Option<String>,
    erratum_type: Option<String>,
    severity: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(filter): Query<ErrataFilter>,
) -> Result<Json<Vec<Erratum>>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let items: Vec<Erratum> = r.scan().primary()
        .map_err(|e| AppError::internal(e.to_string()))?
        .all()
        .map_err(|e| AppError::internal(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let filtered: Vec<Erratum> = items.into_iter()
        .filter(|e| {
            if let Some(ref repo) = filter.repo_id {
                if &e.repo_id != repo { return false; }
            }
            if let Some(ref t) = filter.erratum_type {
                let et = format!("{:?}", e.erratum_type);
                if &et != t { return false; }
            }
            if let Some(ref s) = filter.severity {
                let es = format!("{:?}", e.severity);
                if &es != s { return false; }
            }
            true
        })
        .collect();

    Ok(Json(filtered))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Erratum>, AppError> {
    let r = state.db.r_transaction().map_err(|e| AppError::internal(e.to_string()))?;
    let item: Erratum = r.get().primary(id)
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("erratum not found"))?;
    Ok(Json(item))
}
