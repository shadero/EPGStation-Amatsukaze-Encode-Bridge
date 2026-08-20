//! Coordinates one complete EPGStation-to-Amatsukaze workflow.

mod paths;
mod request;
mod service;
mod status;
mod workflow;

pub(crate) use request::BridgeRequest;
pub(crate) use service::BridgeService;
pub(crate) use status::WorkflowStatus;
