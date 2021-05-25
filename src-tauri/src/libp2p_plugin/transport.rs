use libp2p::{core::upgrade, identity, mplex, noise, tcp::TokioTcpConfig, Transport};

pub fn tokio_tcp_noise_mplex(keys: identity::Keypair) -> libp2p::core::transport::Boxed<(libp2p::PeerId, libp2p::core::muxing::StreamMuxerBox)>{
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed()
}
