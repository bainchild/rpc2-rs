pub(crate) static PLUGIN_NAME: &str = "WebSocket";
pub(crate) static EVENT_FILTER: [&str; 4] = [
    "websocket_open",
    "websocket_close",
    "websocket_send",
    "websocket_poll",
];
// TODO: abi_stable for this plugin
// #[cfg(feature = "sabi")]
// mod sabi
// #[cfg(feature = "sabi")]
// pub use crate::sabi::*;

mod builtin;
pub use crate::builtin::builtin_create;
