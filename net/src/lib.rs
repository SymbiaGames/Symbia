use libp2p::{
    gossipsub::{self, Message, MessageId, TopicHash},
    mdns, noise, swarm::{SwarmEvent, NetworkBehaviour},
    tcp, yamux, PeerId, Swarm, Transport, Multiaddr,
};
use libp2p::identity::Keypair;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::task::{Context, Poll};
use futures::task::noop_waker_ref;
use futures::StreamExt;
use symbia_shared::SymbiaNetMessage;

pub const SYMBIA_TOPIC: &str = "symbia_sync_v1";

// ✅ La macro génère automatiquement l'enum des événements
#[derive(NetworkBehaviour)]
struct SymbiaBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct P2PNode {
    swarm: Swarm<SymbiaBehaviour>,
    topic: TopicHash,
    peer_id: PeerId,
}

impl P2PNode {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let local_key = Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());

        let transport = tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)?)
            .multiplex(yamux::Config::default())
            .boxed();

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(5))
            .validation_mode(gossipsub::ValidationMode::Permissive)
            .message_id_fn(|message: &Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            })
            .build()
            .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        let topic = gossipsub::IdentTopic::new(SYMBIA_TOPIC);
        let topic_hash = topic.hash();
        gossipsub.subscribe(&topic)?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;

        let behaviour = SymbiaBehaviour { gossipsub, mdns };

        let mut swarm = Swarm::new(
            transport,
            behaviour,
            peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(std::time::Duration::from_secs(30)),
        );

        let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?);
        let _ = swarm.listen_on("/ip6/::/tcp/0".parse()?);

        Ok(Self { swarm, topic: topic_hash, peer_id })
    }

    pub fn broadcast(&mut self, msg: &SymbiaNetMessage) {
        let bytes = match bincode::serialize(msg) {
            Ok(b) => b,
            Err(e) => { eprintln!("❌ Sérialisation: {}", e); return; }
        };
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(self.topic.clone(), bytes) {
            eprintln!("❌ Publication: {}", e);
        }
    }

    pub fn poll_events(&mut self) -> Vec<SymbiaNetMessage> {
        let mut messages = Vec::new();
        let waker = noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        loop {
            match self.swarm.poll_next_unpin(&mut cx) {
                Poll::Ready(Some(event)) => {
                    match event {
                        // ✅ Pattern matching DIRECT sur SwarmEvent::Behaviour
                        SwarmEvent::Behaviour(SymbiaBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message { message, .. }
                        )) => {
                            if let Ok(msg) = bincode::deserialize::<SymbiaNetMessage>(&message.data) {
                                messages.push(msg);
                            }
                        }
                        SwarmEvent::Behaviour(SymbiaBehaviourEvent::Mdns(
                            mdns::Event::Discovered(list)
                        )) => {
                            for (peer_id, _addr) in list {
                                println!("🔍 LAN: {} trouvé", peer_id);
                                let _ = self.swarm.dial(peer_id);
                            }
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                            println!("🤝 Connecté à {} via {:?}", peer_id, endpoint);
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            println!("🌐 Écoute: {}", address);
                        }
                        _ => {}
                    }
                }
                Poll::Ready(None) | Poll::Pending => break,
            }
        }
        messages
    }

    pub fn peer_id(&self) -> String {
        self.peer_id.to_string()
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), libp2p::swarm::DialError> {
        self.swarm.dial(addr)
    }
}