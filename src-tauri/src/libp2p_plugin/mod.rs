use libp2p::{
  floodsub::{Floodsub, FloodsubEvent, Topic},
  identity,
  mdns::{Mdns, MdnsConfig, MdnsEvent},
  swarm::SwarmBuilder,
  PeerId,
};
use serde::Serialize;
use tauri::{plugin::Plugin, Invoke, Manager, Params, State, Window};
use tokio::sync::mpsc;
pub mod behaviour;
pub mod transport;

/// Commands the tauri thread can use to control the libp2p thread
#[derive(Debug)]
enum NodeCommand {
  Message { message: String, from: String },
}

/// Events the webview can receive from the libp2p thread
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum NodeEvent {
  Message { message: String, from: String },
}

/// Broadcast a message to all listening floodsub peers
#[tauri::command]
async fn broadcast(
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
      invoke_handler: Box::new(tauri::generate_handler![broadcast]),
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
      let transport = transport::tokio_tcp_noise_mplex(id_keys);

      let mut behaviour = behaviour::Behaviour::new(peer_id.clone()).await;

      let floodsub_topic = Topic::new("chat");

      behaviour.floodsub.subscribe(floodsub_topic.clone());

      let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id)
          .executor(Box::new(|fut| {
            tokio::spawn(fut);
          }))
          .build();

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
              behaviour::BehaviourEvent::MdnsEvent(MdnsEvent::Discovered(peers)) => {
                for (peer, _) in peers {
                  swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer);
                }
              }
              behaviour::BehaviourEvent::MdnsEvent(MdnsEvent::Expired(expired)) => {
                for (peer, _) in expired {
                  swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
                }
              }
              behaviour::BehaviourEvent::FloodsubEvent(FloodsubEvent::Message(message)) => {
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
