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
        eprintln!("{} connected", ws_id);
        WS_CONTROLLER.write().await.join_group("group".to_string(), sender);
    }

    async fn on_disconnected(&self, ws_id: usize) {
        eprintln!("{} disconnected", ws_id);
    }

    async fn on_receive_message(&self, msg: Message) {
        eprintln!("{:?} received", msg);
    }

    async fn on_send_message(&self, msg: Message) -> Result<Message, Error> {
        eprintln!("{:?} sending", msg);
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
    WebSocketUpgrade::new().handle(req, res, handle_socket::<User>).await
}

#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(INDEX_HTML));
}

static INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
    <head>
        <title>WS Chat</title>
    </head>
    <body>
        <h1>WS Chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="submit">Submit</button>
        <script>
            const chat = document.getElementById('chat');
            const msg = document.getElementById('msg');
            const submit = document.getElementById('submit');
            const ws = new WebSocket(`ws://${location.host}/chat`);

            ws.onopen = function() {
                chat.innerHTML = '<p><em>Connected!</em></p>';
            };

            ws.onmessage = function(msg) {
                showMessage(msg.data);
            };

            ws.onclose = function() {
                chat.getElementsByTagName('em')[0].innerText = 'Disconnected!';
            };

            submit.onclick = function() {
                const msg = text.value;
                ws.send(msg);
                text.value = '';

                showMessage('<You>: ' + msg);
            };
            function showMessage(data) {
                const line = document.createElement('p');
                line.innerText = data;
                chat.appendChild(line);
            }
        </script>
    </body>
</html>
"#;
