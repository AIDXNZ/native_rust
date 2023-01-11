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

use std::{
    fmt::Error,
    net::TcpListener,
    thread::{self, spawn},
};

use anyhow::Result;
use cbor::{Decoder, Encoder};
use flutter_rust_bridge::{support::lazy_static, StreamSink};
use futures::{executor::block_on, select, StreamExt};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade::Version},
    identify, identity,
    mdns::{self, MdnsEvent},
    noise,
    swarm::{handler, keep_alive, NetworkBehaviour, SwarmEvent},
    tcp::async_io,
    yamux::YamuxConfig,
    Multiaddr, PeerId, Swarm, Transport,
};
use rustc_serialize::json::{Json, ToJson};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tungstenite::{accept, Message};
use std::sync::mpsc;

fn build_transport(key_pair: identity::Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    let base_transport = async_io::Transport::new(libp2p_tcp::Config::default().nodelay(true));
    let noise_config = noise::NoiseAuthenticated::xx(&key_pair).unwrap();
    let yamux_config = YamuxConfig::default();

    base_transport
        .upgrade(Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux_config)
        .boxed()
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
struct ComposedBehaviour {
    identify: identify::Behaviour,
    mdns: mdns::async_io::Behaviour,
}

#[derive(Debug)]
enum Event {
    Mdns(mdns::Event),
    Identify(identify::Event),
}

impl From<mdns::Event> for Event {
    fn from(event: mdns::Event) -> Self {
        Event::Mdns(event)
    }
}

impl From<identify::Event> for Event {
    fn from(event: identify::Event) -> Self {
        Event::Identify(event)
    }
}


pub fn start() {
    thread::spawn(|| {
        block_on(async_start()).expect("Couldn't start async start");
    });
}

async fn async_start() -> Result<()> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.clone().public());
    let transport = build_transport(local_key.clone());

    let mut swarm = {
        Swarm::with_threadpool_executor(
            transport,
            identify::Behaviour::new(identify::Config::new(
                "/ipfs/id/1.0.0".to_string(),
                local_key.clone().public(),
            )),
            local_peer_id,
        )
    };
    let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?);

    // This is the Rust RPC server
    let server = TcpListener::bind("127.0.0.1:9002")?;

    loop {
        for stream in server.incoming() {
            let mut websocket = accept(stream?)?;
            loop {
                let msg = websocket.read_message()?;

                if msg.is_binary() || msg.is_text() {
                    let pid = local_peer_id.clone();
                    websocket.write_message(tungstenite::Message::Text(pid.to_string()))?;
                    let mut d = Decoder::from_bytes(msg.into_data());
                    let items = d.items().next().unwrap()?.to_json();

                    match items["local_peer_id"].as_string() {
                        Some(_) => {
                            let pid = local_peer_id.clone();
                            websocket.write_message(tungstenite::Message::Text(pid.to_string()))?;
                            websocket.close(None)?;
                        }
                        None => (),
                    }

                    websocket.close(None)?;
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





/////////////////////////////////////////
/// ////////////////////////////////////
/// ///////////////////////////////////
/// ///////////////////////////////////
#[cfg(test)]
mod tests {
    use std::thread;

    use futures::executor::block_on;

    use super::async_start;

    #[test]
    fn it_works() {
        block_on(async {
            async_start().await.unwrap();
        })
    }
}