//! This crate is websocket tool for salvo.
#![deny(missing_docs)]
extern crate core;

mod websocket;

pub use websocket::WebSocketHandler;
pub use websocket::WS_CONTROLLER;
pub use websocket::handle_socket;
