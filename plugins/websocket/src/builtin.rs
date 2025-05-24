use base64::{Engine, prelude::BASE64_STANDARD};
use rpc2_interface::builtin::RPC2BuiltinPlugin;
use serde_json::{Map, Value, json};
use uuid::Uuid;
use websocket::{
    ClientBuilder, Message, stream::sync::NetworkStream, sync::Client, ws::dataframe::DataFrame,
};

#[derive(Default)]
struct WebSocketSession {
    id: Uuid,
    client: Option<Client<Box<dyn NetworkStream + Send>>>,
}
#[derive(Default)]
struct WebSocketManager {
    sessions: Vec<WebSocketSession>,
}

impl RPC2BuiltinPlugin for WebSocketManager {
    fn get_name(&self) -> &'static str {
        crate::PLUGIN_NAME
    }
    fn get_filter(&self) -> &'static [&'static str] {
        &crate::EVENT_FILTER
    }
    fn handle_message(&mut self, cmd: String, args: Vec<String>) -> Option<Vec<u8>> {
        match cmd.as_str() {
            "websocket_open" => {
                Some(
                    if let Some(url) = args.first() {
                        ClientBuilder::new(url)
                            .map_err(|err| err.to_string()) // Result<ClientBuilder, String>
                            .and_then(|mut x| x.connect(None).map_err(|err| err.to_string())) // Result<Client, String>
                            .map(|cl| {
                                let id: Uuid = Uuid::new_v4();
                                // Uuid::parse_str("45f00bb2-3b8e-4164-82f0-b73d60322642")
                                //     .unwrap();
                                self.sessions.push(WebSocketSession {
                                    id,
                                    client: Some(cl),
                                });
                                json!(vec![Value::Bool(true), Value::String(id.to_string())])
                            }) // Result<Json, String>
                            .unwrap_or_else(|x| {
                                json!(vec![Value::Bool(false), Value::String(x.to_string())])
                            }) // Json
                    } else {
                        json!(vec![
                            Value::Bool(false),
                            Value::String("Missing argument #1: URL".to_string())
                        ])
                    }
                    .to_string()
                    .as_bytes()
                    .to_vec(),
                )
            }
            "websocket_close" | "websocket_send" => Some(
                if let Some(id) = args.first() {
                    if let Some(session) =
                        self.sessions.iter_mut().find(|x| x.id.to_string() == *id)
                    {
                        match cmd.as_str() {
                            "websocket_close" => {
                                json!(vec![
                                    Value::Bool(true),
                                    Value::String("Closed.".to_string())
                                ])
                            }
                            "websocket_send" => {
                                if let Some(gooddata) =
                                    args.get(1).map(|d| BASE64_STANDARD.decode(d))
                                {
                                    if let Ok(data) = gooddata {
                                        // apparently this is an anti pattern to be solved
                                        // by try/catch and ControlFlow::Break
                                        if let Some(s) = session.client.as_mut().and_then(|x| {
                                            x.send_message(&Message::binary(data))
                                                .map_err(|x| x.to_string())
                                                .err()
                                        }) {
                                            json!(vec![
                                                Value::Bool(false),
                                                Value::String(s.to_string())
                                            ])
                                        } else {
                                            json!(vec![Value::Bool(true)])
                                        }
                                    } else {
                                        json!(vec![
                                            Value::Bool(false),
                                            Value::String("Bad Base64 data.".to_string())
                                        ])
                                    }
                                } else {
                                    json!(vec![
                                        Value::Bool(false),
                                        Value::String("Missing argument #2: data".to_string())
                                    ])
                                }
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        json!(vec![
                            Value::Bool(false),
                            Value::String("No such session".to_string())
                        ])
                    }
                } else {
                    json!(vec![
                        Value::Bool(false),
                        Value::String("Missing argument #1: Session ID".to_string())
                    ])
                }
                .to_string()
                .as_bytes()
                .to_vec(),
            ),
            "websocket_poll" => Some(
                if !args.is_empty() {
                    let res = {
                        let cl: Vec<&mut WebSocketSession> = self
                            .sessions
                            .iter_mut()
                            .filter(|x| args.contains(&x.id.to_string()))
                            .collect();
                        if cl.len() != args.len() {
                            Err("No such session.".to_string())
                        } else {
                            Ok(cl)
                        }
                    };
                    if let Ok(mut cl) = res {
                        println!("clients: {}", cl.len());
                        let mut results: Map<String, Value> = Map::new();
                        for client in cl.iter_mut() {
                            let mut vals = Vec::new();
                            let ccl = client.client.take();
                            if let Some(mut clie) = ccl {
                                clie.set_nonblocking(true).unwrap(); // - causes errors cause it blocks
                                loop {
                                    let msg = clie.recv_message();
                                    println!("message: {:?}", msg);
                                    if let Ok(message) = msg {
                                        vals.push(Value::Array(vec![
                                            Value::Array(vec![
                                                Value::Bool(message.is_data()),
                                                Value::Bool(message.is_ping()),
                                                Value::Bool(message.is_pong()),
                                                Value::Bool(message.is_close()),
                                                Value::Bool(message.is_last()),
                                            ]),
                                            Value::String(
                                                BASE64_STANDARD.encode(message.take_payload()),
                                            ),
                                        ]))
                                    } else if let Err(
                                        websocket::WebSocketError::NoDataAvailable
                                        | websocket::WebSocketError::IoError(_),
                                    ) = msg
                                    {
                                        break;
                                    }
                                }
                                let _ = client.client.insert(clie);
                            }
                            results.insert(client.id.to_string(), Value::Array(vals));
                        }
                        json!(vec![Value::Bool(true), Value::Object(results)])
                    } else {
                        json!(vec![
                            Value::Bool(false),
                            Value::String(res.err().unwrap().to_string())
                        ])
                    }
                } else {
                    let empty: Vec<Value> = vec![Value::Bool(true), Value::Array(Vec::new())];
                    json!(empty)
                }
                .to_string()
                .as_bytes()
                .to_vec(),
            ),
            _ => None,
        }
    }

    fn cleanup(&mut self) {
        for sesh in self.sessions.iter() {
            if let Some(client) = &sesh.client {
                let _ = client.shutdown();
            }
        }
        self.sessions.clear();
    }
}
pub fn builtin_create() -> impl RPC2BuiltinPlugin {
    WebSocketManager::default()
}
