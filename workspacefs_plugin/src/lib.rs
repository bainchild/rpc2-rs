use std::{
    path::{Component, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    reexports::SelfOps,
    sabi_extern_fn,
    std_types::{ROption, RString, RVec},
};
use rpc2_interface::{RPC2Plugin, RPC2PluginRef};
use serde_json::{json, Value};
static WORKSPACE_DIR: LazyLock<PathBuf> =
    std::sync::LazyLock::new(|| dirs::data_local_dir().unwrap().join("rpc2_workspacefs/"));
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
    "workspacefs".into()
}
#[sabi_extern_fn]
pub fn init() {
    let _ = std::fs::create_dir(WORKSPACE_DIR.clone().into_os_string());
}

#[sabi_extern_fn]
pub fn get_event_mask() -> ROption<RVec<RString>> {
    Some(
        vec![
            "writefile",
            "readfile",
            // "appendfile",
            "listfiles",
            "isfile",
            "isfolder",
            "makefolder",
            "delfolder",
            "delfile",
        ]
        .iter()
        .map(|x| x.to_string().into_::<RString>())
        .collect(),
    )
    .into()
}

// todo: is this sound?
fn chroot(p: String) -> Result<String, ()> {
    let mut pat = PathBuf::from_str(p.as_str()).or(Err(()))?;
    // .canonicalize()
    // .or(Err(()))?;
    if pat.is_relative() {
        loop {
            if pat
                .components()
                .take(1)
                .collect::<Vec<Component>>()
                .first()
                .is_some_and(|x| x.as_os_str() == "..")
            {
                pat.pop();
            } else {
                break;
            }
        }
        Ok(pat.to_str().unwrap().to_string())
    } else if pat.is_absolute() {
        Ok(pat.file_name().ok_or(())?.to_str().unwrap().to_string())
    } else {
        pat.to_str().ok_or(()).map(|x| x.to_string()) // might be a bad idea
    }
}

#[sabi_extern_fn]
pub fn handle_message(command: RString, args: RVec<RString>) -> ROption<RVec<u8>> {
    let mut ret_vals: Vec<Value> = vec![];
    let mut newpath = WORKSPACE_DIR.clone();
    if args.len() < 1 {
        ret_vals.extend([
            Value::Bool(false),
            Value::String("missing argument #1: path".to_string()),
        ]);
    } else {
        let mut dont = false;
        match chroot(args.get(0).unwrap().to_string()) {
            Ok(s) => newpath.push(s),
            Err(_) => {
                ret_vals.extend([
                    Value::Bool(false),
                    Value::String("error decoding argument #1".to_string()),
                ]);
                dont = true;
            }
        }
        // println!("path for workspacefs: {:?} ({})", newpath.to_str(), dont);
        if !dont {
            match command.to_string().as_str() {
                "writefile" => {
                    if args.len() < 2 {
                        ret_vals.extend([
                            Value::Bool(false),
                            Value::String("missing argument #2: data".to_string()),
                        ]);
                    } else {
                        match std::fs::write(newpath, args.get(1).unwrap()) {
                            Ok(_) => ret_vals.push(Value::Bool(true)),
                            Err(_) => ret_vals.extend([
                                Value::Bool(false),
                                Value::String("failed to write file".to_string()),
                            ]),
                        }
                    }
                }
                "readfile" => match std::fs::read(newpath) {
                    Ok(s) => ret_vals.extend([
                        Value::Bool(true),
                        Value::String(
                            s.iter()
                                .filter_map(|x| char::from_u32((*x).into()).map(|x| x.to_string()))
                                .collect::<Vec<String>>()
                                .concat(),
                        ),
                    ]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to read file".to_string()),
                    ]),
                },
                "listfiles" => match std::fs::read_dir(newpath.clone()) {
                    Ok(read) => ret_vals.extend([
                        Value::Bool(true),
                        Value::Array(
                            read.filter_map(|x| {
                                x.map_or(None, |y| {
                                    y.file_name().into_string().map_or(None, |z| {
                                        let mut new2 = PathBuf::new();
                                        new2.push(
                                            chroot(args.get(0).unwrap().to_string()).unwrap() + "/",
                                        );
                                        new2.push(z);
                                        Some(
                                            new2.to_str()
                                                .expect("both sides of the join to be verified")
                                                .to_string(),
                                        )
                                    })
                                })
                            })
                            .map(|x| Value::String(x))
                            .collect(),
                        ),
                    ]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to list directory".to_string()),
                    ]),
                },
                "isfile" | "isfolder" => match std::fs::metadata(newpath) {
                    Ok(meta) => ret_vals.extend([
                        Value::Bool(true),
                        Value::Bool({
                            if command == "isfile" {
                                meta.is_file()
                            } else {
                                meta.is_dir()
                            }
                        }),
                    ]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to stat file or directory".to_string()),
                    ]),
                },
                "makefolder" => match std::fs::create_dir(newpath) {
                    Ok(_) => ret_vals.extend([Value::Bool(true)]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to create directory".to_string()),
                    ]),
                },
                "delfolder" => match std::fs::remove_dir_all(newpath) {
                    Ok(_) => ret_vals.extend([Value::Bool(true)]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to delete directory".to_string()),
                    ]),
                },
                "delfile" => match std::fs::remove_file(newpath) {
                    Ok(_) => ret_vals.extend([Value::Bool(true)]),
                    Err(_) => ret_vals.extend([
                        Value::Bool(false),
                        Value::String("failed to delete file".to_string()),
                    ]),
                },
                _ => {
                    unreachable!("this shouldn't be reached cause the event mask, hopefully.")
                }
            }
        }
    }
    Some(json!(ret_vals).to_string().as_bytes().into()).into()
}
