// manager.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;
use tracing::debug;

use crate::event::PipelineState;
use crate::pipeline::PipelineManager;

use super::protocol::*;
use super::DEFAULT_PIPELINE_ID;

pub struct MessageHandler {
    manager: Arc<PipelineManager>,
}

impl MessageHandler {
    pub fn new(manager: Arc<PipelineManager>) -> Self {
        Self { manager }
    }

    pub async fn handle(&self, request: Request) -> Response {
        debug!("Handling request: {} (id: {})", request.method, request.id);

        match request.method.as_str() {
            "list_pipelines" => self.list_pipelines(request.id).await,
            "create_pipeline" => self.create_pipeline(request).await,
            "remove_pipeline" => self.remove_pipeline(request).await,
            "get_pipeline" => self.get_pipeline(request).await,
            "set_state" => self.set_state(request).await,
            "play" => self.play(request).await,
            "pause" => self.pause(request).await,
            "stop" => self.stop(request).await,
            "get_position" => self.get_position(request).await,
            // snapshot is handled separately in server.rs
            _ => Response::method_not_found(request.id, &request.method),
        }
    }

    async fn list_pipelines(&self, id: String) -> Response {
        let infos = self.manager.list_pipelines().await;
        let pipelines: Vec<PipelineInfoResult> = infos
            .into_iter()
            .map(|info| PipelineInfoResult {
                id: info.id,
                description: info.description,
                state: info.state,
                streaming: info.streaming,
            })
            .collect();

        let result = ListPipelinesResult { pipelines };
        Response::success(id, serde_json::to_value(result).unwrap())
    }

    async fn create_pipeline(&self, request: Request) -> Response {
        let params: CreatePipelineParams = match serde_json::from_value(request.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::invalid_params(request.id, format!("Invalid params: {}", e))
            }
        };

        match self.manager.add_pipeline(&params.description).await {
            Ok(pipeline_id) => {
                let result = PipelineCreatedResult { pipeline_id };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn remove_pipeline(&self, request: Request) -> Response {
        let params: PipelineIdParams = match serde_json::from_value(request.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::invalid_params(request.id, format!("Invalid params: {}", e))
            }
        };

        match self.manager.remove_pipeline(&params.pipeline_id).await {
            Ok(()) => Response::success(request.id, serde_json::json!({})),
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn get_pipeline(&self, request: Request) -> Response {
        let params: PipelineIdParams = match serde_json::from_value(request.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::invalid_params(request.id, format!("Invalid params: {}", e))
            }
        };

        match self.manager.get_pipeline_info(&params.pipeline_id).await {
            Ok(info) => {
                let result = PipelineInfoResult {
                    id: info.id,
                    description: info.description,
                    state: info.state,
                    streaming: info.streaming,
                };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn set_state(&self, request: Request) -> Response {
        let params: SetStateParams = match serde_json::from_value(request.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::invalid_params(request.id, format!("Invalid params: {}", e))
            }
        };

        let state: PipelineState = match params.state.parse() {
            Ok(s) => s,
            Err(e) => return Response::invalid_params(request.id, e),
        };

        match self.manager.set_state(&params.pipeline_id, state).await {
            Ok(()) => {
                let result = SuccessResult { success: true };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn play(&self, request: Request) -> Response {
        let params: OptionalPipelineIdParams =
            serde_json::from_value(request.params).unwrap_or_default();

        let pipeline_id = params.pipeline_id.unwrap_or_else(|| DEFAULT_PIPELINE_ID.to_string());

        match self.manager.play(&pipeline_id).await {
            Ok(()) => {
                let result = SuccessResult { success: true };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn pause(&self, request: Request) -> Response {
        let params: OptionalPipelineIdParams =
            serde_json::from_value(request.params).unwrap_or_default();

        let pipeline_id = params.pipeline_id.unwrap_or_else(|| DEFAULT_PIPELINE_ID.to_string());

        match self.manager.pause(&pipeline_id).await {
            Ok(()) => {
                let result = SuccessResult { success: true };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    async fn stop(&self, request: Request) -> Response {
        let params: OptionalPipelineIdParams =
            serde_json::from_value(request.params).unwrap_or_default();

        let pipeline_id = params.pipeline_id.unwrap_or_else(|| DEFAULT_PIPELINE_ID.to_string());

        match self.manager.stop(&pipeline_id).await {
            Ok(()) => {
                let result = SuccessResult { success: true };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }

    pub async fn snapshot(&self, params: SnapshotParams) -> Option<SnapshotResult> {
        let pipeline_id = params.pipeline_id.unwrap_or_else(|| DEFAULT_PIPELINE_ID.to_string());

        match self
            .manager
            .get_dot(&pipeline_id, params.details.as_deref())
            .await
        {
            Ok(dot) => Some(SnapshotResult {
                response_type: "SnapshotResponse".to_string(),
                pipelines: vec![PipelineSnapshot {
                    id: pipeline_id,
                    dot,
                }],
            }),
            Err(_) => None,
        }
    }

    async fn get_position(&self, request: Request) -> Response {
        let params: OptionalPipelineIdParams =
            serde_json::from_value(request.params).unwrap_or_default();

        let pipeline_id = params.pipeline_id.unwrap_or_else(|| DEFAULT_PIPELINE_ID.to_string());

        match self.manager.get_position(&pipeline_id).await {
            Ok((position_ns, duration_ns)) => {
                let progress = match (position_ns, duration_ns) {
                    (Some(pos), Some(dur)) if dur > 0 => Some(pos as f64 / dur as f64),
                    _ => None,
                };

                let result = PositionResult {
                    position_ns,
                    duration_ns,
                    progress,
                };
                Response::success(request.id, serde_json::to_value(result).unwrap())
            }
            Err(e) => Response::from_gpop_error(request.id, &e),
        }
    }
}
