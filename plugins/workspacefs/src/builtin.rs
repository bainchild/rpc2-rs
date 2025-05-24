use crate::{EVENT_FILTER, PLUGIN_NAME, WORKSPACE_DIR, handle_command, notify};
use rpc2_interface::builtin::RPC2BuiltinPlugin;
use serde_json::{Value, json};
#[derive(Default)]
struct WorkspaceFS {
    notifying: bool,
    permission: bool,
    notified: bool,
}
impl RPC2BuiltinPlugin for WorkspaceFS {
    fn get_name(&self) -> &'static str {
        PLUGIN_NAME
    }
    fn get_filter(&self) -> &'static [&'static str] {
        &EVENT_FILTER
    }
    fn handle_message(&mut self, cmd: String, args: Vec<String>) -> Option<Vec<u8>> {
        println!("workspacefs {} {:?}", cmd, args);
        if self.notifying {
            return Some(
                json!(vec![
                    Value::Bool(false),
                    Value::String("Waiting on permission.".to_string())
                ])
                .to_string()
                .into_bytes(),
            );
        }
        if !self.notified {
            self.notifying = true;
            self.permission = notify();
            self.notified = true;
            self.notifying = false;
        }
        if !self.permission {
            return Some(
                json!(vec![
                    Value::Bool(false),
                    Value::String("Permission denied.".to_string())
                ])
                .to_string()
                .into_bytes(),
            );
        }
        // let path = args.get(0);
        // if path.is_none() {
        //     return Some(
        //         json!(vec![
        //             Value::Bool(false),
        //             Value::String("Missing argument #1: Path".to_string())
        //         ])
        //         .to_string()
        //         .as_bytes()
        //         .to_vec(),
        //     );
        // };
        // let newpath = chroot(path.unwrap().clone()).ok();
        // if newpath.is_none() {
        //     return Some(
        //         json!(vec![
        //             Value::Bool(false),
        //             Value::String("Permission denied. (path)".to_string())
        //         ])
        //         .to_string()
        //         .as_bytes()
        //         .to_vec(),
        //     );
        // }
        Some(
            json!(self::handle_command(cmd, args))
                .to_string()
                .as_bytes()
                .to_vec(),
        )
    }
}
pub fn builtin_create() -> impl RPC2BuiltinPlugin {
    let _ = std::fs::create_dir(WORKSPACE_DIR.clone().into_os_string());
    WorkspaceFS::default()
}
