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
        nonce: Option<u64>,
        #[arg(long, default_value_t = false)]
        auto_nonce: bool,
    },
    PayRequest {
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        nonce: Option<u64>,
        #[arg(long, default_value_t = false)]
        auto_nonce: bool,
    },
    History {
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        receipts_dir: PathBuf,
        #[arg(long)]
        events_dir: PathBuf,
    },
    Rewards {
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        receipts_dir: PathBuf,
        #[arg(long)]
        events_dir: PathBuf,
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
            auto_nonce,
        } => {
            let wallet = load_wallet(&wallet)?;
            let client = Client::new(&base_url);
            let resolved_nonce = if auto_nonce {
                let state = client.get_state()?;
                state.account_or_default(&wallet.public_key_hex()).next_nonce
            } else {
                nonce.context("missing --nonce (or pass --auto-nonce)")?
            };
            let res = client.submit_signed_transfer(&wallet, &to, amount, resolved_nonce)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Command::PayRequest {
            wallet,
            base_url,
            file,
            nonce,
            auto_nonce,
        } => {
            let wallet = load_wallet(&wallet)?;
            let client = Client::new(&base_url);
            let req: serde_json::Value = serde_json::from_slice(
                &fs::read(&file).with_context(|| format!("failed to read {}", file.display()))?
            ).with_context(|| format!("failed to parse {}", file.display()))?;
            let to = req.get("to").and_then(|v| v.as_str()).context("missing to")?;
            let amount = req.get("amount").and_then(|v| v.as_u64()).context("missing amount")?;
            let memo = req.get("memo").and_then(|v| v.as_str()).unwrap_or("");
            let resolved_nonce = if auto_nonce {
                let state = client.get_state()?;
                state.account_or_default(&wallet.public_key_hex()).next_nonce
            } else {
                nonce.context("missing --nonce (or pass --auto-nonce)")?
            };
            let res = client.submit_signed_transfer(&wallet, to, amount, resolved_nonce)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "amount": amount,
                    "memo": memo,
                    "ok": res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
                    "response": res,
                    "to": to,
                    "used_nonce": resolved_nonce
                }))?
            );
        }

        Command::History {
            wallet,
            receipts_dir,
            events_dir,
        } => {
            let wallet = load_wallet(&wallet)?;
            let pk = wallet.public_key_hex();
            let mut rows = Vec::new();

            for entry in fs::read_dir(&receipts_dir)
                .with_context(|| format!("failed to read {}", receipts_dir.display()))? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }

                let receipt: fd_core::Receipt = match serde_json::from_slice(
                    &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?
                ) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                let event_file = events_dir.join(format!("{}.json", receipt.input_hash.replace(":", "_")));
                if !event_file.exists() {
                    continue;
                }

                let event: serde_json::Value = match serde_json::from_slice(
                    &fs::read(&event_file).with_context(|| format!("failed to read {}", event_file.display()))?
                ) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let from = event.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to = event.get("to").and_then(|v| v.as_str()).unwrap_or("");

                if from != pk && to != pk {
                    continue;
                }

                let amount = event.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
                let (user_reward, vendor_reward, is_merchant) = receipt.transfer_effects
                    .as_ref()
                    .map(|fx| (fx.user_reward, fx.vendor_reward, fx.is_merchant))
                    .unwrap_or((0, 0, false));

                let direction = if from == pk { "out" } else { "in" };
                let counterparty = if from == pk { to } else { from };

                rows.push(json!({
                    "run_id": receipt.run_id,
                    "direction": direction,
                    "counterparty": counterparty,
                    "amount": amount,
                    "user_reward": user_reward,
                    "vendor_reward": vendor_reward,
                    "is_merchant": is_merchant
                }));
            }

            println!("{}", serde_json::to_string_pretty(&json!({
                "public_key_hex": pk,
                "history": rows
            }))?);
        }

        Command::Rewards {
            wallet,
            receipts_dir,
            events_dir,
        } => {
            let wallet = load_wallet(&wallet)?;
            let pk = wallet.public_key_hex();
            let mut total_rewards = 0_u64;
            let mut merchant_rewards = 0_u64;
            let mut p2p_rewards = 0_u64;
            let mut by_counterparty = serde_json::Map::new();

            for entry in fs::read_dir(&receipts_dir)
                .with_context(|| format!("failed to read {}", receipts_dir.display()))? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }

                let receipt: fd_core::Receipt = match serde_json::from_slice(
                    &fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?
                ) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                let event_file = events_dir.join(format!("{}.json", receipt.input_hash.replace(":", "_")));
                if !event_file.exists() {
                    continue;
                }

                let event: serde_json::Value = match serde_json::from_slice(
                    &fs::read(&event_file).with_context(|| format!("failed to read {}", event_file.display()))?
                ) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let from = event.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to = event.get("to").and_then(|v| v.as_str()).unwrap_or("");

                if from != pk {
                    continue;
                }

                let counterparty = to;
                let (user_reward, is_merchant) = receipt.transfer_effects
                    .as_ref()
                    .map(|fx| (fx.user_reward, fx.is_merchant))
                    .unwrap_or((0, false));

                total_rewards += user_reward;
                if is_merchant {
                    merchant_rewards += user_reward;
                } else {
                    p2p_rewards += user_reward;
                }

                let current = by_counterparty
                    .get(counterparty)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                by_counterparty.insert(counterparty.to_string(), json!(current + user_reward));
            }

            println!("{}", serde_json::to_string_pretty(&json!({
                "public_key_hex": pk,
                "total_rewards": total_rewards,
                "merchant_rewards": merchant_rewards,
                "p2p_rewards": p2p_rewards,
                "by_counterparty": by_counterparty
            }))?);
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
