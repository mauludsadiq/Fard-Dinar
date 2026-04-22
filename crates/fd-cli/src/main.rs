use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fd_core::{verify_consistency, verify_replay, verify_transition, Event, GenesisConfiguration, LedgerState, ObjectStore, ProgramManifest, Receipt};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(name = "fardverify")]
#[command(about = "Verification and replay tool for Fard Dinar")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Fd {
        #[arg(long)]
        event: PathBuf,
        #[arg(long)]
        pre_state: PathBuf,
        #[arg(long)]
        objects: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    FdReplay {
        #[arg(long)]
        events: PathBuf,
        #[arg(long)]
        genesis: PathBuf,
        #[arg(long)]
        objects: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    FdConsistency {
        #[arg(long)]
        events: PathBuf,
    },
    FdApply {
        #[arg(long)]
        event: PathBuf,
        #[arg(long)]
        pre_state: PathBuf,
        #[arg(long)]
        objects: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    FdReceipt {
        #[arg(long)]
        receipt: PathBuf,
    },
    FdSupply {
        #[arg(long)]
        state: PathBuf,
    },
    FdDiff {
        old_state: PathBuf,
        new_state: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Fd { event, pre_state, objects, repo } => {
            let event: Event = read_json(&event)?;
            let pre_state: LedgerState = read_json(&pre_state)?;
            let store = ObjectStore::new(objects);
            let manifest = build_program_manifest(&repo)?;
            let out = verify_transition(&store, &manifest, &pre_state, &event)?;
            print_json(&out)?;
        }
        Command::FdReplay { events, genesis, objects, repo } => {
            let events: Vec<Event> = read_json(&events)?;
            let genesis: GenesisConfiguration = read_json(&genesis)?;
            let genesis_state: LedgerState = genesis.into();
            let store = ObjectStore::new(objects);
            let manifest = build_program_manifest(&repo)?;
            let out = verify_replay(&store, &manifest, &genesis_state, &events)?;
            print_json(&out)?;
        }
        Command::FdConsistency { events } => {
            let events: Vec<Event> = read_json(&events)?;
            let out = verify_consistency(&events)?;
            print_json(&out)?;
        }
        Command::FdApply { event, pre_state, objects, out, receipt_out, repo } => {
            let event: Event = read_json(&event)?;
            let pre_state: LedgerState = read_json(&pre_state)?;
            let store = ObjectStore::new(objects);
            let manifest = build_program_manifest(&repo)?;
            let result = fd_core::apply_event(&store, &manifest, &pre_state, &event)?;
            let state_json = serde_json::to_string_pretty(&result.state)?;
            std::fs::write(&out, state_json)
                .with_context(|| format!("failed to write {}", out.display()))?;
            if let Some(receipt_path) = receipt_out {
                let receipt_json = serde_json::to_string_pretty(&result.receipt)?;
                std::fs::write(&receipt_path, receipt_json)
                    .with_context(|| format!("failed to write {}", receipt_path.display()))?;
            }
            print_json(&result.receipt)?;
        }
        Command::FdReceipt { receipt } => {
            let receipt: Receipt = read_json(&receipt)?;
            print_receipt_human(&receipt)?;
        }
        Command::FdSupply { state } => {
            let state: LedgerState = read_json(&state)?;
            print_supply_human(&state)?;
        }
        Command::FdDiff { old_state, new_state } => {
            let old_state: LedgerState = read_json(&old_state)?;
            let new_state: LedgerState = read_json(&new_state)?;
            print_diff_human(&old_state, &new_state)?;
        }
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed = serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed)
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    println!("{text}");
    Ok(())
}

fn print_receipt_human(receipt: &Receipt) -> Result<()> {
    println!("Run ID:      {}", receipt.run_id);
    println!("Program:     {}", receipt.program_hash);
    println!("Input:       {}", receipt.input_hash);
    println!("Pre-state:   {}", receipt.pre_state_hash);
    println!("Post-state:  {}", receipt.post_state_hash);
    println!("Trace:       {}", receipt.trace_hash);
    Ok(())
}

fn print_supply_human(state: &LedgerState) -> Result<()> {
    let supply: u64 = state.accounts.values().map(|a| a.balance).sum();
    println!("Total supply: {} FD", supply);
    println!("Accounts: {}", state.accounts.len());
    Ok(())
}

fn print_diff_human(old_state: &LedgerState, new_state: &LedgerState) -> Result<()> {
    let mut keys = std::collections::BTreeSet::new();
    for k in old_state.accounts.keys() { keys.insert(k.clone()); }
    for k in new_state.accounts.keys() { keys.insert(k.clone()); }

    for key in keys {
        let old_acc = old_state.account_or_default(&key);
        let new_acc = new_state.account_or_default(&key);
        let old_exists = old_state.accounts.contains_key(&key);
        let new_exists = new_state.accounts.contains_key(&key);
        if !old_exists && new_exists {
            println!("+ accounts.{} : {{ balance: {}, next_nonce: {} }}", key, new_acc.balance, new_acc.next_nonce);
        } else if old_exists && !new_exists {
            println!("- accounts.{} : {{ balance: {}, next_nonce: {} }}", key, old_acc.balance, old_acc.next_nonce);
        } else if old_acc != new_acc {
            println!("~ accounts.{} : balance {} -> {}, next_nonce {} -> {}", key, old_acc.balance, new_acc.balance, old_acc.next_nonce, new_acc.next_nonce);
        }
    }

    let old_supply: u64 = old_state.accounts.values().map(|a| a.balance).sum();
    let new_supply: u64 = new_state.accounts.values().map(|a| a.balance).sum();
    let delta: i128 = new_supply as i128 - old_supply as i128;
    println!("Supply: {} -> {} ({:+})", old_supply, new_supply, delta);
    Ok(())
}

fn build_program_manifest(repo_root: &Path) -> Result<ProgramManifest> {
    let mut modules = BTreeMap::new();
    for entry in WalkDir::new(repo_root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !matches!(ext, "rs" | "toml") {
            continue;
        }
        let rel = path.strip_prefix(repo_root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let bytes = fs::read(path)?;
        modules.insert(rel, fd_core::sha256_tagged(&bytes).0);
    }
    Ok(ProgramManifest {
        name: "fd-core".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        modules,
    })
}
