use abi_stable::{reexports::SelfOps, traits::IntoReprRust};
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Config, Event, Watcher,
};
use png::{BitDepth, ColorType, Encoder};
use regex::Regex;
use rpc2_interface::RPC2PluginRef;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Error},
    path::Path,
    time::Duration,
};
#[derive(Default)]
pub struct RPC2Server {
    content_dir: String,
    plugins: Vec<RPC2PluginRef>,
    monitered_files: Vec<String>,
    checked_lines: HashMap<String, usize>,
    needs_ack: Vec<String>,
}
impl RPC2Server {
    pub fn load_plugin(mut self, plugin: RPC2PluginRef) {
        self.plugins.push(plugin);
    }
    pub fn handle_command(&mut self, command: String, args: Vec<String>) {
        let content_path = self.content_dir.clone() + "/rpc2/" + command.as_str();
        if command == "__ACK" {
            let found = self.needs_ack.iter().position(|x| *x == command);
            if found.is_some() {
                self.needs_ack.remove(found.unwrap());
                write_data_as_png(content_path, vec![0])
                    .expect("writing to succeed (especially with __ACK)");
            }
            return;
        }
        println!(
            "handle_command {}({})",
            command,
            args.iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<String>>()
                .join(",")
        );
        let mut data: Vec<u8> = vec![]; // the difference is that this will always be a multiple of 4
        let mut handled: bool = false;
        for plugin in &self.plugins {
            if plugin.get_event_mask()()
                .into_rust()
                .is_none_or(|x| x.iter().find(|x| **x == command).is_some())
            {
                let result = plugin.handle_message()(
                    command.clone().into_(),
                    args.iter().map(|x| x.clone().into_()).collect(),
                );
                if result.is_some() {
                    let mut it = result.unwrap().into_rust();
                    loop {
                        let r = it.pop();
                        if r.is_none() {
                            break;
                        }
                        let g = it.pop().unwrap_or(0);
                        let b = it.pop().unwrap_or(0);
                        let a = it.pop().unwrap_or(0);
                        data.extend_from_slice(&[r.unwrap(), g, b, a]);
                    }
                    handled = true;
                    break;
                }
            }
        }
        if !handled {
            data.extend(
                json!(vec![
                    Value::Bool(false),
                    Value::String("No such function/function failed to notify".to_string())
                ])
                .to_string()
                .as_bytes(),
            )
        }
        if data.len() == 0 {
            data.extend(json!(vec![Value::Bool(true)]).to_string().as_bytes());
        };
        write_data_as_png(content_path, data)
            .expect("content/rpc2 directory to be present, and for writing to succeed");
    }
    fn check_lines(&mut self, p: String) -> Result<(), std::io::Error> {
        // ex: 2024-12-10T02:03:25.759Z,1.759155,ad1d2440,6 [FLog::Output]
        let cmd_output_regex: Regex = Regex::new(r"^(?:(?:[1-9]\d{3})-(?:0[1-9]|1[0-2])-(?:0[1-9]|[1-2]\d|3[0-1])T(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9])(?:\.\d{3})?Z,(?:[0-9]*[.]?[0-9]*),[[:xdigit:]]+)(?:,[0-9])? \[FLog::Output\] RPC2:(.+)$").unwrap();
        if !self.checked_lines.contains_key(&p) {
            let _ = self.checked_lines.insert(p.clone(), 0);
        }
        let str = std::fs::read_to_string(p.clone())?;
        let iterr = str.lines();
        let count = iterr.clone().count();
        for data in iterr
            .skip(self.checked_lines.insert(p, count).unwrap())
            .filter_map(|x| cmd_output_regex.captures(x))
        {
            let json = data.get(1).ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "error with captures",
            ))?;
            // println!("got data {:?}", json);
            match serde_json::from_str(json.as_str()) {
                Ok(a) => {
                    let b: Vec<&str> = a;
                    if b.first().is_some() {
                        self.handle_command(
                            b.first().unwrap().to_string(),
                            b.iter().skip(1).map(|x| x.to_string()).collect(),
                        );
                    }
                }
                Err(e) => {
                    println!("json parse error (client problem) {:?}",e);
                }
            }
        }
        Ok(())
    }
    pub async fn listen(mut self, log_dir: &Path) -> Result<(), Error> {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .configure(Config::default().with_poll_interval(Duration::from_millis(200)))
            .unwrap();
        watcher
            .watch(log_dir, notify::RecursiveMode::NonRecursive)
            .unwrap();
        for res in rx {
            match res {
                Ok(ev) => match ev.kind {
                    notify::EventKind::Create(CreateKind::File) => {
                        self.monitered_files
                            .push(ev.paths.first().unwrap().to_str().unwrap().to_string());
                    }
                    notify::EventKind::Modify(
                        ModifyKind::Metadata(_) | ModifyKind::Data(_) | ModifyKind::Other,
                    ) => {
                        let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                        if self.monitered_files.iter().find(|x| **x == path).is_some() {
                            println!("check lines {:?}",   self.check_lines(path));
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::Any)
                    | notify::EventKind::Remove(RemoveKind::File) => {
                        let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                        if self.monitered_files.iter().find(|x| **x == path).is_some() {
                            self.monitered_files.remove(
                                self.monitered_files
                                    .iter()
                                    .position(|x| **x == path)
                                    .unwrap(),
                            );
                        }
                    }
                    _ => {}
                },
                Err(e) => println!("error {:?}", e),
            }
        }
        #[allow(unreachable_code)]
        Ok(()) // never (!) is experimental
    }
}
pub fn new_server(content_dir: String, plugins: Option<Vec<RPC2PluginRef>>) -> RPC2Server {
    RPC2Server {
        content_dir,
        plugins: plugins.unwrap_or(Vec::new()),
        ..Default::default()
    }
}
pub fn write_data_as_png(path: String, data: Vec<u8>) -> Result<(), Error> {
    let mut encoder = Encoder::new(
        BufWriter::new(File::create(path)?),
        u32::min(1024, (data.len() / 4) as u32),
        u32::min(
            1024,
            u32::max(1, ((data.len() / 4 / 1024) as f32).floor() as u32),
        ), // should work
    );
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&data)?;
    Ok(())
}
