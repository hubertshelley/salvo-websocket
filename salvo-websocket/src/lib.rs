extern crate core;

#[deny(doc)]
mod websocket;

pub use websocket::WebSocketHandler;
pub use websocket::WS_CONTROLLER;
pub use websocket::handle_socket;
