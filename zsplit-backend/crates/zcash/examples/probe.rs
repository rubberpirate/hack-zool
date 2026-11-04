//! Public upstream diagnostic: requests no wallet data or secrets.
use std::time::Duration;
use zcash_client_backend::proto::service::{
    Empty, compact_tx_streamer_client::CompactTxStreamerClient,
};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("LIGHTWALLETD_URL").unwrap_or("https://testnet.zec.rocks:443".into());
    let channel = tonic::transport::Endpoint::from_shared(url)?
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .tls_config(tonic::transport::ClientTlsConfig::new().with_webpki_roots())?
        .connect()
        .await?;
    let mut client = CompactTxStreamerClient::new(channel);
    let info = client.get_lightd_info(Empty {}).await?.into_inner();
    println!(
        "chain={} height={} estimated_height={} branch={}",
        info.chain_name, info.block_height, info.estimated_height, info.consensus_branch_id
    );
    Ok(())
}
