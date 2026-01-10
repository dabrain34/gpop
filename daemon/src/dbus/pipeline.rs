// pipeline.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;
use zbus::object_server::SignalEmitter;
use zbus::{interface, zvariant::ObjectPath};

use crate::event::PipelineState;
use crate::pipeline::PipelineManager;

pub struct PipelineInterface {
    pub manager: Arc<PipelineManager>,
    pub pipeline_id: String,
}

#[interface(name = "org.gpop.Pipeline")]
impl PipelineInterface {
    async fn set_state(&self, state: &str) -> zbus::fdo::Result<bool> {
        let state: PipelineState = state
            .parse()
            .map_err(|e: String| zbus::fdo::Error::Failed(e))?;

        self.manager
            .set_state(&self.pipeline_id, state)
            .await
            .map(|_| true)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn play(&self) -> zbus::fdo::Result<bool> {
        self.manager
            .play(&self.pipeline_id)
            .await
            .map(|_| true)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn pause(&self) -> zbus::fdo::Result<bool> {
        self.manager
            .pause(&self.pipeline_id)
            .await
            .map(|_| true)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn stop(&self) -> zbus::fdo::Result<bool> {
        self.manager
            .stop(&self.pipeline_id)
            .await
            .map(|_| true)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    #[zbus(property)]
    async fn id(&self) -> &str {
        &self.pipeline_id
    }

    #[zbus(property)]
    async fn description(&self) -> zbus::fdo::Result<String> {
        self.manager
            .get_pipeline_description(&self.pipeline_id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    #[zbus(property, name = "State")]
    async fn current_state(&self) -> zbus::fdo::Result<String> {
        let info = self
            .manager
            .get_pipeline_info(&self.pipeline_id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(info.state.to_string())
    }

    #[zbus(property)]
    async fn streaming(&self) -> zbus::fdo::Result<bool> {
        let info = self
            .manager
            .get_pipeline_info(&self.pipeline_id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(info.streaming)
    }

    #[zbus(signal, name = "StateChanged")]
    async fn emit_state_changed(
        emitter: &SignalEmitter<'_>,
        old_state: &str,
        new_state: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn error(emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn eos(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl PipelineInterface {
    pub fn new(manager: Arc<PipelineManager>, pipeline_id: String) -> Self {
        Self {
            manager,
            pipeline_id,
        }
    }

    pub fn object_path(index: u32) -> ObjectPath<'static> {
        ObjectPath::try_from(format!("/org/gpop/Pipeline{}", index)).unwrap()
    }
}
