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
    Some(vec!["get_data".into()].into()).into()
}

#[sabi_extern_fn]
pub fn handle_message(command: RString, args: RVec<RString>) -> ROption<RVec<u8>> {
    assert_eq!(Into::<String>::into(command), "get_data");
    println!("example plugin args: {:?}", args);
    Some(
        json!(vec![
            Value::Bool(true),
            Value::String("datadata1".to_string()),
            Value::String("2data".to_string()),
            Value::String("da4ta".to_string())
        ])
        .to_string()
        .as_bytes()
        .into(),
    )
    .into()
}