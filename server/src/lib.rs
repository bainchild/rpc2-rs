#[cfg(feature = "sabi")]
use abi_stable::{reexports::SelfOps, traits::IntoReprRust};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Local};
use json::object;
use notify::{
    Config, Event, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind},
};
use regex::Regex;
use rpc2_interface::builtin::RPC2BuiltinPlugin;
#[cfg(feature = "sabi")]
use rpc2_interface::sabi::RPC2PluginRef;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::Error,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant, SystemTime},
};
#[derive(Default)]
pub struct RPC2Server {
    content_dir: String,
    #[cfg(feature = "sabi")]
    plugins: Vec<RPC2PluginRef>,
    builtins: Vec<Box<dyn RPC2BuiltinPlugin>>,
    monitered_files: Vec<String>,
    checked_lines: HashMap<String, usize>,
    content_files: Vec<String>,
    // needs_ack: Vec<String>,
    // failed_reads: Vec<(DateTime<Local>, String)>,
    ready: Vec<String>,
    listeners: HashMap<String, Vec<mpsc::Sender<String>>>,
}

impl RPC2Server {
    #[cfg(feature = "sabi")]
    pub fn load_plugin(&mut self, plugin: RPC2PluginRef) {
        self.plugins.push(plugin);
    }
    pub fn get_content_path(&self, cacheprefix: String, command: String) -> String {
        self.content_dir.clone() + "/rpc2/" + cacheprefix.as_str() + command.as_str()
    }
    pub fn cleanup(&mut self) {
        for f in self.content_files.iter() {
            println!("clean {}", f);
            let _ = std::fs::remove_file(f);
        }
        #[cfg(feature = "sabi")]
        for plugin in &self.plugins {
            plugin.cleanup()();
        }
        for builtin in self.builtins.iter_mut() {
            builtin.cleanup();
        }
    }
    pub fn handle_command(
        &mut self,
        cacheprefix: String,
        command: String,
        args: Vec<String>,
    ) -> Option<(String, Vec<u8>)> {
        let content_path = self.get_content_path(cacheprefix.clone(), command.clone());
        if self.listeners.contains_key(&command) {
            let cc = args.concat();
            self.listeners
                .get(&command)
                .unwrap()
                .iter()
                .for_each(|x| x.send(cc.clone()).expect("should be able to send."));
            return None;
        }
        // if command == "__ACK" {
        //     let found = self.needs_ack.iter().position(|x| *x == command);
        //     if let Some(foundn) = found {
        //         self.needs_ack.remove(foundn);
        //         std::fs::remove_file(content_path).expect("should be able to remove the file.");
        //     }
        //     return None;
        /*} else*/
        if command == "__CLEANUP" {
            // for v in self.needs_ack.iter() {
            //     let _ = std::fs::remove_file(v);
            // }
            self.cleanup();
            return None;
        // } else if command == "__GETFAILURE" {
        //     let failed = self.failed_reads.clone();
        //     self.failed_reads.clear();
        //     return Some((
        //         content_path,
        //         json!(vec![
        //             Value::Bool(true),
        //             Value::Array(
        //                 failed
        //                     .iter()
        //                     .map(|x| Value::Array(vec![
        //                         Value::String(x.0.to_string()),
        //                         Value::String(x.1.clone())
        //                     ]))
        //                     .collect()
        //             )
        //         ])
        //         .to_string()
        //         .as_bytes()
        //         .to_vec(),
        //     ));
        } else if command == "__READY" {
            return Some((
                content_path,
                json!(
                    self.ready
                        .clone()
                        .iter()
                        .map(|x| Value::String(x.clone()))
                        .collect::<Vec<Value>>()
                )
                .to_string()
                .as_bytes()
                .to_vec(),
            ));
        } else if command == "__READYACK" {
            self.ready.clear();
            return None;
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
        #[cfg(feature = "sabi")]
        for plugin in &self.plugins {
            if plugin.get_event_mask()()
                .into_rust()
                .is_none_or(|x| x.iter().any(|x| **x == command))
            {
                let result = plugin.handle_message()(
                    command.clone().into_(),
                    args.iter().map(|x| x.clone().into_()).collect(),
                );
                if let Some(res) = result {
                    // self.needs_ack.push(content_path.clone());
                    let it = res.into_rust();
                    if !it.is_empty() {
                        data.extend(it);
                    }
                    handled = true;
                    break;
                }
            }
        }
        if !handled {
            for builtin in self.builtins.iter_mut() {
                if builtin.get_filter().contains(&command.as_str()) {
                    let result = builtin
                        .as_mut()
                        .handle_message(command.clone(), args.clone());
                    if let Some(res) = result {
                        // self.needs_ack.push(content_path.clone());
                        if !res.is_empty() {
                            data.extend(res);
                        }
                        handled = true;
                        break;
                    }
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
        if data.is_empty() {
            data.extend(json!(vec![Value::Bool(true)]).to_string().as_bytes());
        };
        self.ready.push(cacheprefix + command.as_str());
        Some((content_path, data))
    }
    fn check_lines(&mut self, p: String) -> Result<(), std::io::Error> {
        // ex: 2024-12-10T02:03:25.759Z,1.759155,ad1d2440,6 [FLog::Output] (or FLog::Warning)
        let cmd_output_regex: Regex = Regex::new(r"^(?:(?:[1-9]\d{3})-(?:0[1-9]|1[0-2])-(?:0[1-9]|[1-2]\d|3[0-1])T(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9])(?:\.\d{3})?Z,(?:[0-9]*[.]?[0-9]*),[[:xdigit:]]+)(?:,[0-9])? \[FLog::Warning\] RPC2:(.+)$").unwrap();
        // let read_failure_regex: Regex = Regex::new(r"^(?:(?:[1-9]\d{3})-(?:0[1-9]|1[0-2])-(?:0[1-9]|[1-2]\d|3[0-1])T(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9]):(?:[0-4]\d|5[0-9])(?:\.\d{3})?Z,(?:[0-9]*[.]?[0-9]*),[[:xdigit:]]+)(?:,[0-9])? \[FLog::Warning\] Warning: Font family rbxasset://rpc2/(?:.+) failed to load: .+$").unwrap();
        if !self.checked_lines.contains_key(&p) {
            let _ = self.checked_lines.insert(p.clone(), 0);
        }
        let str = std::fs::read_to_string(p.clone())?;
        let iterr = str.lines();
        let count = iterr.clone().count();
        // for data in iterr
        //     .clone()
        //     .skip(*self.checked_lines.get(&p).unwrap())
        //     .filter_map(|x| read_failure_regex.captures(x))
        // {
        //     if let Some(path) = data.get(1) {
        //         self.failed_reads
        //             .push((chrono::Local::now(), path.as_str().to_string()));
        //         let content_path = self.get_content_path("".to_string(), path.as_str().to_string());
        //         if self.needs_ack.contains(&content_path) {
        //             self.needs_ack.remove(
        //                 self.needs_ack
        //                     .iter()
        //                     .enumerate()
        //                     .find(|x| *x.1 == content_path)
        //                     .unwrap()
        //                     .0,
        //             );
        //         }
        //     }
        // }
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
                    if let (Some(cachestr), Some(cmdstr)) = (b.first(), b.get(1)) {
                        // b.first().is_some() && b.get(1).is_some() {
                        // let cachestr = b.first().unwrap().to_string();
                        // let cmdstr = b.get(1).unwrap().to_string();
                        let cachestr = cachestr.to_string();
                        let cmdstr = cmdstr.to_string();
                        let cmd = self.handle_command(
                            cachestr,
                            cmdstr.clone(),
                            b.iter().skip(2).map(|x| x.to_string()).collect(),
                        );
                        if let Some(dat) = cmd {
                            self.content_files.push(dat.0.clone());
                            write_data(dat.0, cmdstr.into(), dat.1).expect(
                                "content/rpc2 directory to be present, and for writing to succeed",
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("json parse error (client problem) {:?}", e);
                }
            }
        }
        Ok(())
    }
    pub async fn listen(mut self, log_dir: &Path) -> Result<(), Error> {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        const POLL_INTERVAL: Duration = Duration::from_millis(100);
        // MAYBE, maybe this will work. so far windows hates watching files.
        let confige = Config::default().with_poll_interval(POLL_INTERVAL);
        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher.configure(confige).unwrap();
        watcher
            .watch(log_dir, notify::RecursiveMode::Recursive)
            .unwrap();
        // not the worst rust code I've written
        #[cfg(windows)]
        {
            let mut prev_lengths: HashMap<String, u64> = HashMap::new();
            loop {
                if let Result::Ok(res) = rx.recv_timeout(POLL_INTERVAL) {
                    match res {
                        Ok(ev) => match ev.kind {
                            notify::EventKind::Create(CreateKind::File | CreateKind::Any) => {
                                println!("listening to new file (windows)...");
                                self.monitered_files
                                    .push(ev.paths.first().unwrap().to_str().unwrap().to_string());
                            }
                            notify::EventKind::Modify(
                                ModifyKind::Metadata(_) | ModifyKind::Data(_) | ModifyKind::Other,
                            ) => {
                                let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                                if self.monitered_files.iter().any(|x| **x == path) {
                                    println!("check lines {:?}", self.check_lines(path));
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::Any | RemoveKind::File) => {
                                let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                                if self.monitered_files.iter().any(|x| **x == path) {
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
                } else {
                    for file in self.monitered_files.clone().iter() {
                        let metaa = std::fs::metadata(file.clone());
                        if let Result::Ok(meta) = metaa {
                            let newlen = meta.len();
                            if prev_lengths.contains_key(file) {
                                if *prev_lengths.get(file).unwrap() != newlen {
                                    /*println!("check lines 2 {:?}", */
                                    let _ = self.check_lines(file.clone()); /*);*/
                                }
                            } else {
                                prev_lengths.insert(file.clone(), newlen);
                                /*println!("check lines 3 {:?}", */
                                let _ = self.check_lines(file.clone()); /*);*/
                            }
                        }
                    }
                }
            }
        }
        #[cfg(not(windows))]
        for res in rx {
            // println!("AAAAAaa {:?}", res);
            match res {
                Ok(ev) => match ev.kind {
                    notify::EventKind::Create(CreateKind::File | CreateKind::Any) => {
                        println!("listening to new file...");
                        self.monitered_files
                            .push(ev.paths.first().unwrap().to_str().unwrap().to_string());
                    }
                    notify::EventKind::Modify(
                        ModifyKind::Metadata(_) | ModifyKind::Data(_) | ModifyKind::Other,
                    ) => {
                        let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                        if self.monitered_files.iter().any(|x| **x == path) {
                            println!("check lines {:?}", self.check_lines(path));
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::Any | RemoveKind::File) => {
                        let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                        if self.monitered_files.iter().any(|x| **x == path) {
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
pub fn new_server(
    content_dir: String,
    #[cfg(feature = "sabi")] plugins: Option<Vec<RPC2PluginRef>>,
    builtins: Option<Vec<Box<dyn RPC2BuiltinPlugin>>>,
) -> RPC2Server {
    RPC2Server {
        content_dir,
        #[cfg(feature = "sabi")]
        plugins: plugins.unwrap_or_default(),
        builtins: builtins.unwrap_or_default(),
        ..Default::default()
    }
}
pub fn write_data(path: String, command: Vec<u8>, data: Vec<u8>) -> Result<(), Error> {
    std::fs::write(
        path,
        object! {
            "name": BASE64_STANDARD.encode(command),
            "faces": [{
                "name": BASE64_STANDARD.encode(&data),
                "weight": 400
            }]
        }
        .dump(),
    )
}
