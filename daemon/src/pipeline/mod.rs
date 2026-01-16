// mod.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

pub mod manager;
pub mod parser;

pub use manager::{PipelineInfo, PipelineManager};
pub use parser::Pipeline;

/// Grace period in milliseconds to wait for bus watcher to shutdown
pub const SHUTDOWN_GRACE_PERIOD_MS: u64 = 150;

#[cfg(test)]
mod manager_tests;
