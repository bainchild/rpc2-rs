use serde_json::Value;
pub(crate) const PLUGIN_NAME: &str = "example";
pub(crate) const EVENT_FILTER: [&str; 3] = ["get_data", "get_medium_data", "get_large_data"];
pub(crate) const MEDIUM_STRING: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
pub(crate) fn handle_command(command: String, _args: Vec<String>) -> Vec<serde_json::Value> {
    match command.to_string().as_str() {
        "get_data" => vec![
            Value::Bool(true),
            Value::String("datadata1".to_string()),
            Value::String("2data".to_string()),
            Value::String("da4ta".to_string()),
        ],
        "get_medium_data" => {
            let mut v = Vec::with_capacity(50);
            v.push(Value::Bool(true));
            for _ in 1..50 {
                v.push(Value::String(MEDIUM_STRING.to_string()));
            }
            v
        }
        "get_large_data" => {
            vec![
                Value::Bool(true),
                Value::String(include_str!("large_data.txt").to_string()),
            ]
        }
        _ => vec![
            Value::Bool(false),
            Value::String("No such function".to_string()),
        ],
    }
}
#[cfg(feature = "sabi")]
mod sabi;
#[cfg(feature = "sabi")]
pub use crate::sabi::*;

mod builtin;
pub use crate::builtin::builtin_create;
