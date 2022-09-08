use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use salvo::extra::ws::{Message, WebSocket, WebSocketUpgrade};
use salvo::prelude::*;

struct WebSocketController {
    pub ws_list: HashMap<AtomicUsize, T>,
    pub caller_book: HashMap<String, Vec<UnboundedSender>>,
}

impl WebSocketController {
    pub fn new() -> Self {
        WebSocketController {
            ws_list: vec![],
            caller_book: HashMap::new(),
        }
    }

    pub fn send_group(&mut self, group: String, message: Message) -> Result<()> {
        self.caller_book.insert(group, message)
    }

    pub fn get_websocket()
}

pub static NEXT_WS_ID: AtomicUsize = AtomicUsize::new(1);
pub static WS_CONTROLLER: WebSocketController = Arc::new(RwLock::new(WebSocketController::new()));

pub trait WebSocket {
    #[handler]
    async fn handler(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
        WebSocketUpgrade::new().handle(req, res, handle_socket).await
    }

    async fn handle_socket(&self, ws: WebSocket) {
        // Use a counter to assign a new unique ID for this user.
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

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
            ONLINE_USERS.write().await.insert(my_id, tx);

            while let Some(result) = user_ws_rx.next().await {
                let msg = match result {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("websocket error(uid={}): {}", my_id, e);
                        break;
                    }
                };
                self.receive_message(WebSocketController.Read().await.)
                user_message(my_id, msg).await;
            }

            user_disconnected(my_id).await;
        };
        tokio::task::spawn(fut);
    }

    async fn connected(&self) -> Result<()> {}

    async fn receive_message(&self, msg: Message) -> Result<()> {}

    async fn send_message(&self, msg: Message) -> Result<()> {}
}

#[cfg(test)]
mod test {
    struct User {
        name: String,
    }

    impl WebSocket for User {}

    #[tokio::test]
    async fn websocket_test() {
        user = User {
            name: "test_user".to_string()
        }
    }
}