pub mod builtins;
pub mod client;
pub mod server;

mod dem;

const QUAKE_CONNECTION_REQUEST: &[u8] = b"\x01QUAKE\x03";
const QUAKE_DISCONNECT_REQUEST: &[u8] = b"\x02";
