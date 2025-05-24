use std::fmt::Debug;

use clap::Parser;
use clio::ClioPath;
#[cfg(feature = "sabi")]
use rpc2_interface::sabi::{RPC2PluginRef, load_root_module_from_file};
use rpc2_server::new_server;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[cfg(feature = "sabi")]
    #[arg(short, long)]
    plugin: Vec<ClioPath>,
    #[arg(short, long)]
    cmd: Vec<String>,
    #[arg(short, long)]
    filecmd: Option<ClioPath>,
}
fn main() {
    let arg = Args::parse();
    let mut list: Vec<(String, Vec<String>)> = Vec::new();
    for a in arg.cmd {
        let mut pat = (a + ",")
            .split_terminator(",")
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        pat.reverse();
        let cmd = pat.pop().expect("command should have a command to it");
        pat.reverse();
        list.push((cmd, pat));
    }
    if arg.filecmd.is_some() {
        let content = arg.filecmd.unwrap().read_all().unwrap().to_string();
        for a in content.split_terminator("\n") {
            let mut pat = (a.to_string() + ",")
                .split_terminator(",")
                .map(|x| x.to_string())
                .collect::<Vec<String>>();
            pat.reverse();
            let cmd = pat.pop().expect("command should have a command to it");
            pat.reverse();
            list.push((cmd, pat));
        }
    }
    #[cfg(feature = "sabi")]
    let mut plugins_loaded: Vec<String> = Vec::new();
    #[cfg(feature = "sabi")]
    let pluhs: Vec<RPC2PluginRef> = arg
        .plugin
        .iter()
        .map(|x| load_root_module_from_file(x).unwrap())
        .inspect(|x| {
            plugins_loaded.push(x.get_name()().to_string());
            x.init()();
        })
        .collect();
    #[cfg(feature = "sabi")]
    {
        println!(
            "Creating server with plugins: {}",
            plugins_loaded.join(", ")
        );
        std::mem::drop(plugins_loaded);
    }
    let mut server = new_server(
        "./fake_logs/".to_string(),
        #[cfg(feature = "sabi")]
        Some(pluhs),
        Some(vec![
            Box::new(rpc2_plugin_example::builtin_create()),
            Box::new(rpc2_plugin_workspacefs::builtin_create()),
            Box::new(rpc2_plugin_websocket::builtin_create()),
        ]),
    );
    for a in list {
        let res = server.handle_command("plugindbg".to_string(), a.0, a.1);
        if res.is_some() {
            let chars = res
                .unwrap()
                .1
                .iter()
                .map(|x| char::from_u32((*x).into()).unwrap().to_string())
                .collect::<Vec<String>>();
            println!("returned {:?}", chars.concat());
        } else {
            println!("no return");
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
