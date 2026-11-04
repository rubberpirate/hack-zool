use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};
use zsplit_api::{ApiConfig, AppState};
use zsplit_domain::{Network, PayoutEngine};
use zsplit_store::Store;
use zsplit_zcash::{NativeWallet, WalletConfig};

fn required(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("Missing required configuration {name}"))
}
fn key(name: &str) -> anyhow::Result<[u8; 32]> {
    let encoded = Zeroizing::new(required(name)?);
    let mut decoded = Zeroizing::new(
        hex::decode(encoded.as_str())
            .map_err(|_| anyhow::anyhow!("Invalid key configuration {name}"))?,
    );
    let result = decoded
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid key length {name}"))?;
    decoded.zeroize();
    Ok(result)
}
async fn stop_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {_ = tokio::signal::ctrl_c()=>{},_ = terminate.recv()=>{}}
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    if std::env::args().any(|arg| arg == "--export-openapi") {
        use utoipa::OpenApi;
        println!("{}", zsplit_api::ApiDoc::openapi().to_pretty_json()?);
        return Ok(());
    }
    let network: Network = std::env::var("ZCASH_NETWORK")
        .unwrap_or_else(|_| "testnet".into())
        .parse()?;
    if network != Network::Testnet {
        anyhow::bail!("Only testnet is enabled in this build")
    }
    let confirmations: u32 = std::env::var("ZCASH_CONFIRMATIONS")
        .unwrap_or_else(|_| "3".into())
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid confirmation configuration"))?;
    let store = Store::connect(&required("DATABASE_URL")?).await?;
    store.migrate().await?;
    let organizer_id: Uuid = required("ORGANIZER_ID")?
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid organizer ID"))?;
    store
        .ensure_organizer(organizer_id, "configured-organizer")
        .await?;
    let engine: Arc<dyn PayoutEngine> = Arc::new(
        NativeWallet::open(WalletConfig {
            data_dir: PathBuf::from(
                std::env::var("ZCASH_DATA_DIR").unwrap_or_else(|_| "data/wallet".into()),
            ),
            seed_file: PathBuf::from(required("ZCASH_SEED_FILE")?),
            lightwalletd_url: required("LIGHTWALLETD_URL")?,
            network,
            birthday_height: std::env::var("ZCASH_BIRTHDAY_HEIGHT")
                .ok()
                .map(|v| v.parse())
                .transpose()
                .map_err(|_| anyhow::anyhow!("Invalid birthday height"))?,
            confirmations,
            sapling_spend_params: std::env::var("SAPLING_SPEND_PARAMS")
                .ok()
                .map(PathBuf::from),
            sapling_output_params: std::env::var("SAPLING_OUTPUT_PARAMS")
                .ok()
                .map(PathBuf::from),
        })
        .await?,
    );
    let organizer_secret = Zeroizing::new(required("ORGANIZER_API_TOKEN")?);
    let state = AppState::new(
        store.clone(),
        engine.clone(),
        ApiConfig {
            network,
            confirmations,
            organizer_id,
            allowed_origin: required("ALLOWED_ORIGIN")?,
            public_claim_origin: required("PUBLIC_CLAIM_ORIGIN")?,
            docs_enabled: std::env::var("ENABLE_DOCS").is_ok_and(|v| v == "true"),
        },
        organizer_secret.as_str(),
        key("CLAIM_TOKEN_KEY")?,
        key("RECEIVER_ENCRYPTION_KEY")?,
    )?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let signal_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        stop_signal().await;
        let _ = signal_tx.send(true);
    });
    let worker_alive = state.worker_alive.clone();
    let worker_engine = engine.clone();
    let worker_cipher = state.cipher.clone();
    let worker_rx = shutdown_rx.clone();
    let worker_tx = shutdown_tx.clone();
    let worker = tokio::spawn(async move {
        let result = zsplit_worker::run(
            store,
            worker_engine,
            worker_cipher,
            zsplit_worker::WorkerConfig {
                confirmations,
                poll_interval: Duration::from_secs(5),
            },
            worker_rx,
        )
        .await;
        worker_alive.store(false, Ordering::Release);
        let _ = worker_tx.send(true);
        result
    });
    let address: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".into())
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid bind address"))?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address,%network,"api_started");
    let served = axum::serve(
        listener,
        zsplit_api::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        while !*shutdown_rx.borrow() {
            if shutdown_rx.changed().await.is_err() {
                break;
            }
        }
    })
    .await;
    let _ = shutdown_tx.send(true);
    // Await the worker rather than aborting it: proof construction must persist its durable recovery marker.
    worker
        .await
        .map_err(|_| anyhow::anyhow!("Payout worker terminated unexpectedly"))??;
    served?;
    Ok(())
}
