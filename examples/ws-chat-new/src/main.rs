// Copyright (c) 2018-2020 Sean McArthur
// Licensed under the MIT license http://opensource.org/licenses/MIT
//
// port from https://github.com/seanmonstar/warp/blob/master/examples/websocket_chat.rs

use std::borrow::Borrow;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{FutureExt, StreamExt};
use once_cell::sync::Lazy;
use salvo::Error;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use salvo::extra::ws::{Message, WebSocket, WebSocketUpgrade};
use salvo::http::ParseError;
use salvo::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use salvo_websocket::{handle_socket, WebSocketHandler, WS_CONTROLLER};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct User {
    name: String,
}

#[async_trait]
impl WebSocketHandler for User {
    async fn on_connected(&self, ws_id: usize, sender: UnboundedSender<Result<Message, Error>>) {
        tracing::info!("{} connected", ws_id);
        // eprintln!("{} connected", ws_id);
        match WS_CONTROLLER.write().await.send_group("group".to_string(), Message::text(format!("{:?} joined!", self.name))) {
            Ok(_) => {
                WS_CONTROLLER.write().await.join_group("group".to_string(), sender).unwrap();
            }
            Err(e) => {
                eprintln!("{:?}", e);
                WS_CONTROLLER.write().await.join_group("group".to_string(), sender).unwrap();
            }
        };
    }

    async fn on_disconnected(&self, ws_id: usize) {
        tracing::info!("{} disconnected", ws_id);
        // eprintln!("{} disconnected", ws_id);
    }

    async fn on_receive_message(&self, msg: Message) {
        tracing::info!("{:?} received", msg);
        // eprintln!("{:?} received", msg);
        WS_CONTROLLER.write().await.send_group("group".to_string(), msg).unwrap();
    }

    async fn on_send_message(&self, msg: Message) -> Result<Message, Error> {
        eprintln!("{:?} sending", msg);
        tracing::info!("{:?} sending", msg);
        Ok(msg)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let router = Router::new()
        .handle(index)
        .push(Router::with_path("chat").handle(user_connected));
    tracing::info!("Listening on http://127.0.0.1:7878");
    Server::new(TcpListener::bind("127.0.0.1:7878")).serve(router).await;
}

#[handler]
async fn user_connected(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    let user: Result<User, ParseError> = req.parse_queries();
    match user {
        Ok(user) => {
            WebSocketUpgrade::new().handle(req, res, |mut ws| async move {
                eprintln!("{:?} linked", user);
                handle_socket(ws, user).await;
            }).await
        }
        Err(_err) => {
            Err(StatusError::bad_request())
        }
    }
}

#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(INDEX_HTML));
}

static INDEX_HTML: &str = include_str!("./index.html");
