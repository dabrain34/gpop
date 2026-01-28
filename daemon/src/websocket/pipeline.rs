// pipeline.rs
//
// Copyright 2026 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

//! Pipeline-specific WebSocket protocol types.
//!
//! This module contains request parameters and response types for
//! individual pipeline operations (play, pause, stop, get_position, etc.).
//! This is the WebSocket equivalent of the DBus `PipelineInterface`.

use serde::{Deserialize, Serialize};

use crate::gst::PipelineState;

// Request parameter types for pipeline operations

#[derive(Debug, Clone, Deserialize)]
pub struct PipelineIdParams {
    pub pipeline_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OptionalPipelineIdParams {
    #[serde(default)]
    pub pipeline_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetStateParams {
    pub pipeline_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePipelineParams {
    pub pipeline_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SnapshotParams {
    #[serde(default)]
    pub pipeline_id: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
}

// Response result types for pipeline operations

#[derive(Debug, Clone, Serialize)]
pub struct PipelineInfoResult {
    pub id: String,
    pub description: String,
    pub state: PipelineState,
    pub streaming: bool,
}

impl From<crate::gst::PipelineInfo> for PipelineInfoResult {
    fn from(info: crate::gst::PipelineInfo) -> Self {
        Self {
            id: info.id,
            description: info.description,
            state: info.state,
            streaming: info.streaming,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineSnapshot {
    pub id: String,
    pub dot: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotResult {
    #[serde(rename = "type")]
    pub response_type: String,
    pub pipelines: Vec<PipelineSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionResult {
    /// Current position in nanoseconds, if available
    pub position_ns: Option<u64>,
    /// Total duration in nanoseconds, if available
    pub duration_ns: Option<u64>,
    /// Progress as a value between 0.0 and 1.0, if both position and duration are available
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuccessResult {
    pub success: bool,
}
