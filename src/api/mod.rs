//! HTTP transport layer.

mod error;
mod handlers;
mod model;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::bridge::BridgeService;

pub(crate) fn router(service: Arc<BridgeService>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/workflows", post(handlers::create_workflow))
        .route("/workflows/:workflow_id", get(handlers::get_workflow))
        .with_state(service)
}
