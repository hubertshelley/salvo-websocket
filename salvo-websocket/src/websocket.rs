use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures_util::StreamExt;
use salvo::Error;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use salvo::extra::ws::{Message, WebSocket, WebSocketUpgrade};
use salvo::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

struct WebSocketController<T> {
    pub ws_list: HashMap<AtomicUsize, T>,
    pub caller_book: HashMap<String, Vec<UnboundedSender<Result<Message, Error>>>>,
}

impl<T> WebSocketController<T> {
    pub fn new() -> Self {
        WebSocketController {
            ws_list: HashMap::new(),
            caller_book: HashMap::new(),
        }
    }

    /// 发送消息到群组
    pub fn send_group(&mut self, group: String, message: Message) -> Result<(), Error> {
        let senders = self.caller_book.get(group.as_str());
        match senders {
            None => { Err("群组不存在".into()) }
            Some(senders) => {
                for sender in senders.iter() {
                    sender.send(Ok(message.clone()))
                }
                Ok(())
            }
        }
    }

    ///加入群组
    pub fn join_group(&mut self, group: String, sender: UnboundedSender<Result<Message, Error>>) -> Result<(), Error> {
        let senders = self.caller_book.get(group.as_str());
        match senders {
            None => {
                self.caller_book.insert(group, vec![sender]);
                Ok(())
            }
            Some(mut senders) => {
                senders.insert(0, sender);
                Ok(())
            }
        }
    }
}

pub static NEXT_WS_ID: AtomicUsize = AtomicUsize::new(1);
pub static WS_CONTROLLER: Arc<RwLock<WebSocketController<dyn WebSocketHandler>>> = Arc::new(RwLock::new(WebSocketController::<dyn WebSocketHandler>::new()));

#[async_trait]
pub trait WebSocketHandler {
    #[handler]
    async fn websocket_handler(&self, req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
        WebSocketUpgrade::new().handle(req, res, self.handle_socket).await
    }

    async fn handle_socket(&self, ws: WebSocket) {
        // Use a counter to assign a new unique ID for this user.
        let my_id = NEXT_WS_ID.fetch_add(1, Ordering::Relaxed);

        tracing::info!("new chat user: {}", my_id);

        // Split the socket into a sender and receive of messages.
        let (user_ws_tx, mut user_ws_rx) = ws.split();

        // Use an unbounded channel to handle buffering and flushing of messages
        // to the websocket...
        let (tx, rx) = mpsc::unbounded_channel();
        let rx = UnboundedReceiverStream::new(rx);
        let fut = rx.forward(user_ws_tx).map(|result| {
            if let Err(e) = result {
                tracing::error!(error = ?e, "websocket send error");
            }
        });
        tokio::task::spawn(fut);
        let fut = async move {
            self.on_connected().await;

            while let Some(result) = user_ws_rx.next().await {
                let msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("websocket error(uid={}): {}", my_id, e);
                        break;
                    }
                };
                self.receive_message(msg).await;
            }

            self.on_disconnected().await;
        };
        tokio::task::spawn(fut);
    }

    async fn on_connected(&self) -> Result<()>;

    async fn on_disconnected(&self) -> Result<()>;

    async fn receive_message(&self, msg: Message) -> Result<()>;

    async fn send_message(&self, msg: Message) -> Result<()>;
}

#[cfg(test)]
mod test {
    struct User {
        name: String,
    }

    impl WebSocketHandler for User {}

    #[tokio::test]
    async fn websocket_test() {
        user = User {
            name: "test_user".to_string()
        }
    }
}