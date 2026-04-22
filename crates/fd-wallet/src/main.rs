use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fd_client::{Client, Wallet};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "fd-wallet")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init {
        #[arg(long)]
        secret_hex: String,
        #[arg(long)]
        out: PathBuf,
    },
    Address {
        #[arg(long)]
        wallet: PathBuf,
    },
    Balance {
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        base_url: String,
    },
    Send {
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        nonce: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { secret_hex, out } => {
            let wallet = Wallet::from_secret_hex(&secret_hex)?;
            let doc = json!({
                "secret_key_hex": secret_hex,
                "public_key_hex": wallet.public_key_hex(),
            });
            fs::write(&out, serde_json::to_string_pretty(&doc)?)
                .with_context(|| format!("failed to write {}", out.display()))?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        Command::Address { wallet } => {
            let wallet = load_wallet(&wallet)?;
            println!("{}", wallet.public_key_hex());
        }
        Command::Balance { wallet, base_url } => {
            let wallet = load_wallet(&wallet)?;
            let client = Client::new(&base_url);
            let state = client.get_state()?;
            let pk = wallet.public_key_hex();
            let account = state.account_or_default(&pk);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "public_key_hex": pk,
                    "balance": account.balance,
                    "next_nonce": account.next_nonce
                }))?
            );
        }
        Command::Send {
            wallet,
            base_url,
            to,
            amount,
            nonce,
        } => {
            let wallet = load_wallet(&wallet)?;
            let client = Client::new(&base_url);
            let res = client.submit_signed_transfer(&wallet, &to, amount, nonce)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
    }
    Ok(())
}

fn load_wallet(path: &PathBuf) -> Result<Wallet> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).with_context(|| format!("failed to read {}", path.display()))?)?;
    let secret_hex = value
        .get("secret_key_hex")
        .and_then(|v| v.as_str())
        .context("missing secret_key_hex")?;
    Wallet::from_secret_hex(secret_hex)
}
