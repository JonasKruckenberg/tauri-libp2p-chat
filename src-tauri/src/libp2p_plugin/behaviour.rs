use libp2p::{
  NetworkBehaviour,
  mdns::{Mdns,MdnsEvent,MdnsConfig},
  floodsub::{Floodsub,FloodsubEvent},
  PeerId
};

/// A combined NetworkBehaviour that supports both MDNS and Floodsub
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent", event_process = false)]
pub struct Behaviour {
  pub mdns: Mdns,
  pub floodsub: Floodsub,
}

impl Behaviour {
  pub async fn new(peer_id: PeerId) -> Self {
    Self {
      mdns: Mdns::new(MdnsConfig::default()).await.unwrap(),
      floodsub: Floodsub::new(peer_id)
    }
  }
}

/// The custom NetworkEvent emits these events
#[derive(Debug)]
pub enum BehaviourEvent {
  MdnsEvent(MdnsEvent),
  FloodsubEvent(FloodsubEvent),
}

impl From<MdnsEvent> for BehaviourEvent {
  fn from(event: MdnsEvent) -> Self {
    BehaviourEvent::MdnsEvent(event)
  }
}
impl From<FloodsubEvent> for BehaviourEvent {
  fn from(event: FloodsubEvent) -> Self {
    BehaviourEvent::FloodsubEvent(event)
  }
}