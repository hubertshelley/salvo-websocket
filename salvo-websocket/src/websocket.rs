use futures_util::{FutureExt, StreamExt};
use salvo::Error;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use once_cell::sync::Lazy;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use salvo::extra::ws::{Message, WebSocket};
use salvo::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

pub struct WebSocketController {
    pub caller_book: HashMap<String, Vec<UnboundedSender<Result<Message, Error>>>>,
}

impl WebSocketController {
    pub fn new() -> Self {
        WebSocketController {
            caller_book: HashMap::new(),
        }
    }

    /// 发送消息到群组
    pub fn send_group(&mut self, group: String, message: Message) -> Result<(), Error> {
        let senders = self.caller_book.get(group.as_str());
        match senders {
            None => panic!("群组不存在"),
            Some(senders) => {
                for sender in senders.iter() {
                    sender.send(Ok(message.clone())).unwrap();
                }
                Ok(())
            }
        }
    }

    ///加入群组
    pub fn join_group(
        &mut self,
        group: String,
        sender: UnboundedSender<Result<Message, Error>>,
    ) -> Result<(), Error> {
        match self.caller_book.get_mut(group.as_str()) {
            None => {
                self.caller_book.insert(group, vec![sender]);
            }
            Some(senders) => {
                senders.insert(0, sender);
            }
        };
        Ok(())
    }
}

impl Default for WebSocketController {
    fn default() -> Self {
        Self::new()
    }
}

type Controller = RwLock<WebSocketController>;

static NEXT_WS_ID: AtomicUsize = AtomicUsize::new(1);
pub static WS_CONTROLLER: Lazy<Controller> = Lazy::new(Controller::default);


pub async fn handle_socket<T: WebSocketHandler + Send + Sync>(ws: WebSocket, _self: T) {
    // let _self: T = ws.req.parse_queries().unwrap();
    // Use a counter to assign a new unique ID for this user.
    let ws_id = NEXT_WS_ID.fetch_add(1, Ordering::Relaxed);

    tracing::info!("new ws connected: {}", ws_id);

    // Split the socket into a sender and receive of messages.
    let (ws_sender, mut ws_reader) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (sender, reader) = mpsc::unbounded_channel();
    let reader = UnboundedReceiverStream::new(reader);
    let fut = reader.forward(ws_sender).map(|result| {
        if let Err(e) = result {
            tracing::error!(error = ?e, "websocket send error");
        }
    });
    tokio::task::spawn(fut);
    let fut = async move {
        _self.on_connected(ws_id, sender).await;

        while let Some(result) = ws_reader.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("websocket error(uid={}): {}", ws_id, e);
                    break;
                }
            };
            // send_msg(ws_id, msg).await;
            _self.on_receive_message(msg).await;
        }

        _self.on_disconnected(ws_id).await;
    };
    tokio::task::spawn(fut);
}

async fn on_connected(ws_id: usize, sender: UnboundedSender<Result<Message, Error>>) {
    WS_CONTROLLER.write().await.join_group("group1".to_string(), sender).unwrap();
}

async fn send_msg(ws_id: usize, msg: Message) {
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };
    let new_msg = format!("<User#{}>: {}", ws_id, msg);
    WS_CONTROLLER.write().await.send_group("group1".to_string(), Message::text(new_msg.clone())).unwrap();
}

#[async_trait]
pub trait WebSocketHandler {
    async fn on_connected(&self, ws_id: usize, sender: UnboundedSender<Result<Message, Error>>);

    async fn on_disconnected(&self, ws_id: usize);

    async fn on_receive_message(&self, msg: Message);

    async fn on_send_message(&self, msg: Message) -> Result<Message, Error>;
}

#[cfg(test)]
mod test {
    use salvo::Error;
    use salvo::extra::ws::Message;
    use tokio::sync::mpsc::UnboundedSender;
    use crate::websocket::WS_CONTROLLER;

    // struct User {
    //     name: String,
    // }
    //
    // #[async_trait]
    // impl super::WebSocketHandler for User {
    //     async fn on_connected(&self, ws_id: usize, sender: UnboundedSender<Result<Message, Error>>) {
    //         WS_CONTROLLER.write().await.join_group("group1".to_string(), sender);
    //     }
    //     async fn on_disconnected(&self, ws_id: usize) {}
    //     async fn on_receive_message(&self, msg: Message) {
    //         eprintln!("{:?}", msg);
    //     }
    //     async fn on_send_message(&self, msg: Message) -> Result<Message, Error> {
    //         eprintln!("{:?}", msg);
    //         Ok(msg)
    //     }
    // }

    #[tokio::test]
    async fn websocket_test() {
        // let user = User {
        //     name: "test_user".to_string(),
        // };
    }
}
