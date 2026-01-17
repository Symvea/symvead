use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "symvead")]
#[command(about = "Symvea server operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the server
    Start,
    /// Show server status
    Status,
    /// Show corpus statistics
    Stats,
    /// Verify entire corpus integrity
    VerifyCorpus,
    /// Create a snapshot
    Snapshot,
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start => {
            // This is the normal server startup
            crate::main().await
        }
        Commands::Status => {
            show_status().await
        }
        Commands::Stats => {
            show_stats().await
        }
        Commands::VerifyCorpus => {
            verify_corpus().await
        }
        Commands::Snapshot => {
            create_snapshot().await
        }
    }
}

async fn show_status() -> Result<()> {
    println!("📊 Symvea Server Status");
    println!("=======================");
    
    let data_path = "./data";
    
    // Check if data directory exists
    if !std::path::Path::new(data_path).exists() {
        println!("❌ Status: Not initialized");
        println!("   Data directory not found: {}", data_path);
        return Ok(());
    }
    
    // Check STATE file
    let state_path = format!("{}/STATE", data_path);
    if std::path::Path::new(&state_path).exists() {
        let state = std::fs::read_to_string(&state_path)?;
        println!("✅ Status: {}", state.trim());
    } else {
        println!("⚠️  Status: Unknown (no STATE file)");
    }
    
    // Count symbols
    let symbols_dir = format!("{}/symbols", data_path);
    let symbol_count = if let Ok(entries) = std::fs::read_dir(&symbols_dir) {
        entries.filter_map(|e| e.ok()).filter(|e| {
            e.file_name().to_string_lossy().ends_with(".bin")
        }).count()
    } else {
        0
    };
    
    println!("   Symbols: {}", symbol_count);
    
    // Count files
    let files_dir = format!("{}/corpus/files", data_path);
    let file_count = if let Ok(entries) = std::fs::read_dir(&files_dir) {
        entries.filter_map(|e| e.ok()).filter(|e| {
            e.file_name().to_string_lossy().ends_with(".meta.json")
        }).count()
    } else {
        0
    };
    
    println!("   Files: {}", file_count);
    
    Ok(())
}

async fn show_stats() -> Result<()> {
    println!("📈 Corpus Statistics");
    println!("===================");
    
    let storage = crate::storage::PersistentStorage::new("./data")?;
    let (symbol_count, total_size) = storage.calculate_total_size().map(|size| (storage.count_symbols().unwrap_or(0), size)).unwrap_or((0, 0));
    
    println!("Total symbols: {}", symbol_count);
    println!("Total size: {} bytes ({:.1} KB)", total_size, total_size as f64 / 1024.0);
    
    if symbol_count > 0 {
        println!("Average symbol size: {} bytes", total_size / symbol_count);
    }
    
    Ok(())
}

async fn verify_corpus() -> Result<()> {
    println!("🔍 Verifying Corpus Integrity");
    println!("=============================");
    
    let validator = crate::startup::StartupValidator::new("./data")?;
    
    match validator.validate_and_start() {
        Ok(_) => {
            println!("✅ Corpus verification PASSED");
            println!("   All symbols verified");
            println!("   All metadata validated");
            println!("   All references consistent");
        }
        Err(e) => {
            println!("❌ Corpus verification FAILED");
            println!("   Error: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

async fn create_snapshot() -> Result<()> {
    println!("📸 Creating Snapshot");
    println!("===================");
    
    let snapshot_manager = crate::snapshot::SnapshotManager::new("./data");
    
    match snapshot_manager.create_snapshot() {
        Ok(snapshot) => {
            println!("✅ Snapshot created successfully");
            println!("   Epoch: {}", snapshot.epoch);
            println!("   Symbols: {}", snapshot.symbols.len());
            println!("   Files: {}", snapshot.files.len());
            println!("   Path: ./data/snapshots/snapshot_{}.json", snapshot.epoch);
        }
        Err(e) => {
            println!("❌ Snapshot creation failed");
            println!("   Error: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}