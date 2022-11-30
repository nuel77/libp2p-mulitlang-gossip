mod utils;

use std::time::Duration;
use crate::utils::set_panic_hook;
use gloo::console::log;
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use libp2p::futures::channel::mpsc;
use libp2p::futures::channel::mpsc::UnboundedReceiver;
use libp2p::wasm_ext::ffi::{websocket_transport};
use libp2p::wasm_ext::ExtTransport;
use libp2p::{identity, noise, Multiaddr, PeerId, gossipsub, Transport, Swarm, NetworkBehaviour};
use libp2p::core::upgrade::Version::{V1, V1Lazy};
use futures::{prelude::*, select};
use gloo::timers::future::sleep;
use js_sys::Math::log;
use libp2p::noise::NoiseConfig;
use libp2p::swarm::{ SwarmEvent};
use libp2p::yamux::YamuxConfig;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlFormElement};
// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
    utils::log_meta();
    let peer_consumer = attach_add_peer();
    let message_consumer = attach_add_message_box();
    start_chat(peer_consumer, message_consumer);
}

fn attach_add_peer() -> UnboundedReceiver<Multiaddr> {
    let window = web_sys::window().expect("window does not exist");
    let document = window.document().expect("document does not exist");
    let body = document.body().expect("body does not exist");

    let input = document
        .get_element_by_id("input-multiaddr")
        .expect("peer input should be defined")
        .dyn_into::<HtmlFormElement>()
        .expect("elemnt is a form");

    let (mut peer_producer, peer_consumer) = mpsc::unbounded();

    let closure = Closure::wrap(Box::new(move |e: Event| {
        let input = e
            .current_target()
            .unwrap()
            .dyn_into::<web_sys::HtmlFormElement>()
            .unwrap()
            .get_with_index(0)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        log!(format_args!("user input, add peer: {:?}", input.value()).to_string());
        let multi_addr = input
            .value()
            .parse::<Multiaddr>()
            .expect("invalid multiaddr");
        e.prevent_default();
        peer_producer.unbounded_send(multi_addr).unwrap_throw();
    }) as Box<dyn FnMut(_)>);
    input.set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
    closure.forget();
    peer_consumer
}

fn attach_add_message_box() -> UnboundedReceiver<String> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let input = document
        .get_element_by_id("message")
        .expect("message input should be defined")
        .dyn_into::<HtmlFormElement>()
        .expect("elemnt is a form");

    // channel split
    let (mut message_producer, message_consumer) = mpsc::unbounded();

    // move producer into closure of input
    let closure = Closure::wrap(Box::new(move |e: Event| {
        //absoulte unit of a call chain
        let input = e
            .current_target()
            .unwrap()
            .dyn_into::<web_sys::HtmlFormElement>()
            .unwrap()
            .get_with_index(0)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        log!(format_args!("user input, send message: {:?}", input.value()).to_string());

        message_producer
            .unbounded_send(input.value())
            .unwrap_throw();

        input.set_value("");
        e.prevent_default();
    }) as Box<dyn FnMut(_)>);

    input.set_onsubmit(Some(&closure.as_ref().unchecked_ref()));
    closure.forget();

    message_consumer
}

