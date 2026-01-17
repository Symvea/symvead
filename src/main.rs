mod analytics;
mod proof;
mod server;
mod config;
mod startup;
mod snapshot;
mod session;
mod storage;
mod protocol;
mod utils;
mod engine;
mod coordination;
mod metrics;

use tracing::info;
use startup::StartupValidator;
use clap::Parser;
use config::ServerConfig;

#[derive(Parser)]
#[command(name = "symvead")]
#[command(about = "Symvea server operations")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(long, help = "Config file path")]
    config: Option<String>,
    
    #[arg(long, help = "Data directory path (overrides config)")]
    data: Option<String>,
    
    #[arg(long, help = "Listen address (overrides config)")]
    listen: Option<String>,
    
    #[arg(long, help = "Mount readonly corpus paths (overrides config)")]
    mount_readonly: Vec<String>,
    
    #[arg(long, help = "Output as JSON")]
    json: bool,
}

#[derive(clap::Subcommand)]
enum Commands {
    Status,
    Stats,
    VerifyCorpus,
    Snapshot,
    ListSnapshots,
    RestoreSnapshot {
        snapshot_file: String,
    },
    ListSymbols,
    FreezeDictionary,
    Symbol {
        #[command(subcommand)]
        symbol_cmd: SymbolCommands,
    },
    Analytics,
    Proof,
    Test,
    GenerateConfig {
        #[arg(long, default_value = "symvea.toml", help = "Config file path")]
        output: String,
    },
}

#[derive(clap::Subcommand)]
enum SymbolCommands {
    Inspect { id: String },
    Stability { id: String },
    Dominance { id: String },
    History { id: String },
    ListStability,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("symvead=trace,symvea=trace")
        .init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let mut config = ServerConfig::load_or_create(cli.config.as_deref())?;
    
    // Override config with CLI args if provided
    if let Some(data) = cli.data {
        config.data_directory = data.into();
    }
    if let Some(listen) = cli.listen {
        config.listen_address = listen;
    }
    if !cli.mount_readonly.is_empty() {
        config.readonly_mounts = cli.mount_readonly.into_iter().map(Into::into).collect();
    }
    
    // Ensure directories exist
    if let Err(e) = config.ensure_directories() {
        if cli.json {
            println!("{}", serde_json::json!({"error": format!("Failed to create directories: {}", e)}));
        } else {
            eprintln!("❌ Failed to create directories: {}", e);
        }
        return Err(e);
    }
    
    let data_dir = config.data_directory.to_string_lossy().to_string();
    let listen_addr = config.listen_address.clone();
    
