#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
pub mod libp2p_plugin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  tauri::Builder::default()
    .plugin(libp2p_plugin::TauriLibp2p::new())
    .run(tauri::generate_context!())
    .expect("failed to run app");

  Ok(())
}