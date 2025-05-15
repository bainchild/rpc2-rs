use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    std_types::{ROption, RString, RVec},
};
use rpc2_interface::{RPC2Plugin, RPC2PluginRef};
use serde_json::{json, Value};
#[export_root_module]
pub fn get_library() -> RPC2PluginRef {
    RPC2Plugin {
        get_name,
        get_event_mask,
        handle_message,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
pub fn get_name() -> RString {
    "example".into()
}

#[sabi_extern_fn]
pub fn init() {
    println!("example plugin init");
}

#[sabi_extern_fn]
pub fn get_event_mask() -> ROption<RVec<RString>> {
    Some(
        vec![
            "get_data".into(),
            "get_medium_data".into(),
            "get_large_data".into(),
        ]
        .into(),
    )
    .into()
}

const MEDIUM_STRING: &'static str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

#[sabi_extern_fn]
pub fn handle_message(command: RString, args: RVec<RString>) -> ROption<RVec<u8>> {
    println!("example plugin args: {:?}", args);
    Some(
        json!(match command.to_string().as_str() {
            "get_data" => vec![
                Value::Bool(true),
                Value::String("datadata1".to_string()),
                Value::String("2data".to_string()),
                Value::String("da4ta".to_string())
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
                let mut v = Vec::with_capacity(2);
                v.push(Value::Bool(true));
                v.push(Value::String(include_str!("large_data.txt").to_string()));
                v
            }
            _ => vec![
                Value::Bool(false),
                Value::String("No such function".to_string())
            ],
        })
        .to_string()
        .as_bytes()
        .into(),
    )
    .into()
}