fn start_chat(
    mut peer_consumer: UnboundedReceiver<Multiaddr>,
    mut message_consumer: UnboundedReceiver<String>,
) {
    spawn_local(async move {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        log!(format_args!("Local peer ids: {:?}", local_peer_id).to_string());

        let topic = Topic::new("chat");
        let gossip_topic= gossipsub::Sha256Topic::new("chat-gossip");

        let ws = ExtTransport::new(websocket_transport());
        log!("created websocket transport");
        let noise = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&local_key)
            .unwrap();
        log!("created noise encrypt");
        // Behavior
        #[derive(NetworkBehaviour)]
        #[behaviour(out_event = "OutEvent")]
        struct DevBrowserBehavior {
            floodsub: Floodsub,
            gossipsub:gossipsub::Gossipsub,
            ping: libp2p::ping::Behaviour, // ping is used to force keepalive during development
        }

        #[derive(Debug)]
        enum OutEvent {
            Floodsub(FloodsubEvent),
            Gossipsub(gossipsub::GossipsubEvent),
            Ping(libp2p::ping::Event),
        }

        impl From<libp2p::ping::Event> for OutEvent {
            fn from(v: libp2p::ping::Event) -> Self {
                Self::Ping(v)
            }
        }
        impl From<FloodsubEvent> for OutEvent {
            fn from(v: FloodsubEvent) -> Self {
                Self::Floodsub(v)
            }
        }
        impl From<gossipsub::GossipsubEvent> for OutEvent {
            fn from(v: gossipsub::GossipsubEvent) -> Self {
                Self::Gossipsub(v)
            }
        }
        log!("creating behaviour");
        let behaviour:DevBrowserBehavior = {
            let floodsub = Floodsub::new(local_peer_id);
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::new());
            let cfg = gossipsub::GossipsubConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Permissive) // This sets the kind of message validation. The default is Strict (enforce message signing)
                // same content will be propagated.
                .build()
                .expect("Valid config");
            let gossipsub = gossipsub::Gossipsub::new(gossipsub::MessageAuthenticity::Signed(local_key), cfg).unwrap();

            // subscribes to our topic
            let mut behaviour = DevBrowserBehavior {
                floodsub,
                gossipsub,
                ping,
            };
            behaviour.floodsub.subscribe(topic.clone());
            behaviour.gossipsub.subscribe(&gossip_topic).unwrap();
            behaviour
        };
        let base = Transport::boxed(ws);
        log!("created base transport");
        // let base = OrTransport::new(webrtc, ws);
        let transport = base
            .upgrade(V1Lazy)
            .authenticate(NoiseConfig::xx(noise).into_authenticated())
            .multiplex(YamuxConfig::default())
            .timeout(Duration::from_secs(10))
            .boxed();

        log!("created full transports");
        let mut swarm= Swarm::new(transport, behaviour, local_peer_id);
        log!("created swarm");
        loop {
            log!("looping over events!");
            select! {
                event = swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {:?}", address),
                    SwarmEvent::Behaviour(event) => match event {
                        // messages!
                        OutEvent::Floodsub(
                            FloodsubEvent::Message(message)
                        ) => log!(format_args!(
                            "Received: '{:?}' from {:?}",
                            String::from_utf8_lossy(&message.data),
                            message.source
                        ).to_string()),

                        OutEvent::Gossipsub(
                            gossipsub::GossipsubEvent::Message{message, ..}
                        ) => log!(format_args!(
                            "Received Gossip: '{:?}' from {:?}",
                            String::from_utf8_lossy(&message.data),
                            message.source
                        ).to_string()),

                        // etc events
                        OutEvent::Floodsub(e) => { log!(format_args!("Floodsub event: {:?}", e).to_string())},
                        OutEvent::Gossipsub(e) => { log!(format_args!("Gossipsub event: {:?}", e).to_string())},

                        e => { log!(format_args!("Ping event: {:?}", e).to_string())}
                    },
                    SwarmEvent::IncomingConnection {
                        local_addr,
                        send_back_addr,
                    } => log!(format_args!(
                        "incoming from:{:?}, sendback: {:?}",
                        local_addr, send_back_addr
                    )
                    .to_string()),
                    SwarmEvent::Dialing(peer_id) => log!(format_args!("dialing {:?}", peer_id).to_string()),

                    //add all new peers to floodsub
                    SwarmEvent::ConnectionEstablished{peer_id, ..} => {
                        let b = swarm
                        .behaviour_mut();
                        b.floodsub
                        .add_node_to_partial_view(peer_id);
                        b.gossipsub.add_explicit_peer(&peer_id);
                        log!(format_args!("Connected to {:?}", peer_id).to_string());
                    },
                    e => ()//{ log!(format_args!("{:?}", e).to_string())}
                },
                peer_multiaddr = peer_consumer.select_next_some() => {
                    log!(format_args!("trying to establish connection {:?}", peer_multiaddr).to_string());
                    match swarm.dial(peer_multiaddr) {
                        Err(e) => log!(format_args!("dial error {:?}", e).to_string()),
                        _ => ()
                    }
                },
                message = message_consumer.select_next_some() => {
                    swarm.behaviour_mut().floodsub.publish(topic.clone(), message.clone());
                    swarm.behaviour_mut().gossipsub.publish(gossip_topic.clone(), message);
                },
            }
        }
        log!("async function ended");
    });
    log!("function  ended");
}
