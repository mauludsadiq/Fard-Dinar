use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use fd_core::{conflict_key, deposit_signing_payload, event_hash, transfer_signing_payload, verify_consistency, verify_replay, verify_transition, DepositAttestation, Event, GenesisConfiguration, LedgerState, ObjectStore, ProgramManifest, Receipt, RegistryEntry, RegistryState, TransferIntent};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use tiny_http::{Server, Response};
use std::thread;
use std::time::Duration;

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
    WalletGen {
        #[arg(long)]
        out: PathBuf,
    },
    WalletShow {
        #[arg(long)]
        secret: PathBuf,
    },
    WalletSignTransfer {
        #[arg(long)]
        secret: PathBuf,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        nonce: u64,
        #[arg(long)]
        out: PathBuf,
    },
    FdNode {
        #[arg(long)]
        watch: PathBuf,
        #[arg(long)]
        genesis: PathBuf,
        #[arg(long)]
        objects: PathBuf,
        #[arg(long)]
        state_out: PathBuf,
        #[arg(long)]
        receipts: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        peer_watch: Vec<PathBuf>,
        #[arg(long)]
        peer_registry: Vec<PathBuf>,
    },
    FdRegistry {
        #[arg(long)]
        watch: PathBuf,
        #[arg(long)]
        registry_out: PathBuf,
        #[arg(long)]
        peer_registry: Vec<PathBuf>,
    },
    FdHttp {
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: String,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        state: Option<PathBuf>,
        #[arg(long)]
        ingest_dir: Option<PathBuf>,
        #[arg(long)]
        receipts_dir: Option<PathBuf>,
        #[arg(long)]
        objects_dir: Option<PathBuf>,
    },

    WalletSignDeposit {
        #[arg(long)]
        secret: PathBuf,
        #[arg(long)]
        beneficiary: String,
        #[arg(long)]
        usd_cents: u64,
        #[arg(long)]
        external_ref: String,
        #[arg(long)]
        timestamp: u64,
        #[arg(long)]
        out: PathBuf,
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
        Command::FdNode { watch, genesis, objects, state_out, receipts, registry, peer_watch, peer_registry } => {
            let mut state: LedgerState = if state_out.exists() {
                read_json(&state_out)?
            } else {
                let genesis_cfg: GenesisConfiguration = read_json(&genesis)?;
                genesis_cfg.into()
            };
            let store = ObjectStore::new(objects);
            let manifest = build_program_manifest(Path::new("."))?;

            std::fs::create_dir_all(&receipts)
                .with_context(|| format!("failed to create {}", receipts.display()))?;
            let errors_dir = watch.join("_errors");
            std::fs::create_dir_all(&errors_dir)
                .with_context(|| format!("failed to create {}", errors_dir.display()))?;

            let processed_path = state_out.with_extension("processed.json");
            let mut processed: std::collections::BTreeSet<String> = if processed_path.exists() {
                read_json(&processed_path)?
            } else {
                std::collections::BTreeSet::new()
            };

            loop {
                for peer_dir in &peer_watch {
                    if peer_dir.exists() {
                        for entry in std::fs::read_dir(peer_dir)? {
                            let entry = entry?;
                            let src = entry.path();
                            if src.extension().and_then(|s| s.to_str()) != Some("json") {
                                continue;
                            }
                            let Some(name) = src.file_name() else {
                                continue;
                            };
                            let dst = watch.join(name);
                            if !dst.exists() {
                                std::fs::copy(&src, &dst)
                                    .with_context(|| format!("failed to copy {} -> {}", src.display(), dst.display()))?;
                            }
                        }
                    }
                }

                let registry_state: Option<RegistryState> = if let Some(registry_path) = &registry {
                    if registry_path.exists() {
                        Some(read_json(registry_path)?)
                    } else {
                        None
                    }
                } else {
                    let mut merged: Option<RegistryState> = None;
                    for peer_path in &peer_registry {
                        let peer: RegistryState = if peer_path.to_string_lossy().starts_with("http://") || peer_path.to_string_lossy().starts_with("https://") {
                            let body = ureq::get(&peer_path.to_string_lossy()).call()
                                .map_err(|e| anyhow::anyhow!("failed to GET {}: {}", peer_path.display(), e))?
                                .into_string()
                                .map_err(|e| anyhow::anyhow!("failed to read HTTP body {}: {}", peer_path.display(), e))?;
                            serde_json::from_str(&body)
                                .map_err(|e| anyhow::anyhow!("failed to parse HTTP registry {}: {}", peer_path.display(), e))?
                        } else {
                            if !peer_path.exists() {
                                continue;
                            }
                            read_json(peer_path)?
                        };
                        match &mut merged {
                            None => merged = Some(peer),
                            Some(acc) => {
                                for (k, v) in peer.entries {
                                    match acc.entries.get(&k) {
                                        Some(existing) if existing.event_hash <= v.event_hash => {}
                                        _ => {
                                            acc.entries.insert(k, v);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    merged
                };

                for entry in std::fs::read_dir(&watch)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("json") {
                        continue;
                    }
                    let key = path.to_string_lossy().to_string();
                    if processed.contains(&key) {
                        continue;
                    }

                    let event: Event = read_json(&path)?;

                    if let Some(reg) = &registry_state {
                        let effect_kind = match &event {
                            Event::Deposit(_) => "deposit",
                            Event::Transfer(_) => "transfer",
                        };
                        let ck = conflict_key(&event);
                        let eh = event_hash(&event).0;
                        let map_key = format!("{}:{}", effect_kind, ck);

                        match reg.entries.get(&map_key) {
                            Some(entry) if entry.event_hash == eh => {}
                            Some(_) => {
                                println!("Skipped (non-canonical): {}", path.display());
                                // moved into success/rejection branches
                                continue;
                            }
                            None => {
                                println!("Skipped (not registered): {}", path.display());
                                // moved into success/rejection branches
                                continue;
                            }
                        }
                    }

                    let event: Event = event;
                    match fd_core::apply_event(&store, &manifest, &state, &event) {
                        Ok(result) => {
                            state = result.state;

                            let receipt_path = receipts.join(format!("{}.json", result.receipt.run_id.replace("ahd1024:", "")));
                            std::fs::write(&receipt_path, serde_json::to_string_pretty(&result.receipt)?)
                                .with_context(|| format!("failed to write {}", receipt_path.display()))?;

                            std::fs::write(&state_out, serde_json::to_string_pretty(&state)?)
                                .with_context(|| format!("failed to write {}", state_out.display()))?;

                            println!("Applied: {}", path.display());
                            println!("Run ID: {}", result.receipt.run_id);

                            processed.insert(key);
                            std::fs::write(&processed_path, serde_json::to_string_pretty(&processed)?)
                                .with_context(|| format!("failed to write {}", processed_path.display()))?;
                        }
                        Err(err) => {
                            if err.to_string().contains("insufficient balance") {
                                println!("Deferred: {}", path.display());
                                println!("Error: {}", err);
                                continue;
                            }

                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("event");
                            let error_path = errors_dir.join(format!("{}.error.json", stem));
                            let error_doc = serde_json::json!({
                                "event_path": path,
                                "error": err.to_string(),
                            });
                            std::fs::write(&error_path, serde_json::to_string_pretty(&error_doc)?)
                                .with_context(|| format!("failed to write {}", error_path.display()))?;
                            println!("Rejected: {}", path.display());
                            println!("Error: {}", err);

                            processed.insert(key);
                            std::fs::write(&processed_path, serde_json::to_string_pretty(&processed)?)
                                .with_context(|| format!("failed to write {}", processed_path.display()))?;
                        }
                    }
                }

                thread::sleep(Duration::from_millis(500));
            }
        }
        Command::FdRegistry { watch, registry_out, peer_registry } => {
            let processed_path = registry_out.with_extension("processed.json");
            let mut processed: std::collections::BTreeSet<String> = if processed_path.exists() {
                read_json(&processed_path)?
            } else {
                std::collections::BTreeSet::new()
            };
            let mut registry: RegistryState = if registry_out.exists() {
                read_json(&registry_out)?
            } else {
                RegistryState { entries: BTreeMap::new() }
            };

            loop {
                let mut changed = false;
                let mut events = Vec::new();

                for peer_path in &peer_registry {
                    let peer: RegistryState = if peer_path.to_string_lossy().starts_with("http://") || peer_path.to_string_lossy().starts_with("https://") {
                        let body = ureq::get(&peer_path.to_string_lossy()).call()
                            .map_err(|e| anyhow::anyhow!("failed to GET {}: {}", peer_path.display(), e))?
                            .into_string()
                            .map_err(|e| anyhow::anyhow!("failed to read HTTP body {}: {}", peer_path.display(), e))?;
                        serde_json::from_str(&body)
                            .map_err(|e| anyhow::anyhow!("failed to parse HTTP registry {}: {}", peer_path.display(), e))?
                    } else {
                        if !peer_path.exists() {
                            continue;
                        }
                        read_json(peer_path)?
                    };
                    for (k, v) in peer.entries {
                        match registry.entries.get(&k) {
                            Some(existing) if existing.event_hash <= v.event_hash => {}
                            _ => {
                                registry.entries.insert(k, v);
                                changed = true;
                            }
                        }
                    }
                }

                for entry in std::fs::read_dir(&watch)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("json") {
                        continue;
                    }
                    let key = path.to_string_lossy().to_string();
                    if processed.contains(&key) {
                        continue;
                    }
                    let event: Event = read_json(&path)?;
                    events.push((key, event));
                }

                for (key, event) in events {
                    let effect_kind = match &event {
                        Event::Deposit(_) => "deposit".to_string(),
                        Event::Transfer(_) => "transfer".to_string(),
                    };
                    let ck = conflict_key(&event);
                    let eh = event_hash(&event).0;
                    let map_key = format!("{}:{}", effect_kind, ck);
                    let prior = registry.entries.get(&map_key).cloned();
                    let should_update = match &prior {
                        Some(existing) => eh < existing.event_hash,
                        None => true,
                    };
                    if should_update {
                        registry.entries.insert(map_key, RegistryEntry {
                            effect_kind,
                            conflict_key: ck,
                            event_hash: eh.clone(),
                            run_id: String::new(),
                        });
                        changed = true;
                        match prior {
                            None => println!("Registry accepted: {}", eh),
                            Some(existing) => println!("Registry replaced: {} -> {}", existing.event_hash, eh),
                        }
                    } else {
                        println!("Registry ignored: {}", eh);
                    }
                    processed.insert(key);
                }

                if changed {
                    std::fs::write(&registry_out, serde_json::to_string_pretty(&registry)?)
                        .with_context(|| format!("failed to write {}", registry_out.display()))?;
                }
                std::fs::write(&processed_path, serde_json::to_string_pretty(&processed)?)
                    .with_context(|| format!("failed to write {}", processed_path.display()))?;

                thread::sleep(Duration::from_millis(500));
            }
        }


        Command::FdHttp { bind, registry, state, ingest_dir, receipts_dir, objects_dir } => {
            let server = Server::http(&bind)
                .map_err(|e| anyhow::anyhow!("failed to bind {}: {}", bind, e))?;
            println!("HTTP listening on {}", bind);

            for mut request in server.incoming_requests() {
                let url = request.url().to_string();
                let response = if (url == "/ingest" || url == "/v1/events") && request.method() == &tiny_http::Method::Post {
                    let mut content = String::new();
                    request.as_reader().read_to_string(&mut content)
                        .map_err(|e| anyhow::anyhow!("failed to read body: {}", e))?;

                    let event: Event = serde_json::from_str(&content)
                        .map_err(|e| anyhow::anyhow!("invalid event JSON: {}", e))?;

                    if let Some(dir) = &ingest_dir {
                        std::fs::create_dir_all(dir)
                            .map_err(|e| anyhow::anyhow!("failed to create {}: {}", dir.display(), e))?;

                        let fname = format!("{}.json", event_hash(&event).0.replace(":", "_"));
                        let out = dir.join(fname);

                        std::fs::write(&out, content)
                            .map_err(|e| anyhow::anyhow!("failed to write {}: {}", out.display(), e))?;

                        Response::from_string("ok")
                    } else {
                        Response::from_string("no ingest dir configured").with_status_code(400)
                    }
                } else if url == "/registry" || url == "/v1/registry" {
                    if let Some(path) = &registry {
                        if path.exists() {
                            let body = std::fs::read_to_string(path)?;
                            Response::from_string(body)
                        } else {
                            Response::from_string("null")
                        }
                    } else {
                        Response::from_string("null")
                    }
                } else if url == "/state" || url == "/v1/state" {
                    if let Some(path) = &state {
                        if path.exists() {
                            let body = std::fs::read_to_string(path)?;
                            Response::from_string(body)
                        } else {
                            Response::from_string("null")
                        }
                    } else {
                        Response::from_string("null")
                    }
                } else if url == "/info" || url == "/v1/info" {
                    let body = serde_json::json!({
                        "name": "fd-http",
                        "version": env!("CARGO_PKG_VERSION"),
                        "registry": registry.is_some(),
                        "state": state.is_some(),
                        "ingest_dir": ingest_dir.as_ref().map(|p| p.display().to_string()),
                        "receipts_dir": receipts_dir.as_ref().map(|p| p.display().to_string()),
                        "objects_dir": objects_dir.as_ref().map(|p| p.display().to_string()),
                    });
                    Response::from_string(serde_json::to_string_pretty(&body)?)
                } else if let Some(hash) = url.strip_prefix("/objects/").or_else(|| url.strip_prefix("/v1/objects/")) {
                    if let Some(dir) = &objects_dir {
                        let file = dir.join(hash.replace("ahd1024:", ""));
                        if file.exists() {
                            let body = std::fs::read_to_string(file)?;
                            Response::from_string(body)
                        } else {
                            Response::from_string("not found").with_status_code(404)
                        }
                    } else {
                        Response::from_string("objects not configured").with_status_code(400)
                    }
                } else if let Some(hash) = url.strip_prefix("/receipts/").or_else(|| url.strip_prefix("/v1/receipts/")) {
                    if let Some(dir) = &receipts_dir {
                        let file = dir.join(format!("{}.json", hash.replace("ahd1024:", "")));
                        if file.exists() {
                            let body = std::fs::read_to_string(file)?;
                            Response::from_string(body)
                        } else {
                            Response::from_string("not found").with_status_code(404)
                        }
                    } else {
                        Response::from_string("receipts not configured").with_status_code(400)
                    }
                } else {
                    Response::from_string("not found").with_status_code(404)
                };

                let _ = request.respond(response);
            }
        }

        Command::WalletGen { out } => {
            let mut secret = [0u8; 32];
            let mut urandom = std::fs::File::open("/dev/urandom")
                .context("failed to open /dev/urandom")?;
            urandom
                .read_exact(&mut secret)
                .context("failed to read 32 random bytes from /dev/urandom")?;
            let secret_hex = hex::encode(secret);
            let signing_key = signing_key_from_secret_hex(&secret_hex)?;
            let public_key = hex::encode(signing_key.verifying_key().to_bytes());
            let wallet = serde_json::json!({
                "secret_key_hex": secret_hex,
                "public_key_hex": public_key,
            });
            std::fs::write(&out, serde_json::to_string_pretty(&wallet)?)
                .with_context(|| format!("failed to write {}", out.display()))?;
            print_json(&wallet)?;
        }
        Command::WalletShow { secret } => {
            let wallet: serde_json::Value = read_json(&secret)?;
            let secret_hex = wallet
                .get("secret_key_hex")
                .and_then(|v| v.as_str())
                .context("missing secret_key_hex")?;
            let signing_key = signing_key_from_secret_hex(secret_hex)?;
            let public_key = hex::encode(signing_key.verifying_key().to_bytes());
            println!("Public key: {}", public_key);
        }
        Command::WalletSignTransfer { secret, to, amount, nonce, out } => {
            let wallet: serde_json::Value = read_json(&secret)?;
            let secret_hex = wallet
                .get("secret_key_hex")
                .and_then(|v| v.as_str())
                .context("missing secret_key_hex")?;
            let signing_key = signing_key_from_secret_hex(secret_hex)?;
            let from_key = hex::encode(signing_key.verifying_key().to_bytes());
            let mut tx = TransferIntent {
                kind: "transfer".to_string(),
                from_key,
                to_key: to,
                amount,
                nonce,
                signature: String::new(),
            };
            let sig = signing_key.sign(&transfer_signing_payload(&tx));
            tx.signature = hex::encode(sig.to_bytes());
            std::fs::write(&out, serde_json::to_string_pretty(&tx)?)
                .with_context(|| format!("failed to write {}", out.display()))?;
            print_json(&tx)?;
        }
        Command::WalletSignDeposit { secret, beneficiary, usd_cents, external_ref, timestamp, out } => {
            let wallet: serde_json::Value = read_json(&secret)?;
            let secret_hex = wallet
                .get("secret_key_hex")
                .and_then(|v| v.as_str())
                .context("missing secret_key_hex")?;
            let signing_key = signing_key_from_secret_hex(secret_hex)?;
            let oracle_id = hex::encode(signing_key.verifying_key().to_bytes());
            let mut dep = DepositAttestation {
                kind: "deposit".to_string(),
                oracle_id,
                usd_cents,
                beneficiary,
                external_ref,
                timestamp,
                signature: String::new(),
            };
            let sig = signing_key.sign(&deposit_signing_payload(&dep));
            dep.signature = hex::encode(sig.to_bytes());
            std::fs::write(&out, serde_json::to_string_pretty(&dep)?)
                .with_context(|| format!("failed to write {}", out.display()))?;
            print_json(&dep)?;
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

fn signing_key_from_secret_hex(secret_hex: &str) -> Result<SigningKey> {
    let bytes = hex::decode(secret_hex).context("invalid secret key hex")?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("secret key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
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
