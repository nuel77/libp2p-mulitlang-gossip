use libp2p::core::transport::upgrade;
use libp2p::floodsub::{Floodsub, FloodsubEvent};
use libp2p::gossipsub::{Gossipsub, GossipsubEvent, Sha256Topic, ValidationMode};
use libp2p::swarm::SwarmEvent;
use libp2p::yamux::YamuxConfig;
use libp2p::{
    dns, floodsub, gossipsub, identity, noise, tcp, websocket, Multiaddr, PeerId, Swarm, Transport,
};
use std::error::Error;
use async_std::io;
use std::time::Duration;
use futures::{prelude::*, select};
use libp2p::futures;


#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //generate local key
    let local_key = identity::Keypair::generate_ed25519();
    let my_peer_id = PeerId::from(local_key.public());
    println!("local peer_id : {:?} ", my_peer_id);

    let noise_encryption = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&local_key)
        .unwrap();

    let base_transport = websocket::WsConfig::new(
        dns::DnsConfig::system(tcp::TcpTransport::new(
            tcp::GenTcpConfig::new().nodelay(true),
        ))
        .await?,
    )
    .boxed();

    let transport = base_transport
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::NoiseConfig::xx(noise_encryption).into_authenticated())
        .multiplex(YamuxConfig::default())
        .timeout(Duration::from_secs(10))
        .boxed();

    let flood_sub_topic = floodsub::Topic::new("chat");
    let gossip_sub_topic = Sha256Topic::new("chat-gossip");
    println!("gossip sub topic {}", gossip_sub_topic);

    #[derive(libp2p::NetworkBehaviour)]
    #[behaviour(out_event = "OutEvent")]
    struct MyBehaviour {
        floodsub: Floodsub,
        gossipsub: Gossipsub,
        ping: libp2p::ping::Behaviour,
    }

    #[derive(Debug)]
    enum OutEvent {
        Floodsub(FloodsubEvent),
        Gossipsub(GossipsubEvent),
        Ping(libp2p::ping::Event),
    }

    impl From<FloodsubEvent> for OutEvent {
        fn from(e: FloodsubEvent) -> Self {
            Self::Floodsub(e)
        }
    }

    impl From<GossipsubEvent> for OutEvent {
        fn from(e: GossipsubEvent) -> Self {
            Self::Gossipsub(e)
        }
    }
    impl From<libp2p::ping::Event> for OutEvent {
        fn from(e: libp2p::ping::Event) -> Self {
            Self::Ping(e)
        }
    }

    // Create a Swarm to manage peers and events
    let mut swarm = {
        use libp2p::ping;

        let ping = ping::Behaviour::new(ping::Config::new());
        let cfg = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Permissive) // This sets the kind of message validation. The default is Strict (enforce message signing)
            // same content will be propagated.
            .build()
            .expect("Valid config");

        let mut gossipsub =
            Gossipsub::new(gossipsub::MessageAuthenticity::Signed(local_key), cfg).unwrap();
        // gossipsub.set_topic_params(gossipsub_topic, TopicScoreParams::default());
        // gossipsub.with_peer_score(PeerScoreParams::default(), PeerScoreThresholds::default());
        gossipsub.subscribe(&gossip_sub_topic).unwrap();

        let mut behaviour = MyBehaviour {
            floodsub: Floodsub::new(my_peer_id),
            gossipsub,
            ping,
        };

        behaviour.floodsub.subscribe(flood_sub_topic.clone());
        Swarm::new(transport, behaviour, my_peer_id)
    };
    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("Dialed {:?}", to_dial)
    }

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
    swarm.listen_on("/ip4/0.0.0.0/tcp/0/ws".parse()?)?;
    loop {
        select! {
            line = stdin.select_next_some() => {
                    let line = line.expect("Stdin not to close");
                    let b = swarm.behaviour_mut();
                    b.floodsub.publish(flood_sub_topic.clone(), line.as_bytes());
                    b.gossipsub.publish(gossip_sub_topic.clone(), line);
                }

            event = swarm.select_next_some() =>  match event {
                 SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?}", address);
                }

                 SwarmEvent::Behaviour(OutEvent::Gossipsub(
                    GossipsubEvent::Message { message, .. }
                )) => {
                    println!(
                        "Received gossip: '{:?}' from {:?}",
                        String::from_utf8_lossy(&message.data),
                        message.source
                    );
                }

                SwarmEvent::ConnectionEstablished{peer_id, ..} => {
                    swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                }
                e => {println!("unhandled event: {:?}",e)}
            }
        }
    }
    Ok(())
}
