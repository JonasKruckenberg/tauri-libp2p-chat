use libp2p::{
  core::upgrade,
  floodsub::{Floodsub, FloodsubEvent, Topic},
  identity,
  mdns::{Mdns, MdnsConfig, MdnsEvent},
  mplex, noise,
  swarm::SwarmBuilder,
  tcp::TokioTcpConfig,
  NetworkBehaviour, PeerId, Transport,
};
use serde::Serialize;
use tauri::{plugin::Plugin, Invoke, Manager, Params, State, Window};
use tokio::sync::mpsc;

#[derive(Debug)]
enum NodeCommand {
  Message { message: String, from: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum NodeEvent {
  Message { message: String, from: String },
}

#[derive(Debug)]
enum BehaviourEvent {
  MdnsEvent(MdnsEvent),
  FloodsubEvent(FloodsubEvent),
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent", event_process = false)]
struct Behaviour {
  mdns: Mdns,
  floodsub: Floodsub,
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

#[tauri::command]
async fn send(
  message: String,
  peer_id: State<'_, PeerId>,
  cmd_tx: State<'_, tauri::async_runtime::Sender<NodeCommand>>,
) -> Result<(), String> {
  cmd_tx
    .send(NodeCommand::Message {
      message,
      from: peer_id.to_base58(),
    })
    .await
    .unwrap();
  Ok(())
}

pub struct TauriLibp2p<P: Params> {
  invoke_handler: Box<dyn Fn(Invoke<P>) + Send + Sync>,
}

impl<P: Params> TauriLibp2p<P> {
  pub fn new() -> Self {
    Self {
      invoke_handler: Box::new(tauri::generate_handler![send]),
    }
  }
}

impl<P: Params> Plugin<P> for TauriLibp2p<P> {
  fn name(&self) -> &'static str {
    "libp2p"
  }

  /// Extend the invoke handler.
  fn extend_api(&mut self, invoke: Invoke<P>) {
    (self.invoke_handler)(invoke)
  }

  fn created(&mut self, window: Window<P>) {
    // create the command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<NodeCommand>(100);
    window.manage(cmd_tx);

    // Create a random PeerId.
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer id: {:?}", peer_id);
    window.manage(peer_id);

    // spawn the libp2p node
    tauri::async_runtime::spawn(async move {
      let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

      // Create a transport.
      let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

      let floodsub_topic = Topic::new("chat");

      let mut swarm = {
        let mut behaviour = Behaviour {
          mdns: Mdns::new(MdnsConfig::default()).await.unwrap(),
          floodsub: Floodsub::new(peer_id.clone()),
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());

        SwarmBuilder::new(transport, behaviour, peer_id)
          .executor(Box::new(|fut| {
            tokio::spawn(fut);
          }))
          .build()
      };

      swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

      loop {
        tokio::select! {
          Some(cmd) = cmd_rx.recv() => {
            match cmd {
              NodeCommand::Message { message, .. } => {
                swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), message);
              }
            }
          }
          event = swarm.next() => {
            match event {
              BehaviourEvent::MdnsEvent(MdnsEvent::Discovered(peers)) => {
                for (peer, _) in peers {
                  swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer);
                }
              }
              BehaviourEvent::MdnsEvent(MdnsEvent::Expired(expired)) => {
                for (peer, _) in expired {
                  swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
                }
              }
              BehaviourEvent::FloodsubEvent(FloodsubEvent::Message(message)) => {
                let from = message.source.to_base58();
                let message = String::from_utf8_lossy(&message.data).into_owned();

                window.emit(&"plugin:libp2p|message".parse().unwrap_or_else(|_| {
                  panic!("could not parse tag");
                }), NodeEvent::Message{ message, from }).unwrap();
              }
              _ => {}
            }
          }
        }
      }
    });
  }
}
