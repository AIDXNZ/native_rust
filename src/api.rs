// This is the entry point of your Rust library.
// When adding new code to your project, note that only items used
// here will be transformed to their Dart equivalents.

// A plain enum without any fields. This is similar to Dart- or C-style enums.
// flutter_rust_bridge is capable of generating code for enums with fields
// (@freezed classes in Dart and tagged unions in C).
pub enum Platform {
    Unknown,
    Android,
    Ios,
    Windows,
    Unix,
    MacIntel,
    MacApple,
    Wasm,
}

// A function definition in Rust. Similar to Dart, the return type must always be named
// and is never inferred.
pub fn platform() -> Platform {
    // This is a macro, a special expression that expands into code. In Rust, all macros
    // end with an exclamation mark and can be invoked with all kinds of brackets (parentheses,
    // brackets and curly braces). However, certain conventions exist, for example the
    // vector macro is almost always invoked as vec![..].
    //
    // The cfg!() macro returns a boolean value based on the current compiler configuration.
    // When attached to expressions (#[cfg(..)] form), they show or hide the expression at compile time.
    // Here, however, they evaluate to runtime values, which may or may not be optimized out
    // by the compiler. A variety of configurations are demonstrated here which cover most of
    // the modern oeprating systems. Try running the Flutter application on different machines
    // and see if it matches your expected OS.
    //
    // Furthermore, in Rust, the last expression in a function is the return value and does
    // not have the trailing semicolon. This entire if-else chain forms a single expression.
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "android") {
        Platform::Android
    } else if cfg!(target_os = "ios") {
        Platform::Ios
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Platform::MacApple
    } else if cfg!(target_os = "macos") {
        Platform::MacIntel
    } else if cfg!(target_family = "wasm") {
        Platform::Wasm
    } else if cfg!(unix) {
        Platform::Unix
    } else {
        Platform::Unknown
    }
}

// The convention for Rust identifiers is the snake_case,
// and they are automatically converted to camelCase on the Dart side.
pub fn rust_release_mode() -> bool {
    cfg!(not(debug_assertions))
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    addrs: Vec<String>,
}


use std::{fmt::Error, thread::{self, spawn}, net::TcpListener};

use flutter_rust_bridge::{StreamSink, support::lazy_static};
use futures::{executor::block_on, select, StreamExt};
use libp2p::{core::{transport::Boxed, upgrade::Version, muxing::StreamMuxerBox}, identity, PeerId, yamux::YamuxConfig, noise, tcp::async_io, Transport, Swarm, identify, swarm::{SwarmEvent, handler, keep_alive, NetworkBehaviour}, Multiaddr};
use serde::{Serialize, Deserialize};
use tungstenite::{accept, Message};
use serde_json::{Result, Value};
use cbor::{Decoder, Encoder};
use rustc_serialize::json::{Json, ToJson};

fn build_transport(
    key_pair: identity::Keypair,
    
) -> Boxed<(PeerId, StreamMuxerBox)>{
    let base_transport = async_io::Transport::new(libp2p_tcp::Config::default().nodelay(true));
    let noise_config = noise::NoiseAuthenticated::xx(&key_pair).unwrap();
    let yamux_config = YamuxConfig::default();

    base_transport
    .upgrade(Version::V1)
    .authenticate(noise_config)
    .multiplex(yamux_config)
    .boxed()
}


pub fn start() {
    let handler = thread::spawn(|| {
        block_on(async_start());
    });
    handler.join().unwrap();
}

async fn async_start() {

    let mut local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.clone().public());
    let transport = build_transport(local_key.clone());

    let mut swarm = {
        Swarm::with_threadpool_executor(
            transport,
            identify::Behaviour::new(identify::Config::new("/ipfs/id/1.0.0".to_string(), local_key.clone().public())),
            local_peer_id
        )
    };
    let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap());


    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    
    loop {
        if let Some(stream) =  server.incoming().next() {
                let mut websocket = accept(stream.unwrap()).unwrap();
                loop {
                    match websocket.read_message().unwrap() {
                        msg @ Message::Text(_) | msg @ Message::Binary(_) => {
                            let mut decoder = Decoder::from_bytes(msg.into_data());
                            let cbor = decoder.items().next().unwrap().unwrap();
                            let p: Value = serde_json::from_str(cbor.to_json().to_string().as_str()).unwrap();
                            
                            if p["local_peer_id"].as_bool().is_some() {
                                let pid = local_peer_id.clone();
                                websocket.write_message(tungstenite::Message::Text(pid.to_string())).unwrap();
                            } else if p["dial_addrs"].as_array().is_some() {
                                let addrs = p["dial_addrs"].as_array().unwrap();
                                for addr in addrs.into_iter() {
                                    let multi_addr: Multiaddr = addr.as_str().unwrap().parse().unwrap();
                                    swarm.dial(multi_addr).unwrap();
                                }
                            }
                            
                        }
                        Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {}
                    }
                }
        }
        select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::ConnectionEstablished {
                        peer_id,
                        endpoint,
                        ..
                    } => {
                        println!("{:?},", endpoint.get_remote_address());
                    },
                    SwarmEvent::OutgoingConnectionError {peer_id, error} => {
                            println!("Connection Error: {:?}", error);
                        }
                    _ => (),
                }
            }
            
        }
    }
}

pub fn is_valid_multiaddr(s: String) -> bool {
    let is_ok = s.parse::<Multiaddr>().is_ok();
    return is_ok;
}

