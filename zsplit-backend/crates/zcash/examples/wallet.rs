//! Operator-only native wallet smoke tool. No recipient spending key is accepted.
use std::{env, path::PathBuf, time::Instant};
use uuid::Uuid;
use zsplit_domain::{FundingCommand, Network, PayoutEngine, Zats};
use zsplit_zcash::{NativeWallet, WalletConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let action = args.get(1).map(String::as_str).unwrap_or("help");
    if action == "help" {
        println!(
            "wallet seed | address ROOM_UUID | sync | status TXID | pay ROOM_UUID PAYOUT_UUID AMOUNT_ZAT"
        );
        println!(
            "Set ZCASH_DATA_DIR,ZCASH_SEED_FILE,LIGHTWALLETD_URL. pay reads EPHEMERAL_RECEIVER from environment; it builds and broadcasts REAL testnet ZEC."
        );
        return Ok(());
    }
    let data_dir = PathBuf::from(env::var("ZCASH_DATA_DIR").unwrap_or("./data/wallet".into()));
    let seed_file =
        PathBuf::from(env::var("ZCASH_SEED_FILE").unwrap_or("./data/coordinator.seed".into()));
    if action == "seed" {
        if let Some(parent) = seed_file.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        NativeWallet::create_seed_file(&seed_file)?;
        println!("Created protected seed file. Back it up offline before accepting deposits.");
        return Ok(());
    }
    let start = Instant::now();
    let wallet = NativeWallet::open(WalletConfig {
        data_dir,
        seed_file,
        lightwalletd_url: env::var("LIGHTWALLETD_URL")
            .unwrap_or("https://testnet.zec.rocks:443".into()),
        network: Network::Testnet,
        birthday_height: env::var("ZCASH_BIRTHDAY_HEIGHT")
            .ok()
            .map(|s| s.parse())
            .transpose()?,
        confirmations: env::var("ZCASH_CONFIRMATIONS")
            .unwrap_or("10".into())
            .parse()?,
        sapling_spend_params: env::var_os("SAPLING_SPEND_PARAMS").map(PathBuf::from),
        sapling_output_params: env::var_os("SAPLING_OUTPUT_PARAMS").map(PathBuf::from),
    })
    .await?;
    println!(
        "{{\"event\":\"open\",\"duration_ms\":{}}}",
        start.elapsed().as_millis()
    );
    match action {
        "address" => {
            let room = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("room UUID required"))?
                .parse()?;
            println!(
                "{}",
                serde_json::to_string(&wallet.deposit_address(room).await?)?
            );
        }
        "sync" => {
            let start = Instant::now();
            let balances = wallet.sync().await?;
            println!(
                "{{\"event\":\"sync\",\"duration_ms\":{},\"balances\":{}}}",
                start.elapsed().as_millis(),
                serde_json::to_string(&balances)?
            );
        }
        "status" => println!(
            "{}",
            serde_json::to_string(
                &wallet
                    .transaction_status(
                        args.get(2)
                            .ok_or_else(|| anyhow::anyhow!("txid required"))?
                    )
                    .await?
            )?
        ),
        "pay" => {
            if env::var("ZCASH_ALLOW_TESTNET_PAYMENT").as_deref() != Ok("yes") {
                anyhow::bail!("set ZCASH_ALLOW_TESTNET_PAYMENT=yes to authorize a real payment");
            }
            let room_id: Uuid = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("room UUID required"))?
                .parse()?;
            let payout_id: Uuid = args
                .get(3)
                .ok_or_else(|| anyhow::anyhow!("durable payout UUID required"))?
                .parse()?;
            let amount = Zats::new(
                args.get(4)
                    .ok_or_else(|| anyhow::anyhow!("zatoshi amount required"))?
                    .parse()?,
            )?;
            wallet.sync().await?;
            let start = Instant::now();
            let tx = wallet
                .prepare_payout(FundingCommand {
                    payout_id,
                    room_id,
                    network: Network::Testnet,
                    amount,
                    ephemeral_receiver: zeroize::Zeroizing::new(env::var("EPHEMERAL_RECEIVER")?),
                })
                .await?;
            println!(
                "{{\"event\":\"proved\",\"duration_ms\":{},\"txid\":\"{}\"}}",
                start.elapsed().as_millis(),
                tx.txid
            );
            wallet.broadcast(&tx).await?;
            println!(
                "Broadcast accepted; confirmation requires wallet sync and status. txid={}",
                tx.txid
            );
        }
        _ => anyhow::bail!("unknown action"),
    }
    Ok(())
}
