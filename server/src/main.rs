use clap::Parser;
use clio::ClioPath;
use rpc2_interface::{load_root_module_from_file, RPC2PluginRef};
use rpc2_server::new_server;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
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
    let mut plugins_loaded: Vec<String> = Vec::new();
    let pluhs: Vec<RPC2PluginRef> = arg
        .plugin
        .iter()
        .map(|x| load_root_module_from_file(x).unwrap())
        .inspect(|x| {
            plugins_loaded.push(x.get_name()().to_string());
            x.init()();
        })
        .collect();
    println!(
        "Creating server with plugins: {}",
        plugins_loaded.join(", ")
    );
    std::mem::drop(plugins_loaded);
    let server = new_server(
        arg.content_dir.path().to_str().unwrap().to_string(),
        Some(pluhs),
    );
    server.listen(arg.log_dir.path()).await.unwrap();
}
