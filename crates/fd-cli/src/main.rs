use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fd_core::{verify_consistency, verify_replay, verify_transition, Event, GenesisConfiguration, LedgerState, ObjectStore, ProgramManifest};
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
