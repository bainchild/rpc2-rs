use base64::{Engine, prelude::BASE64_STANDARD};
use native_dialog::{DialogBuilder, MessageLevel};
use serde_json::Value;
use std::{
    path::{Component, PathBuf},
    str::FromStr,
    sync::LazyLock,
};
pub static WORKSPACE_DIR: LazyLock<PathBuf> =
    std::sync::LazyLock::new(|| dirs::data_local_dir().unwrap().join("rpc2_workspacefs/"));
pub(crate) static PLUGIN_NAME: &str = "workspacefs";
pub(crate) static EVENT_FILTER: [&str; 8] = [
    "writefile",
    "readfile",
    // "appendfile",
    "listfiles",
    "isfile",
    "isfolder",
    "makefolder",
    "delfolder",
    "delfile",
];
pub(crate) fn notify() -> bool {
    DialogBuilder::message()
        .set_level(MessageLevel::Info)
        .set_title("[rpc2] Allow access?")
        .set_text("Do you want to allow the roblox game to access your workspace folder?")
        .confirm()
        .show()
        .unwrap()
}
// todo: is this sound?
pub(crate) fn chroot(p: String) -> Result<String, ()> {
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
pub(crate) fn handle_command(command: String, args: Vec<String>) -> Vec<serde_json::Value> {
    let mut newpath: PathBuf = PathBuf::new();
    let mut ret_vals: Vec<serde_json::Value> = Vec::new();
    let mut dont = false;
    match chroot(args.first().unwrap().to_string()) {
        Ok(s) => newpath.push(s),
        Err(_) => {
            ret_vals.extend([
                Value::Bool(false),
                Value::String("error decoding argument #1".to_string()),
            ]);
            dont = true;
        }
    }
    let mut newer = WORKSPACE_DIR.as_path().to_path_buf();
    newer.push(newpath);
    println!(
        "path for workspacefs: {:?} ({})",
        newer.clone().into_os_string(),
        dont
    );
    if !dont {
        match command.to_string().as_str() {
            "writefile" => {
                if args.len() < 2 {
                    ret_vals.extend([
                        Value::Bool(false),
                        Value::String("missing argument #2: data".to_string()),
                    ]);
                } else {
                    match std::fs::write(
                        newer.into_os_string(),
                        BASE64_STANDARD.decode(args.get(1).unwrap()).unwrap(),
                    ) {
                        Ok(_) => ret_vals.push(Value::Bool(true)),
                        Err(_) => ret_vals.extend([
                            Value::Bool(false),
                            Value::String("failed to write file".to_string()),
                        ]),
                    }
                }
            }
            "readfile" => match std::fs::read(newer.into_os_string()) {
                Ok(s) => ret_vals.extend([
                    Value::Bool(true),
                    Value::String(
                        BASE64_STANDARD.encode(
                            s.iter()
                                .filter_map(|x| char::from_u32((*x).into()).map(|x| x.to_string()))
                                .collect::<Vec<String>>()
                                .concat(),
                        ),
                    ),
                ]),
                Err(_) => ret_vals.extend([
                    Value::Bool(false),
                    Value::String("failed to read file".to_string()),
                ]),
            },
            "listfiles" => match std::fs::read_dir(newer.into_os_string().clone()) {
                Ok(read) => ret_vals.extend([
                    Value::Bool(true),
                    Value::Array(
                        read.filter_map(|x| {
                            x.map_or(None, |y| {
                                y.file_name().into_string().map_or(None, |z| {
                                    let mut new2 = PathBuf::new();
                                    new2.push(
                                        chroot(args.first().unwrap().to_string()).unwrap() + "/",
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
                        .map(Value::String)
                        .collect(),
                    ),
                ]),
                Err(_) => ret_vals.extend([
                    Value::Bool(false),
                    Value::String("failed to list directory".to_string()),
                ]),
            },
            "isfile" | "isfolder" => match std::fs::metadata(newer.into_os_string()) {
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
            "makefolder" => match std::fs::create_dir(newer.into_os_string()) {
                Ok(_) => ret_vals.extend([Value::Bool(true)]),
                Err(_) => ret_vals.extend([
                    Value::Bool(false),
                    Value::String("failed to create directory".to_string()),
                ]),
            },
            "delfolder" => match std::fs::remove_dir_all(newer.into_os_string()) {
                Ok(_) => ret_vals.extend([Value::Bool(true)]),
                Err(_) => ret_vals.extend([
                    Value::Bool(false),
                    Value::String("failed to delete directory".to_string()),
                ]),
            },
            "delfile" => match std::fs::remove_file(newer.into_os_string()) {
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
    };
    ret_vals
}
#[cfg(feature = "sabi")]
mod sabi;
#[cfg(feature = "sabi")]
pub use crate::sabi::*;
mod builtin;
pub use crate::builtin::builtin_create;
