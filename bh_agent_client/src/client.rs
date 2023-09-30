use bh_agent_common::BhAgentServiceClient;
use tarpc::{client, tokio_serde::formats::Json};
use tokio::net::ToSocketAddrs;

pub async fn build_client<A>(socket_addr: A) -> anyhow::Result<BhAgentServiceClient>
where
    A: ToSocketAddrs,
{
    let mut transport = tarpc::serde_transport::tcp::connect(socket_addr, Json::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = BhAgentServiceClient::new(client::Config::default(), transport.await?).spawn();

    Ok(client)
}
