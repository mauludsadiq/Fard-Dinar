use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fd_client::Client;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "fd-vendor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    RequestPayment {
        #[arg(long)]
        vendor: PathBuf,
        #[arg(long)]
        amount: u64,
        #[arg(long, default_value = "")]
        memo: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    VerifyReceipt {
        #[arg(long)]
        vendor: PathBuf,
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        amount: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::RequestPayment { vendor, amount, memo, out } => {
            let vendor_pubkey = load_public_key(&vendor)?;
            let req = json!({
                "amount": amount,
                "kind": "fd_payment_request_v1",
                "memo": memo,
                "nonce_mode": "auto",
                "to": vendor_pubkey,
            });
            let text = serde_json::to_string_pretty(&req)?;
            if let Some(path) = out {
                fs::write(&path, &text)
                    .with_context(|| format!("failed to write {}", path.display()))?;
            }
            println!("{}", text);
        }
        Command::VerifyReceipt {
            vendor,
            base_url,
            run_id,
            amount,
        } => {
            let vendor_pubkey = load_public_key(&vendor)?;
            let client = Client::new(&base_url);
            let receipt = client.get_receipt(&run_id)?;
            let ok = receipt.transfer_effects.as_ref().map(|fx| fx.amount == amount && fx.is_merchant).unwrap_or(false);
            let result = json!({
                "ok": ok,
                "vendor_pubkey": vendor_pubkey,
                "run_id": run_id,
                "expected_amount": amount,
                "receipt": receipt,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }
    Ok(())
}

fn load_public_key(path: &PathBuf) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).with_context(|| format!("failed to read {}", path.display()))?)?;
    value
        .get("public_key_hex")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("missing public_key_hex")
}
