// mod.rs
//
// Copyright 2021 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

pub mod handler;
pub mod protocol;
pub mod server;

pub use server::WebSocketServer;

#[cfg(test)]
mod protocol_tests;
