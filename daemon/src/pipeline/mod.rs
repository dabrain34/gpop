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

#[cfg(test)]
mod manager_tests;
