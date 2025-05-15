use abi_stable::{reexports::SelfOps, traits::IntoReprRust};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use json::object;
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Config, Event, Watcher,
};
use regex::Regex;
use rpc2_interface::RPC2PluginRef;
use serde_json::{json, Value};
use std::{
    collections::HashMap, io::Error, path::Path, pin::Pin, sync::mpsc, task::Poll, time::Duration,
};
#[derive(Default)]
pub struct RPC2Server {
    content_dir: String,
    plugins: Vec<RPC2PluginRef>,
    monitered_files: Vec<String>,
    checked_lines: HashMap<String, usize>,
    needs_ack: Vec<String>,
    listeners: HashMap<String, Vec<mpsc::Sender<String>>>,
}
pub struct RPC2Stream {
    chunk: Vec<char>,
    pending: Vec<String>,
    timeout: Duration,
    mpsc: std::sync::mpsc::Receiver<String>,
}

impl RPC2Server {
    pub fn load_plugin(&mut self, plugin: RPC2PluginRef) {
        self.plugins.push(plugin);
    }
    pub fn handle_command(
        &mut self,
        cacheprefix: String,
        command: String,
        args: Vec<String>,
    ) -> Option<(String, Vec<u8>)> {
        let content_path =
            self.content_dir.clone() + "/rpc2/" + cacheprefix.as_str() + command.as_str();
        if self.listeners.contains_key(&command) {
            let cc = args.concat();
            self.listeners
                .get(&command)
                .unwrap()
                .iter()
                .for_each(|x| x.send(cc.clone()).expect("should be able to send."));
            return None;
        }
        if command == "__ACK" {
            let found = self.needs_ack.iter().position(|x| *x == command);
            if found.is_some() {
                self.needs_ack.remove(found.unwrap());
                std::fs::remove_file(content_path).expect("should be able to remove the file.");
            }
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
                    let it = result.unwrap().into_rust();
                    if it.len() > 0 {
                        data.extend(it);
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
        return Some((content_path, data));
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
                    if b.first().is_some() && b.get(1).is_some() {
                        let cachestr = b.first().unwrap().to_string();
                        let cmdstr = b.get(1).unwrap().to_string();
                        let cmd = self.handle_command(
                            cachestr,
                            cmdstr.clone(),
                            b.iter().skip(2).map(|x| x.to_string()).collect(),
                        );
                        if cmd.is_some() {
                            let dat = cmd.unwrap();
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
        const POLL_INTERVAL: Duration = Duration::from_millis(200);
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
                                println!("listening to new file...");
                                self.monitered_files
                                    .push(ev.paths.first().unwrap().to_str().unwrap().to_string());
                            }
                            notify::EventKind::Modify(
                                ModifyKind::Metadata(_) | ModifyKind::Data(_) | ModifyKind::Other,
                            ) => {
                                let path = ev.paths.first().unwrap().to_str().unwrap().to_string();
                                if self.monitered_files.iter().find(|x| **x == path).is_some() {
                                    println!("check lines {:?}", self.check_lines(path));
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::Any | RemoveKind::File) => {
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
                } else {
                    for file in self.monitered_files.clone().iter() {
                        let metaa = std::fs::metadata(file.clone());
                        if let Result::Ok(meta) = metaa {
                            let newlen = meta.len();
                            if prev_lengths.contains_key(file) {
                                if *prev_lengths.get(file).unwrap() != newlen {
                                    /*println!("check lines 2 {:?}", */
                                    self.check_lines(file.clone()); /*);*/
                                }
                            } else {
                                prev_lengths.insert(file.clone(), newlen);
                                /*println!("check lines 3 {:?}", */
                                self.check_lines(file.clone()); /*);*/
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
                        if self.monitered_files.iter().find(|x| **x == path).is_some() {
                            println!("check lines {:?}", self.check_lines(path));
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::Any | RemoveKind::File) => {
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
    pub fn get_stream(
        &mut self,
        scope: String,
        timeout: Option<Duration>,
    ) -> impl futures_core::Stream<Item = char> {
        if !self.listeners.contains_key(&scope) {
            self.listeners.insert(scope.clone(), Vec::new());
        }
        let (send, recv) = mpsc::channel();
        self.listeners.get_mut(&scope).unwrap().push(send);
        RPC2Stream {
            chunk: Vec::new(),
            pending: Vec::new(),
            timeout: timeout.unwrap_or(Duration::from_secs(3)),
            mpsc: recv,
        }
    }
}
impl futures_core::Stream for RPC2Stream {
    type Item = char;
    fn poll_next<'b>(
        self: Pin<&mut Self>,
        _ctx: &mut std::task::Context<'b>,
    ) -> Poll<Option<char>> {
        if self.chunk.is_empty() {
            let mutt = self.get_mut();
            if mutt.pending.is_empty() {
                let ok = mutt.mpsc.recv_timeout(mutt.timeout).ok();
                if ok.is_some() {
                    mutt.chunk = ok
                        .unwrap()
                        .as_bytes()
                        .iter()
                        .filter_map(|x| char::from_u32((*x).into()))
                        .collect();
                    loop {
                        let r = mutt.mpsc.recv();
                        if r.is_ok() {
                            mutt.pending.push(r.unwrap());
                        } else {
                            break;
                        }
                    }
                }
                return Poll::Ready(mutt.chunk.pop());
            } else {
                mutt.chunk = mutt
                    .pending
                    .pop()
                    .unwrap()
                    .as_bytes()
                    .iter()
                    .filter_map(|x| char::from_u32((*x).into()))
                    .collect();
            }
            return Poll::Pending;
        } else {
            return Poll::Ready(Some(self.get_mut().chunk.pop().unwrap()));
        }
    }
}
pub fn new_server(content_dir: String, plugins: Option<Vec<RPC2PluginRef>>) -> RPC2Server {
    RPC2Server {
        content_dir,
        plugins: plugins.unwrap_or(Vec::new()),
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
