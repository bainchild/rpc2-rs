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
    #[clap(value_parser = clap::value_parser!(ClioPath).exists().is_dir())]
    log_dir: ClioPath,
    #[clap(value_parser = clap::value_parser!(ClioPath).exists().is_dir())]
    content_dir: ClioPath,
}
#[tokio::main]
async fn main() {
    let arg = Args::parse();
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
    let server = new_server(
        arg.content_dir.path().to_str().unwrap().to_string(),
        #[cfg(feature = "sabi")]
        Some(pluhs),
        Some(vec![
            Box::new(rpc2_plugin_example::builtin_create()),
            Box::new(rpc2_plugin_workspacefs::builtin_create()),
            Box::new(rpc2_plugin_websocket::builtin_create()),
        ]),
    );
    server.listen(arg.log_dir.path()).await.unwrap();
}
