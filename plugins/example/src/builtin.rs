use crate::{EVENT_FILTER, PLUGIN_NAME, handle_command};
use rpc2_interface::builtin::RPC2BuiltinPlugin;
use serde_json::json;
#[derive(Default)]
struct Example {}
impl RPC2BuiltinPlugin for Example {
    fn get_name(&self) -> &'static str {
        PLUGIN_NAME
    }
    fn get_filter(&self) -> &'static [&'static str] {
        &EVENT_FILTER
    }
    fn handle_message(&mut self, cmd: String, args: Vec<String>) -> Option<Vec<u8>> {
        Some(
            json!(self::handle_command(cmd, args))
                .to_string()
                .as_bytes()
                .to_vec(),
        )
    }
}
pub fn builtin_create() -> impl RPC2BuiltinPlugin {
    Example::default()
}
