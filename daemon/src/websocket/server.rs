// server.rs
//
// Copyright 2026 Stéphane Cerveau <scerveau@igalia.com>
//
// This file is part of GstPrinceOfParser
//
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use crate::error::Result;
use crate::gst::{EventReceiver, PipelineManager};

use super::manager::ManagerInterface;
use super::pipeline::SnapshotParams;
use super::protocol::Request;
use super::{CLIENT_MESSAGE_BUFFER, MAX_CONCURRENT_CLIENTS};

type ClientTx = mpsc::Sender<Message>;
type ClientMap = Arc<RwLock<HashMap<SocketAddr, ClientTx>>>;

pub struct WebSocketServer {
    addr: SocketAddr,
    manager: Arc<PipelineManager>,
    clients: ClientMap,
}

impl WebSocketServer {
    pub fn new(addr: SocketAddr, manager: Arc<PipelineManager>) -> Self {
        Self {
            addr,
            manager,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self, mut event_rx: EventReceiver) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("WebSocket server listening on ws://{}", self.addr);

        let clients = Arc::clone(&self.clients);
        let manager = Arc::clone(&self.manager);

        // Spawn event broadcaster
        let broadcast_clients = Arc::clone(&clients);
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Serialize once, then clone for each client
                        // Note: Message::Text requires owned String, so we must clone per-client
                        let msg = serde_json::to_string(&event).unwrap();
                        let clients = broadcast_clients.read().await;
                        for (addr, tx) in clients.iter() {
                            // Use try_send to avoid blocking; if buffer is full, client is slow
                            if tx.try_send(Message::Text(msg.clone().into())).is_err() {
                                debug!("Failed to send event to client {} (buffer full or disconnected)", addr);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket broadcaster lagged by {} messages", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Event channel closed, stopping WebSocket broadcaster");
                        break;
                    }
                }
            }
        });

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let clients = Arc::clone(&clients);
                    let manager = Arc::clone(&manager);
                    tokio::spawn(handle_connection(stream, addr, clients, manager));
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    pub fn clients(&self) -> ClientMap {
        Arc::clone(&self.clients)
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    clients: ClientMap,
    manager: Arc<PipelineManager>,
) {
    info!("New WebSocket connection from {}", addr);

    // Check connection limit before accepting
    {
        let clients_map = clients.read().await;
        if clients_map.len() >= MAX_CONCURRENT_CLIENTS {
            warn!("Max clients ({}) reached, rejecting connection from {}", MAX_CONCURRENT_CLIENTS, addr);
            return;
        }
    }

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {}: {}", addr, e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(CLIENT_MESSAGE_BUFFER);

    // Register client
    {
        let mut clients_map = clients.write().await;
        clients_map.insert(addr, tx);
    }

    let handler = ManagerInterface::new(manager);

    // Spawn task to forward messages from channel to WebSocket
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("Received from {}: {}", addr, text);

                let request = match serde_json::from_str::<Request>(&text) {
                    Ok(req) => req,
                    Err(e) => {
                        error!("Failed to parse request from {}: {}", addr, e);

                        // Try to extract the ID from malformed JSON for better error correlation
                        let id = serde_json::from_str::<serde_json::Value>(&text)
                            .ok()
                            .and_then(|v| v.get("id").and_then(|id| id.as_str()).map(String::from))
                            .unwrap_or_else(|| "unknown".to_string());

                        let response = super::protocol::Response::parse_error(
                            id,
                            format!("Parse error: {}", e),
                        );
                        let response_json = serde_json::to_string(&response).unwrap();
                        let clients_map = clients.read().await;
                        if let Some(tx) = clients_map.get(&addr) {
                            let _ = tx.try_send(Message::Text(response_json.into()));
                        }
                        continue;
                    }
                };

                // Handle snapshot specially - returns direct response without JSON-RPC wrapper
                let response_json = if request.method == "snapshot" {
                    let params: SnapshotParams =
                        serde_json::from_value(request.params).unwrap_or_default();
                    match handler.snapshot(params).await {
                        Ok(result) => serde_json::to_string(&result).unwrap(),
                        Err(e) => {
                            let response =
                                super::protocol::Response::from_gpop_error(request.id, &e);
                            serde_json::to_string(&response).unwrap()
                        }
                    }
                } else {
                    let response = handler.handle(request).await;
                    serde_json::to_string(&response).unwrap()
                };

                let clients_map = clients.read().await;
                if let Some(tx) = clients_map.get(&addr) {
                    let _ = tx.try_send(Message::Text(response_json.into()));
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client {} disconnected", addr);
                break;
            }
            Ok(Message::Ping(data)) => {
                let clients_map = clients.read().await;
                if let Some(tx) = clients_map.get(&addr) {
                    let _ = tx.try_send(Message::Pong(data));
                }
            }
            Ok(_) => {}
            Err(e) => {
                error!("Error receiving message from {}: {}", addr, e);
                break;
            }
        }
    }

    // Unregister client
    {
        let mut clients_map = clients.write().await;
        clients_map.remove(&addr);
    }

    sender_task.abort();
    info!("Connection closed for {}", addr);
}