    match cli.command {
        Some(Commands::Status) => {
            if cli.json {
                let status = if std::path::Path::new(&data_dir).exists() {
                    serde_json::json!({
                        "status": "ready",
                        "data_directory": data_dir,
                        "listen_address": listen_addr
                    })
                } else {
                    serde_json::json!({
                        "status": "not_initialized"
                    })
                };
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("📊 Symvea Server Status");
                println!("=======================");
                
                if !std::path::Path::new(&data_dir).exists() {
                    println!("❌ Status: Not initialized");
                    return Ok(());
                }
                
                println!("✅ Status: Ready");
                println!("   Data directory: {}", data_dir);
                println!("   Listen address: {}", listen_addr);
            }
            return Ok(());
        }
        Some(Commands::Stats) => {
            if cli.json {
                println!("{}", serde_json::json!({
                    "total_symbols": 0
                }));
            } else {
                println!("📈 Corpus Statistics");
                println!("===================");
                println!("Total symbols: 0");
            }
            return Ok(());
        }
        Some(Commands::VerifyCorpus) => {
            match StartupValidator::new(&data_dir) {
                Ok(validator) => {
                    match validator.validate_and_start() {
                        Ok(_) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"status": "passed"}));
                            } else {
                                println!("🔍 Verifying Corpus Integrity");
                                println!("=============================");
                                println!("✅ Corpus verification PASSED");
                            }
                        }
                        Err(e) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"error": e.to_string()}));
                            } else {
                                println!("🔍 Verifying Corpus Integrity");
                                println!("=============================");
                                println!("❌ Corpus verification FAILED: {}", e);
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("🔍 Verifying Corpus Integrity");
                        println!("=============================");
                        println!("❌ Validation setup failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::Snapshot) => {
            use crate::snapshot::SnapshotManager;
            
            let snapshot_manager = SnapshotManager::new(&data_dir);
            
            match snapshot_manager.create_snapshot() {
                Ok(snapshot) => {
                    if cli.json {
                        println!("{}", serde_json::json!({
                            "success": true,
                            "epoch": snapshot.epoch,
                            "symbols_count": snapshot.symbols.len(),
                            "files_count": snapshot.files.len()
                        }));
                    } else {
                        println!("📸 Creating Snapshot");
                        println!("===================");
                        println!("✅ Snapshot created successfully");
                        println!("   Epoch: {}", snapshot.epoch);
                        println!("   Symbols: {}", snapshot.symbols.len());
                        println!("   Files: {}", snapshot.files.len());
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("📸 Creating Snapshot");
                        println!("===================");
                        println!("❌ Snapshot creation failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::ListSnapshots) => {
            let snapshots_dir = format!("{}/snapshots", data_dir);
            
            if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
                let mut snapshots = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        let filename = entry.file_name();
                        let filename_str = filename.to_string_lossy();
                        if filename_str.starts_with("snapshot_") && filename_str.ends_with(".json") {
                            snapshots.push(filename_str.to_string());
                        }
                    }
                }
                
                snapshots.sort();
                if cli.json {
                    println!("{}", serde_json::json!({
                        "snapshots": snapshots,
                        "count": snapshots.len()
                    }));
                } else {
                    println!("📋 Available Snapshots");
                    println!("=====================");
                    if snapshots.is_empty() {
                        println!("   No snapshots found");
                    } else {
                        for snapshot in snapshots {
                            println!("   {}", snapshot);
                        }
                    }
                }
            } else {
                if cli.json {
                    println!("{}", serde_json::json!({"error": "No snapshots directory found"}));
                } else {
                    println!("📋 Available Snapshots");
                    println!("=====================");
                    println!("   No snapshots directory found");
                }
            }
            return Ok(());
        }
        Some(Commands::RestoreSnapshot { snapshot_file }) => {
            use crate::snapshot::SnapshotManager;
            
            let snapshot_manager = SnapshotManager::new(&data_dir);
            let snapshot_path = if snapshot_file.contains('/') {
                snapshot_file.clone()
            } else {
                format!("{}/snapshots/{}", data_dir, snapshot_file)
            };
            
            match snapshot_manager.restore_snapshot(&snapshot_path) {
                Ok(_) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"success": true, "snapshot_file": snapshot_file}));
                    } else {
                        println!("🔄 Restoring Snapshot");
                        println!("====================");
                        println!("✅ Snapshot restored successfully");
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("🔄 Restoring Snapshot");
                        println!("====================");
                        println!("❌ Snapshot restore failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::ListSymbols) => {
            use crate::storage::PersistentStorage;
            
            match PersistentStorage::new(&data_dir) {
                Ok(storage) => {
                    match storage.list_symbols() {
                        Ok(symbols) => {
                            if cli.json {
                                println!("{}", serde_json::json!({
                                    "total_symbols": symbols.len(),
                                    "symbols": symbols.iter().take(100).collect::<Vec<_>>()
                                }));
                            } else {
                                println!("📋 Symbol List");
                                println!("==============");
                                
                                if symbols.is_empty() {
                                    println!("   No symbols found");
                                } else {
                                    println!("   Total symbols: {}", symbols.len());
                                    for symbol in symbols.iter().take(20) {
                                        println!("   {}", symbol);
                                    }
                                    if symbols.len() > 20 {
                                        println!("   ... and {} more", symbols.len() - 20);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"error": e.to_string()}));
                            } else {
                                println!("❌ Failed to list symbols: {}", e);
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("❌ Storage initialization failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::FreezeDictionary) => {
            if cli.json {
                println!("{}", serde_json::json!({"message": "Use client command: symvea-client freeze-dictionary"}));
            } else {
                println!("🧊 Freezing Dictionary");
                println!("====================");
                println!("✅ Use client command: symvea-client freeze-dictionary");
            }
            return Ok(());
        }
        Some(Commands::Proof) => {
            use crate::proof::ProofVerifier;
            
            match ProofVerifier::new(&data_dir) {
                Ok(verifier) => {
                    match verifier.generate_proof_report() {
                        Ok(report) => {
                            if cli.json {
                                println!("{}", serde_json::json!({
                                    "total_symbols": report.total_symbols,
                                    "verified_symbols": report.verified_symbols,
                                    "integrity_score": report.integrity_score,
                                    "oldest_symbol_age_days": report.oldest_symbol_age_days,
                                    "append_only_verified": report.append_only_verified,
                                    "corrupted_symbols": report.corrupted_symbols
                                }));
                            } else {
                                println!("🔐 Data Provability Report");
                                println!("=========================");
                                println!("   Total symbols: {}", report.total_symbols);
                                println!("   Verified symbols: {}", report.verified_symbols);
                                println!("   Integrity score: {:.1}%", report.integrity_score);
                                println!("   Oldest data: {} days", report.oldest_symbol_age_days);
                                println!("   Append-only verified: {}", if report.append_only_verified { "✅" } else { "❌" });
                                
                                if !report.corrupted_symbols.is_empty() {
                                    println!("   ⚠️  Corrupted symbols: {:?}", report.corrupted_symbols);
                                }
                                
                                if report.integrity_score == 100.0 {
                                    println!("   🎉 All data cryptographically verified!");
                                }
                            }
                        }
                        Err(e) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"error": e.to_string()}));
                            } else {
                                println!("❌ Proof verification failed: {}", e);
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("❌ Proof verifier initialization failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::Analytics) => {
            use crate::analytics::PatternAnalytics;
            use crate::storage::PersistentStorage;
            
            match PersistentStorage::new(&data_dir) {
                Ok(storage) => {
                    match PatternAnalytics::analyze_corpus(&storage) {
                        Ok(analytics) => {
                            let insights = analytics.get_insights();
                            if cli.json {
                                println!("{}", serde_json::json!({"insights": insights}));
                            } else {
                                println!("📊 Pattern Analytics");
                                println!("==================");
                                for insight in insights {
                                    println!("   {}", insight);
                                }
                            }
                        }
                        Err(e) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"error": e.to_string()}));
                            } else {
                                println!("📊 Pattern Analytics");
                                println!("==================");
                                println!("❌ Analytics failed: {}", e);
                            }
                            return Err(anyhow::anyhow!("Analytics failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("📊 Pattern Analytics");
                        println!("==================");
                        println!("❌ Storage initialization failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::GenerateConfig { output }) => {
            let config = ServerConfig::default();
            match config.save(&output) {
                Ok(_) => {
                    if cli.json {
                        println!("{}", serde_json::json!({
                            "success": true,
                            "config_file": output,
                            "message": "Default configuration file created"
                        }));
                    } else {
                        println!("⚙️  Generate Configuration");
                        println!("========================");
                        println!("✅ Default configuration saved to: {}", output);
                        println!("   Edit the file to customize server settings");
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("⚙️  Generate Configuration");
                        println!("========================");
                        println!("❌ Failed to create config file: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::Test) => {
            use crate::storage::layered::LayeredStorage;
            
            if config.readonly_mounts.is_empty() {
                if cli.json {
                    println!("{}", serde_json::json!({"error": "No readonly mounts specified"}));
                } else {
                    println!("🧪 Testing Layered Storage");
                    println!("========================");
                    println!("❌ No readonly mounts specified");
                }
                return Ok(());
            }
            
            let mount_paths: Vec<String> = config.readonly_mounts.iter().map(|p| p.to_string_lossy().to_string()).collect();
            match LayeredStorage::new(&data_dir, &mount_paths) {
                Ok(layered) => {
                    match layered.list_symbols() {
                        Ok(symbols) => {
                            if cli.json {
                                println!("{}", serde_json::json!({
                                    "total_symbols": symbols.len(),
                                    "symbols": symbols.iter().take(10).collect::<Vec<_>>(),
                                    "readonly_mounts": mount_paths
                                }));
                            } else {
                                println!("🧪 Testing Layered Storage");
                                println!("========================");
                                println!("   Found {} symbols across all layers:", symbols.len());
                                for symbol in symbols.iter().take(10) {
                                    println!("     - {}", symbol);
                                }
                                if symbols.len() > 10 {
                                    println!("     ... and {} more", symbols.len() - 10);
                                }
                            }
                        }
                        Err(e) => {
                            if cli.json {
                                println!("{}", serde_json::json!({"error": e.to_string()}));
                            } else {
                                println!("🧪 Testing Layered Storage");
                                println!("========================");
                                println!("❌ Failed to list symbols: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({"error": e.to_string()}));
                    } else {
                        println!("🧪 Testing Layered Storage");
                        println!("========================");
                        println!("❌ Layered storage initialization failed: {}", e);
                    }
                    return Err(e);
                }
            }
            return Ok(());
        }
        Some(Commands::Symbol { symbol_cmd }) => {
            use crate::storage::PersistentStorage;
            
            match PersistentStorage::new(&data_dir) {
                Ok(storage) => {
                    match symbol_cmd {
                        SymbolCommands::Inspect { id } => {
                            // Find symbol by prefix if short ID provided
                            let symbols = storage.list_symbols()?;
                            let matching_symbol = if id.len() < 32 {
                                symbols.iter().find(|s| s.starts_with(&id)).cloned()
                            } else {
                                Some(id.clone())
                            };
                            
                            if let Some(symbol_hash) = matching_symbol {
                                match storage.load_symbol(&symbol_hash) {
                                    Ok(symbol) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({
                                                "symbol_id": id,
                                                "full_hash": symbol.hash,
                                                "size": symbol.size,
                                                "usage_count": symbol.usage_count,
                                                "first_seen": symbol.first_seen,
                                                "content_hash": format!("{:x?}", &symbol.content_hash[..8])
                                            }));
                                        } else {
                                            println!("🔍 Symbol Inspection: {}", id);
                                            println!("========================");
                                            println!("   Hash: {}", symbol.hash);
                                            println!("   Size: {} bytes", symbol.size);
                                            println!("   Usage count: {}", symbol.usage_count);
                                            println!("   First seen: {} (epoch)", symbol.first_seen);
                                            println!("   Content hash: {:x?}", &symbol.content_hash[..8]);
                                        }
                                    }
                                    Err(_) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                        } else {
                                            println!("🔍 Symbol Inspection: {}", id);
                                            println!("========================");
                                            println!("   Symbol not found");
                                        }
                                    }
                                }
                            } else {
                                if cli.json {
                                    println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                } else {
                                    println!("🔍 Symbol Inspection: {}", id);
                                    println!("========================");
                                    println!("   Symbol not found");
                                }
                            }
                        }
                        SymbolCommands::Stability { id } => {
                            let symbols = storage.list_symbols()?;
                            let matching_symbol = if id.len() < 32 {
                                symbols.iter().find(|s| s.starts_with(&id)).cloned()
                            } else {
                                Some(id.clone())
                            };
                            
                            if let Some(symbol_hash) = matching_symbol {
                                match storage.load_symbol(&symbol_hash) {
                                    Ok(symbol) => {
                                        let age_days = (std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap().as_secs() - symbol.first_seen) / 86400;
                                        let stability_score = symbol.usage_count as f64 / (age_days + 1) as f64;
                                        
                                        if cli.json {
                                            println!("{}", serde_json::json!({
                                                "symbol_id": id,
                                                "full_hash": symbol_hash,
                                                "age_days": age_days,
                                                "usage_count": symbol.usage_count,
                                                "stability_score": stability_score
                                            }));
                                        } else {
                                            println!("📊 Symbol Stability: {}", id);
                                            println!("=======================");
                                            println!("   Age: {} days", age_days);
                                            println!("   Usage count: {}", symbol.usage_count);
                                            println!("   Stability score: {:.2}", stability_score);
                                        }
                                    }
                                    Err(_) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                        } else {
                                            println!("📊 Symbol Stability: {}", id);
                                            println!("=======================");
                                            println!("   Symbol not found");
                                        }
                                    }
                                }
                            } else {
                                if cli.json {
                                    println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                } else {
                                    println!("📊 Symbol Stability: {}", id);
                                    println!("=======================");
                                    println!("   Symbol not found");
                                }
                            }
                        }
                        SymbolCommands::Dominance { id } => {
                            let symbols = storage.list_symbols()?;
                            let matching_symbol = if id.len() < 32 {
                                symbols.iter().find(|s| s.starts_with(&id)).cloned()
                            } else {
                                Some(id.clone())
                            };
                            
                            if let Some(symbol_hash) = matching_symbol {
                                match storage.get_symbol_usage(&symbol_hash) {
                                    Ok(usage) => {
                                        if cli.json {
                                            let result = serde_json::json!({
                                                "symbol_id": id,
                                                "full_hash": symbol_hash,
                                                "total_occurrences": usage.total_occurrences,
                                                "files_count": usage.objects.len(),
                                                "bytes_contributed": usage.total_bytes_contributed,
                                                "dominance_score": usage.total_occurrences * usage.objects.len() as u64,
                                                "files": usage.objects
                                            });
                                            println!("{}", serde_json::to_string_pretty(&result)?);
                                        } else {
                                            println!("🏆 Symbol Dominance: {}", id);
                                            println!("=====================");
                                            println!("   Total occurrences: {}", usage.total_occurrences);
                                            println!("   Files using symbol: {}", usage.objects.len());
                                            println!("   Bytes contributed: {}", usage.total_bytes_contributed);
                                            println!("   Dominance score: {}", usage.total_occurrences * usage.objects.len() as u64);
                                            println!("   Files:");
                                            for (file_key, count) in usage.objects.iter() {
                                                println!("     {} (used {} times)", file_key, count);
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                        } else {
                                            println!("   Symbol not found");
                                        }
                                    }
                                }
                            } else {
                                if cli.json {
                                    println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                } else {
                                    println!("   Symbol not found");
                                }
                            }
                        }
                        SymbolCommands::History { id } => {
                            let symbols = storage.list_symbols()?;
                            let matching_symbol = if id.len() < 32 {
                                symbols.iter().find(|s| s.starts_with(&id)).cloned()
                            } else {
                                Some(id.clone())
                            };
                            
                            if let Some(symbol_hash) = matching_symbol {
                                match storage.load_symbol(&symbol_hash) {
                                    Ok(symbol) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({
                                                "symbol_id": id,
                                                "full_hash": symbol_hash,
                                                "created": symbol.first_seen,
                                                "current_usage": symbol.usage_count,
                                                "content_hash": format!("{:x?}", &symbol.content_hash[..8]),
                                                "note": "Full versioning not enabled"
                                            }));
                                        } else {
                                            println!("📜 Symbol History: {}", id);
                                            println!("===================");
                                            println!("   Created: {} (epoch)", symbol.first_seen);
                                            println!("   Current usage: {}", symbol.usage_count);
                                            println!("   Content hash: {:x?}", &symbol.content_hash[..8]);
                                            println!("   Note: Full versioning not enabled");
                                        }
                                    }
                                    Err(_) => {
                                        if cli.json {
                                            println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                        } else {
                                            println!("📜 Symbol History: {}", id);
                                            println!("===================");
                                            println!("   Symbol not found");
                                        }
                                    }
                                }
                            } else {
                                if cli.json {
                                    println!("{}", serde_json::json!({"error": "Symbol not found"}));
                                } else {
                                    println!("📜 Symbol History: {}", id);
                                    println!("===================");
                                    println!("   Symbol not found");
                                }
                            }
                        }
                        SymbolCommands::ListStability => {
                            match storage.list_symbols() {
                                Ok(symbols) => {
                                    let mut stability_scores = Vec::new();
                                    let current_time = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap().as_secs();
                                    
                                    for symbol_hash in symbols.iter().take(100) { // Limit for performance
                                        if let Ok(symbol) = storage.load_symbol(symbol_hash) {
                                            let age_days = (current_time - symbol.first_seen) / 86400;
                                            let stability = symbol.usage_count as f64 / (age_days + 1) as f64;
                                            stability_scores.push((symbol_hash.clone(), stability));
                                        }
                                    }
                                    
                                    stability_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                                    
                                    if cli.json {
                                        let top_symbols: Vec<_> = stability_scores.iter().take(10).map(|(hash, score)| {
                                            serde_json::json!({
                                                "hash": hash,
                                                "short_hash": &hash[..8],
                                                "stability_score": score
                                            })
                                        }).collect();
                                        println!("{}", serde_json::json!({
                                            "total_analyzed": stability_scores.len(),
                                            "top_symbols": top_symbols
                                        }));
                                    } else {
                                        println!("📊 Symbol Stability Rankings");
                                        println!("=============================");
                                        for (i, (hash, score)) in stability_scores.iter().take(10).enumerate() {
                                            println!("   {}. {}... (score: {:.2})", i + 1, &hash[..8], score);
                                        }
                                    }
                                }
                                Err(_) => {
                                    if cli.json {
                                        println!("{}", serde_json::json!({"error": "No symbols found"}));
                                    } else {
                                        println!("📊 Symbol Stability Rankings");
                                        println!("=============================");
                                        println!("   No symbols found");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Storage initialization failed: {}", e);
                    return Err(e);
                }
            }
            return Ok(());
        }
        None => {
            // Normal server startup
            info!("Starting Symvea server on {}", listen_addr);
            
            if !config.readonly_mounts.is_empty() {
                info!("Readonly mounts: {:?}", config.readonly_mounts);
            }
            
            let validator = StartupValidator::new(&data_dir)?;
            validator.validate_and_start()?;
            
            server::run_on(&listen_addr, &data_dir).await
        }
    }
}
