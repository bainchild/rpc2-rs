use std::{
    path::{Component, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use crate::{chroot, handle_command, notify, EVENT_FILTER, PLUGIN_NAME, WORKSPACE_DIR};
use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    reexports::SelfOps,
    sabi_extern_fn,
    std_types::{ROption, RString, RVec},
};
use native_dialog::{DialogBuilder, MessageLevel};
use rpc2_interface::sabi::{RPC2Plugin, RPC2PluginRef};
use serde_json::{json, Value};
static mut NOTIFYING: bool = false;
static mut GOT_PERMISSION: bool = false;
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
    PLUGIN_NAME.to_string().into()
}
#[sabi_extern_fn]
pub fn init() {
    let _ = std::fs::create_dir(WORKSPACE_DIR.clone().into_os_string());
}

#[sabi_extern_fn]
pub fn get_event_mask() -> ROption<RVec<RString>> {
    Some(
        EVENT_FILTER
            .iter()
            .map(|x| x.to_string().into_::<RString>())
            .collect(),
    )
    .into()
}

#[sabi_extern_fn]
pub fn handle_message(command: RString, args: RVec<RString>) -> ROption<RVec<u8>> {
    let mut ret_vals: Vec<Value> = vec![];
    let mut newpath: PathBuf = WORKSPACE_DIR.to_path_buf();
    // I just don't want to create an entire way to hold state for this one variable
    // which would also include reworking all of the interfacing, which is unstable
    // by itself anyway
    unsafe {
        if !NOTIFYING && !GOT_PERMISSION {
            NOTIFYING = true;
            GOT_PERMISSION = crate::notify();
            NOTIFYING = false;
        } else {
            return Some(
                json!([
                    Value::Bool(false),
                    Value::String("Waiting on permission.".to_string())
                ])
                .to_string()
                .into_bytes()
                .into(),
            )
            .into();
        }
        if !GOT_PERMISSION {
            return Some(
                json!([
                    Value::Bool(false),
                    Value::String("PERMISSION DENIED.".to_string())
                ])
                .to_string()
                .into_bytes()
                .into(),
            )
            .into();
        }
    }
    if args.len() < 1 {
        ret_vals.extend([
            Value::Bool(false),
            Value::String("missing argument #1: path".to_string()),
        ]);
    } else {
        return Some(
            json!(crate::handle_command(
                command.to_string(),
                args.iter().map(|x| x.to_string()).collect::<Vec<String>>()
            ))
            .to_string()
            .as_bytes()
            .into(),
        )
        .into();
    }
    Some(json!(ret_vals).to_string().as_bytes().into()).into()
}
