use futures_util::{future, FutureExt, SinkExt, StreamExt};
use salvo::Error;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use once_cell::sync::Lazy;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use salvo::ws::{Message, WebSocket};
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
    pub fn send_group(&mut self, group: String, message: Message) -> Result<(), anyhow::Error> {
        let senders = self.caller_book.get_mut(group.as_str());
        match senders {
            None => Err(anyhow::anyhow!("群组不存在")),
            Some(senders) => {
                let mut pre_delete_list = vec![];
                for (index, sender) in senders.iter().enumerate() {
                    match sender.send(Ok(message.clone())) {
                        Ok(_) => {}
                        Err(_) => {
                            pre_delete_list.push(index);
                        }
                    };
                }
                for delete_index in pre_delete_list {
                    senders.remove(delete_index);
                }
                Ok(())
            }
        }
    }

    /// 加入群组
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
/// 全局默认websocket控制器
pub static WS_CONTROLLER: Lazy<Controller> = Lazy::new(Controller::default);

/// salvo websocket 处理方法
/// ## Example
/// ```
/// use salvo::{Request, Response};
/// use salvo::ws::WebSocketUpgrade;
/// use salvo::http::{ParseError, StatusError};
/// use salvo_websocket::handle_socket;
/// #[handler]
/// async fn user_connected(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
///     let user: Result<User, ParseError> = req.parse_queries();
///     match user {
///         Ok(user) => {
///             WebSocketUpgrade::new().upgrade(req, res, |ws| async move {
///                 handle_socket(ws, user).await;
///             }).await
///         }
///         Err(_err) => {
///             Err(StatusError::bad_request())
///         }
///     }
/// }
pub async fn handle_socket<T: WebSocketHandler + Send + Sync + 'static>(ws: WebSocket, _self: T) {
    // Use a counter to assign a new unique ID for this user.
    let ws_id = NEXT_WS_ID.fetch_add(1, Ordering::Relaxed);

    tracing::info!("new ws connected: {}", ws_id);

    // Split the socket into a sender and receive of messages.
    let (mut ws_sender, mut ws_reader) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (sender, reader) = mpsc::unbounded_channel();
    let reader = UnboundedReceiverStream::new(reader);
    let fut = reader.forward(ws_sender).map(|result| {
        eprintln!("发送：{:?}", result);
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
            eprintln!("on_receive_message message(uid={}): {:?}", ws_id, msg);
            _self.on_receive_message(msg).await;
        }

        _self.on_disconnected(ws_id).await;
    };
    tokio::task::spawn(fut);
}

#[async_trait]
/// websocket处理器trait
/// ## Example
/// ```
/// use salvo::Error;
/// use salvo::ws::Message;
/// use tokio::sync::mpsc::UnboundedSender;
/// use salvo_websocket::{WebSocketHandler, WS_CONTROLLER};
/// #[derive(Debug, Clone, Deserialize)]
/// struct User {
///     name: String,
///     room: String,
/// }
///
/// #[async_trait]
/// impl WebSocketHandler for User {
///     async fn on_connected(&self, ws_id: usize, sender: UnboundedSender<Result<Message, Error>>) {
///         tracing::info!("{} connected", ws_id);
///         WS_CONTROLLER.write().await.join_group(self.room.clone(), sender).unwrap();
///         WS_CONTROLLER.write().await.send_group(
///             self.room.clone(),
///             Message::text(format!("{:?} joined!", self.name)
///             ),
///         ).unwrap();
///     }
///
///     async fn on_disconnected(&self, ws_id: usize) {
///         tracing::info!("{} disconnected", ws_id);
///     }
///
///     async fn on_receive_message(&self, msg: Message) {
///         tracing::info!("{:?} received", msg);
///         let msg = if let Ok(s) = msg.to_str() {
///             s
///         } else {
///             return;
///         };
///         let new_msg = format!("<User#{}>: {}", self.name, msg);
///         WS_CONTROLLER.write().await.send_group(self.room.clone(), Message::text(new_msg.clone())).unwrap();
///     }
///
///     async fn on_send_message(&self, msg: Message) -> Result<Message, Error> {
///         tracing::info!("{:?} sending", msg);
///         Ok(msg)
///     }
/// }
///
/// ```
pub trait WebSocketHandler {
    /// websocket客户端连接事件
    async fn on_connected(&self, ws_id: usize, sender: UnboundedSender<Result<Message, Error>>);

    /// websocket客户端断开事件
    async fn on_disconnected(&self, ws_id: usize);

    /// websocket客户端收到消息事件
    async fn on_receive_message(&self, msg: Message);

    /// websocket客户端发送消息事件
    async fn on_send_message(&self, msg: Message) -> Result<Message, Error>;
}

#[cfg(test)]
mod test {
    // use salvo::Error;
    // use salvo::extra::ws::Message;
    // use tokio::sync::mpsc::UnboundedSender;
    // use crate::websocket::WS_CONTROLLER;

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
