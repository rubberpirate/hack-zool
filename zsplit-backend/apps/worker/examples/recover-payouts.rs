//! Operator-only recovery. Stop the API first; never reset a native transaction intent.
use std::{env, path::PathBuf};
use uuid::Uuid;
use zsplit_domain::Network;
use zsplit_store::Store;
use zsplit_zcash::{NativeWallet, WalletConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    match dotenvy::from_path(env_file) {
        Ok(()) => {}
        Err(error) if error.not_found() => {}
        Err(_) => anyhow::bail!("Could not parse backend .env"),
    }
    let ids = env::args()
        .skip(1)
        .map(|id| id.parse::<Uuid>())
        .collect::<Result<Vec<_>, _>>()?;
    anyhow::ensure!(
        !ids.is_empty(),
        "Provide explicit payout UUIDs to inspect and recover"
    );
    let required = |name: &str| env::var(name).map_err(|_| anyhow::anyhow!("Missing {name}"));
    let spend = PathBuf::from(required("SAPLING_SPEND_PARAMS")?);
    let output = PathBuf::from(required("SAPLING_OUTPUT_PARAMS")?);
    anyhow::ensure!(
        spend.is_file() && output.is_file(),
        "Install verified proving parameters first"
    );
    let wallet = NativeWallet::open(WalletConfig {
        data_dir: PathBuf::from(required("ZCASH_DATA_DIR")?),
        seed_file: PathBuf::from(required("ZCASH_SEED_FILE")?),
        lightwalletd_url: required("LIGHTWALLETD_URL")?,
        network: Network::Testnet,
        birthday_height: env::var("ZCASH_BIRTHDAY_HEIGHT")
            .ok()
            .map(|v| v.parse())
            .transpose()?,
        confirmations: env::var("ZCASH_CONFIRMATIONS")
            .unwrap_or_else(|_| "3".into())
            .parse()?,
        sapling_spend_params: Some(spend),
        sapling_output_params: Some(output),
    })
    .await?;
    let store = Store::connect(&required("DATABASE_URL")?).await?;
    for id in ids {
        zsplit_worker::requeue_verified_unprepared(&store, &wallet, id).await?;
        println!(
            "Requeued verified unprepared payout {id}; reservation and payout identity preserved"
        );
    }
    Ok(())
}
