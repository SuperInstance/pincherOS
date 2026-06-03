//! PincherOS CLI — the post-model operating system command-line interface.
//!
//! A hermit crab finds the right shell for every situation.
//! PincherOS finds the right reflex for every intent.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use pincher_core::{DbStats, Embedder, Reflex, ReflexEngine, ShellFingerprint};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── CLI Definition ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "pincher",
    about = "PincherOS — the post-model operating system",
    version = VERSION,
    after_help = "A hermit crab finds the right shell for every situation."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Database path (default: ~/.pincher/reflexes.db)
    #[arg(long, env = "PINCHER_DB", global = true)]
    db: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "warn", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current shell fingerprint, DB stats, resource state
    Status,

    /// Interactive teach flow: prompt for intent + action, store reflex
    Teach {
        /// Intent to teach (skip interactive prompt)
        #[arg(short, long)]
        intent: Option<String>,

        /// Action to associate (skip interactive prompt)
        #[arg(short, long)]
        action: Option<String>,
    },

    /// Execute a natural language intent through the reflex engine
    Do {
        /// Natural language intent to execute
        intent: String,
    },

    /// Show what would match without executing
    Match {
        /// Natural language intent to match
        intent: String,
    },

    /// Pack current state into .nail file for migration
    Pack {
        /// Output file path (default: pincher-state.nail)
        output: Option<PathBuf>,
    },

    /// Unpack .nail file and merge state
    Unpack {
        /// Path to .nail file to import
        nail: PathBuf,
    },

    /// Run benchmark: embed latency, match latency, teach latency
    Bench,

    /// Detailed hardware fingerprint
    ShellInfo,

    /// List all stored reflexes with confidence scores
    Reflexes {
        /// Show detailed information for each reflex
        #[arg(long)]
        verbose: bool,
    },

    /// Start JSON-RPC server for Python sidecar
    Rpc {
        /// Port to listen on
        #[arg(long, default_value = "9876")]
        port: u16,
    },
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(format!("{}{}", home, &path[1..]));
        }
    }
    PathBuf::from(path)
}

fn default_db_path() -> String {
    format!(
        "{}/.pincher/reflexes.db",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}

fn init_tracing(level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .with_target(false)
        .init();
}

async fn create_engine(db_path: &str) -> Result<ReflexEngine> {
    let path = expand_tilde(db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let embedder = Embedder::new()?;
    let engine = ReflexEngine::new(&path, embedder)?;
    Ok(engine)
}

fn prompt(prompt_text: &str) -> Result<String> {
    print!("{}", prompt_text);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn print_crab_status(fingerprint: &ShellFingerprint, stats: &DbStats) {
    println!();
    println!(
        "{}",
        format!("   🦀 PincherOS v{}", VERSION).bright_red().bold()
    );
    println!("{}", "  ╱╱╱╱╱╱╱╱╱╱╱╱╱".yellow());
    println!(
        "{} {}",
        " ╱".yellow(),
        format!("Shell: {}  ╲", fingerprint.hostname).green()
    );
    println!(
        "{} {}",
        "╱".yellow(),
        format!("   Reflexes: {}    ╲", stats.reflex_count).green()
    );
    println!(
        "{} {}",
        "╲".yellow(),
        "   State: Normal        ╱".green()
    );
    println!(
        "{} {}",
        " ╲".yellow(),
        format!("  RAM: {:.1}%       ╱", fingerprint.ram_usage_percent).green()
    );
    println!("{}", "  ╰────────────────────╯".yellow());
    println!();
}

fn print_timing(label: &str, duration: std::time::Duration) {
    let ms = duration.as_secs_f64() * 1000.0;
    println!(
        "  {} {}",
        format!("{}:", label).dimmed(),
        format!("{:.2}ms", ms).cyan()
    );
}

// ─── Command Implementations ─────────────────────────────────────────────────

async fn cmd_status(engine: &ReflexEngine) -> Result<()> {
    let fingerprint = engine.shell_fingerprint();
    let stats = engine.db_stats();

    print_crab_status(&fingerprint, &stats);

    println!("  {} {}", "OS:".dimmed(), fingerprint.os);
    println!("  {} {}", "Arch:".dimmed(), fingerprint.arch);
    println!("  {} {}", "CPU Cores:".dimmed(), fingerprint.cpu_cores);
    println!(
        "  {} {}",
        "Total RAM:".dimmed(),
        format!("{:.1} GB", fingerprint.total_ram_gb)
    );
    println!(
        "  {} {}",
        "DB Size:".dimmed(),
        format!("{:.2} KB", stats.db_size_bytes as f64 / 1024.0)
    );
    println!(
        "  {} {}",
        "Embed Dim:".dimmed(),
        engine.embedder().dimension()
    );

    Ok(())
}

async fn cmd_teach(engine: &ReflexEngine, intent: Option<String>, action: Option<String>) -> Result<()> {
    println!("\n{}", "🦀 Teach PincherOS a new reflex".bright_red().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    let intent = match intent {
        Some(i) => i,
        None => {
            let i = prompt(&format!(
                "{} ",
                "What intent should PincherOS learn?".green()
            ))?;
            if i.is_empty() {
                anyhow::bail!("Intent cannot be empty");
            }
            i
        }
    };

    let action = match action {
        Some(a) => a,
        None => {
            let a = prompt(&format!(
                "{} ",
                "What action should be taken?".green()
            ))?;
            if a.is_empty() {
                anyhow::bail!("Action cannot be empty");
            }
            a
        }
    };

    println!(
        "\n  {} Storing reflex...",
        "⏳".to_string().yellow()
    );

    let start = Instant::now();
    let reflex = engine.teach(&intent, &action).await?;
    let elapsed = start.elapsed();

    println!(
        "\n  {} {}",
        "✓".to_string().green(),
        "Reflex stored!".green().bold()
    );
    println!("  {} {}", "  ID:".dimmed(), reflex.id);
    println!("  {} {}", "  Intent:".dimmed(), reflex.intent);
    println!("  {} {}", "  Action:".dimmed(), reflex.action);
    println!(
        "  {} {}",
        "  Confidence:".dimmed(),
        format!("{:.2}", reflex.confidence)
    );
    println!("  {} {}", "  Created:".dimmed(), reflex.created_at);
    print_timing("  Time", elapsed);

    Ok(())
}

async fn cmd_do(engine: &ReflexEngine, intent: &str) -> Result<()> {
    println!("\n{}", format!("🦀 Executing: \"{}\"", intent).bright_red().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    let start = Instant::now();
    let result = engine.do_command(intent).await?;
    let elapsed = start.elapsed();

    match result {
        Some(action) => {
            println!(
                "\n  {} {}",
                "✓".to_string().green(),
                "Matched reflex:".green().bold()
            );
            println!("  {} {}", "  Action:".dimmed(), action.cyan());
            print_timing("  Time", elapsed);
        }
        None => {
            println!(
                "\n  {} {}",
                "✗".to_string().red(),
                "No matching reflex found".red().bold()
            );
            println!(
                "  {}",
                "  Try 'pincher teach' to add one.".dimmed()
            );
            print_timing("  Time", elapsed);
        }
    }

    Ok(())
}

async fn cmd_match(engine: &ReflexEngine, intent: &str) -> Result<()> {
    println!(
        "\n{}",
        format!("🦀 Matching: \"{}\"", intent).bright_red().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );
    println!(
        "  {} {}",
        "ℹ".to_string().blue(),
        "Dry run — no execution".blue()
    );

    let start = Instant::now();
    let result = engine.match_reflex(intent).await?;
    let elapsed = start.elapsed();

    match result {
        Some(reflex) => {
            println!(
                "\n  {} {}",
                "✓".to_string().green(),
                "Would match:".green().bold()
            );
            println!("  {} {}", "  Intent:".dimmed(), reflex.intent);
            println!("  {} {}", "  Action:".dimmed(), reflex.action.cyan());
            println!(
                "  {} {}",
                "  Confidence:".dimmed(),
                format!("{:.2}", reflex.confidence)
            );
            println!(
                "  {} {}",
                "  Invoked:".dimmed(),
                reflex.invoked_count
            );
            print_timing("  Time", elapsed);
        }
        None => {
            println!(
                "\n  {} {}",
                "✗".to_string().red(),
                "No matching reflex".red().bold()
            );
            print_timing("  Time", elapsed);
        }
    }

    Ok(())
}

async fn cmd_pack(engine: &ReflexEngine, output: Option<PathBuf>) -> Result<()> {
    let output_path = output.unwrap_or_else(|| PathBuf::from("pincher-state.nail"));

    println!(
        "\n{}",
        format!("🦀 Packing state to {}", output_path.display()).bright_red().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    let start = Instant::now();
    engine.pack(&output_path)?;
    let elapsed = start.elapsed();

    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    println!(
        "\n  {} {}",
        "✓".to_string().green(),
        "State packed!".green().bold()
    );
    println!("  {} {}", "  File:".dimmed(), output_path.display());
    println!(
        "  {} {}",
        "  Size:".dimmed(),
        format!("{:.2} KB", file_size as f64 / 1024.0)
    );
    print_timing("  Time", elapsed);

    Ok(())
}

async fn cmd_unpack(engine: &ReflexEngine, nail_path: &PathBuf) -> Result<()> {
    println!(
        "\n{}",
        format!("🦀 Unpacking from {}", nail_path.display()).bright_red().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    let start = Instant::now();
    engine.unpack(nail_path).await?;
    let elapsed = start.elapsed();

    let count = engine.reflex_count()?;

    println!(
        "\n  {} {}",
        "✓".to_string().green(),
        "State unpacked and merged!".green().bold()
    );
    println!("  {} {}", "  Total reflexes:".dimmed(), count);
    print_timing("  Time", elapsed);

    Ok(())
}

async fn cmd_bench(engine: &ReflexEngine) -> Result<()> {
    println!(
        "\n{}",
        "🦀 PincherOS Benchmark Suite".bright_red().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    let embedder = engine.embedder();

    // 1. Single embed latency
    print!("  {} Embedding single text...", "⏳".to_string().yellow());
    let start = Instant::now();
    embedder.embed("hello world").await?;
    let single_embed = start.elapsed();
    println!(
        " {}",
        format!("{:.2}ms", single_embed.as_secs_f64() * 1000.0).cyan()
    );

    // 2. Batch embed (10x) latency
    print!(
        "  {} Embedding batch (10x)...",
        "⏳".to_string().yellow()
    );
    let texts: Vec<&str> = (0..10)
        .map(|i| {
            match i {
                0 => "open the browser",
                1 => "show me the files",
                2 => "connect to server",
                3 => "list all processes",
                4 => "kill the daemon",
                5 => "start the service",
                6 => "check disk usage",
                7 => "display network stats",
                8 => "update the system",
                9 => "clean temp files",
                _ => "unknown",
            }
        })
        .collect();
    let start = Instant::now();
    embedder.embed_batch(&texts).await?;
    let batch_embed = start.elapsed();
    println!(
        " {}",
        format!("{:.2}ms", batch_embed.as_secs_f64() * 1000.0).cyan()
    );

    // 3. Teach latency (with seeding)
    print!(
        "  {} Seeding 10 reflexes...",
        "⏳".to_string().yellow()
    );
    let seed_intents = [
        ("open browser", "xdg-open https://example.com"),
        ("list files", "ls -la"),
        ("show processes", "ps aux"),
        ("check memory", "free -h"),
        ("disk usage", "df -h"),
        ("network status", "ip addr show"),
        ("git status", "git status"),
        ("build project", "cargo build --release"),
        ("run tests", "cargo test"),
        ("deploy app", "kubectl apply -f deploy.yaml"),
    ];
    let start = Instant::now();
    for (intent, action) in &seed_intents {
        engine.teach(intent, action).await?;
    }
    let teach_total = start.elapsed();
    let teach_avg = teach_total / 10;
    println!(
        " {}",
        format!("{:.2}ms avg", teach_avg.as_secs_f64() * 1000.0).cyan()
    );

    // 4. Match latency
    print!("  {} Matching reflex...", "⏳".to_string().yellow());
    let start = Instant::now();
    let match_count = 100;
    for _ in 0..match_count {
        let _ = engine.match_reflex("show me the files").await?;
    }
    let match_total = start.elapsed();
    let match_avg = match_total / match_count;
    println!(
        " {}",
        format!(
            "{:.2}ms avg ({} iterations)",
            match_avg.as_secs_f64() * 1000.0,
            match_count
        )
        .cyan()
    );

    // 5. Full do_command latency
    print!(
        "  {} Full do_command...",
        "⏳".to_string().yellow()
    );
    let start = Instant::now();
    let do_count = 50;
    for _ in 0..do_count {
        let _ = engine.do_command("check memory").await?;
    }
    let do_total = start.elapsed();
    let do_avg = do_total / do_count;
    println!(
        " {}",
        format!(
            "{:.2}ms avg ({} iterations)",
            do_avg.as_secs_f64() * 1000.0,
            do_count
        )
        .cyan()
    );

    // Summary table
    println!();
    println!(
        "{}",
        "  ┌─────────────────────────┬──────────────┬───────┐".dimmed()
    );
    println!(
        "{}",
        "  │ Benchmark               │ Latency      │ Status│".dimmed()
    );
    println!(
        "{}",
        "  ├─────────────────────────┼──────────────┼───────┤".dimmed()
    );

    let single_pass = single_embed.as_secs_f64() * 1000.0 < 50.0;
    let batch_pass = batch_embed.as_secs_f64() * 1000.0 < 500.0;
    let teach_pass = teach_avg.as_secs_f64() * 1000.0 < 50.0;
    let match_pass = match_avg.as_secs_f64() * 1000.0 < 50.0;
    let do_pass = do_avg.as_secs_f64() * 1000.0 < 50.0;

    for (label, latency_ms, pass) in [
        ("Single Embed", single_embed.as_secs_f64() * 1000.0, single_pass),
        ("Batch Embed (10x)", batch_embed.as_secs_f64() * 1000.0, batch_pass),
        ("Teach (avg)", teach_avg.as_secs_f64() * 1000.0, teach_pass),
        ("Match (avg)", match_avg.as_secs_f64() * 1000.0, match_pass),
        ("Do Command (avg)", do_avg.as_secs_f64() * 1000.0, do_pass),
    ] {
        let status = if pass {
            "  PASS".green()
        } else {
            "  FAIL".red()
        };
        println!(
            "  {} {} {} {}",
            "│".dimmed(),
            format!("{:<23}", label),
            format!("{:>10.2}ms", latency_ms),
            format!("{} {}", "│", status),
        );
    }

    println!(
        "{}",
        "  └─────────────────────────┴──────────────┴───────┘".dimmed()
    );

    let all_pass = single_pass && batch_pass && teach_pass && match_pass && do_pass;
    if all_pass {
        println!(
            "\n  {} {}",
            "✓".to_string().green(),
            "All benchmarks passed!".green().bold()
        );
    } else {
        println!(
            "\n  {} {}",
            "⚠".to_string().yellow(),
            "Some benchmarks exceeded thresholds".yellow().bold()
        );
    }

    Ok(())
}

async fn cmd_shell_info(engine: &ReflexEngine) -> Result<()> {
    let fp = engine.shell_fingerprint();

    println!("\n{}", "🦀 Shell Fingerprint".bright_red().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    println!("\n  {} Hardware", "━━".cyan().bold());
    println!("  {} {}", "Hostname:".dimmed(), fp.hostname);
    println!("  {} {}", "OS:".dimmed(), fp.os);
    println!("  {} {}", "Architecture:".dimmed(), fp.arch);
    println!("  {} {}", "CPU Cores:".dimmed(), fp.cpu_cores);
    println!(
        "  {} {}",
        "Total RAM:".dimmed(),
        format!("{:.2} GB", fp.total_ram_gb)
    );
    println!(
        "  {} {}",
        "RAM Usage:".dimmed(),
        format!("{:.1}%", fp.ram_usage_percent)
    );

    println!("\n  {} Runtime", "━━".cyan().bold());
    println!("  {} {}", "Embedding Dim:".dimmed(), engine.embedder().dimension());
    println!("  {} {}", "DB Path:".dimmed(), default_db_path());

    let stats = engine.db_stats();
    println!("  {} {}", "Reflexes:".dimmed(), stats.reflex_count);
    println!(
        "  {} {}",
        "DB Size:".dimmed(),
        format!("{:.2} KB", stats.db_size_bytes as f64 / 1024.0)
    );

    println!("\n  {} Environment", "━━".cyan().bold());
    println!("  {} {}", "PID:".dimmed(), std::process::id());
    println!(
        "  {} {}",
        "Working Dir:".dimmed(),
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    );
    println!(
        "  {} {}",
        "Rust Version:".dimmed(),
        "1.78.0"
    );

    Ok(())
}

async fn cmd_reflexes(engine: &ReflexEngine, verbose: bool) -> Result<()> {
    let reflexes = engine.list_reflexes()?;

    println!("\n{}", "🦀 Stored Reflexes".bright_red().bold());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );

    if reflexes.is_empty() {
        println!(
            "\n  {} No reflexes stored yet.",
            "∅".to_string().dimmed()
        );
        println!(
            "  {}",
            "  Use 'pincher teach' to add one.".dimmed()
        );
        return Ok(());
    }

    println!(
        "\n  {} {} reflexes found\n",
        "📋".to_string(),
        reflexes.len()
    );

    if verbose {
        for reflex in &reflexes {
            print_reflex_detail(reflex);
        }
    } else {
        // Compact table
        println!(
            "  {} {} {} {} {}",
            "ID".dimmed().to_string().pad_to_width(5),
            "Intent".dimmed().to_string().pad_to_width(25),
            "Action".dimmed().to_string().pad_to_width(25),
            "Confidence".dimmed().to_string().pad_to_width(12),
            "Invoked".dimmed()
        );
        println!(
            "  {}",
            "─────────────────────────────────────────────────────".dimmed()
        );

        for reflex in &reflexes {
            let confidence_bar = confidence_bar(reflex.confidence);
            println!(
                "  {:<5} {:<25} {:<25} {} {:>6}",
                reflex.id,
                truncate(&reflex.intent, 25),
                truncate(&reflex.action, 25),
                confidence_bar,
                reflex.invoked_count,
            );
        }
    }

    Ok(())
}

fn print_reflex_detail(reflex: &Reflex) {
    println!("  {} {}", "── Reflex #".dimmed(), reflex.id.to_string().cyan());
    println!("  {} {}", "    Intent:".dimmed(), reflex.intent);
    println!("  {} {}", "    Action:".dimmed(), reflex.action.cyan());
    println!(
        "  {} {}",
        "    Confidence:".dimmed(),
        format!("{:.4}", reflex.confidence)
    );
    println!("  {} {}", "    Created:".dimmed(), reflex.created_at);
    println!("  {} {}", "    Invoked:".dimmed(), reflex.invoked_count);
    println!();
}

fn confidence_bar(confidence: f64) -> String {
    let width = 10;
    let filled = (confidence * width as f64).round() as usize;
    let empty = width - filled;

    let bar: String = "█".repeat(filled);
    let space: String = "░".repeat(empty);

    if confidence >= 0.8 {
        format!("{}{} {}", bar.green(), space.dimmed(), format!("{:.2}", confidence).green())
    } else if confidence >= 0.5 {
        format!("{}{} {}", bar.yellow(), space.dimmed(), format!("{:.2}", confidence).yellow())
    } else {
        format!("{}{} {}", bar.red(), space.dimmed(), format!("{:.2}", confidence).red())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

async fn cmd_rpc(engine: &ReflexEngine, port: u16) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    println!(
        "\n{}",
        format!("🦀 PincherOS JSON-RPC Server").bright_red().bold()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    );
    println!(
        "\n  {} Listening on {}",
        "✓".to_string().green(),
        addr.cyan()
    );
    println!(
        "  {} {}",
        "ℹ".to_string().blue(),
        "Press Ctrl+C to stop".blue()
    );

    // We need to share the engine across connections.
    // Since ReflexEngine uses rusqlite::Connection which is not Send+Sync,
    // we'll handle requests sequentially for simplicity.
    loop {
        let (stream, addr) = listener.accept().await?;
        println!("  {} Client connected: {}", "→".to_string().dimmed(), addr);

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!(
                        "  {} Client disconnected: {}",
                        "←".to_string().dimmed(),
                        addr
                    );
                    break;
                }
                Ok(_) => {
                    let request = line.trim().to_string();
                    if request.is_empty() {
                        continue;
                    }

                    let response = handle_rpc_request(engine, &request).await;

                    let response_str = match serde_json::to_string(&response) {
                        Ok(s) => s,
                        Err(e) => serde_json::to_string(&serde_json::json!({
                            "jsonrpc": "2.0",
                            "error": {"code": -32603, "message": format!("Internal error: {}", e)},
                            "id": null
                        }))
                        .unwrap_or_else(|_| "{}".to_string()),
                    };

                    if let Err(e) = writer.write_all(format!("{}\n", response_str).as_bytes()).await {
                        println!("  {} Write error: {}", "✗".to_string().red(), e);
                        break;
                    }
                }
                Err(e) => {
                    println!("  {} Read error: {}", "✗".to_string().red(), e);
                    break;
                }
            }
        }
    }
}

async fn handle_rpc_request(engine: &ReflexEngine, request: &str) -> serde_json::Value {
    let msg: serde_json::Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32700, "message": format!("Parse error: {}", e)},
                "id": null
            });
        }
    };

    let id = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(serde_json::Value::Null);

    let result = match method {
        "status" => {
            let fp = engine.shell_fingerprint();
            let stats = engine.db_stats();
            serde_json::json!({
                "version": VERSION,
                "hostname": fp.hostname,
                "os": fp.os,
                "arch": fp.arch,
                "cpu_cores": fp.cpu_cores,
                "total_ram_gb": fp.total_ram_gb,
                "ram_usage_percent": fp.ram_usage_percent,
                "reflex_count": stats.reflex_count,
                "db_size_bytes": stats.db_size_bytes,
            })
        }
        "teach" => {
            let intent = params.get("intent").and_then(|p| p.as_str()).unwrap_or("");
            let action = params.get("action").and_then(|p| p.as_str()).unwrap_or("");
            if intent.is_empty() || action.is_empty() {
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32602, "message": "Missing 'intent' or 'action' params"},
                    "id": id
                });
            }
            match engine.teach(intent, action).await {
                Ok(reflex) => serde_json::json!({
                    "id": reflex.id,
                    "intent": reflex.intent,
                    "action": reflex.action,
                    "confidence": reflex.confidence,
                    "created_at": reflex.created_at,
                }),
                Err(e) => {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {"code": -32603, "message": format!("Teach error: {}", e)},
                        "id": id
                    });
                }
            }
        }
        "match" => {
            let intent = params.get("intent").and_then(|p| p.as_str()).unwrap_or("");
            if intent.is_empty() {
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32602, "message": "Missing 'intent' param"},
                    "id": id
                });
            }
            match engine.match_reflex(intent).await {
                Ok(Some(reflex)) => serde_json::json!({
                    "matched": true,
                    "reflex": {
                        "id": reflex.id,
                        "intent": reflex.intent,
                        "action": reflex.action,
                        "confidence": reflex.confidence,
                    }
                }),
                Ok(None) => serde_json::json!({"matched": false}),
                Err(e) => {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {"code": -32603, "message": format!("Match error: {}", e)},
                        "id": id
                    });
                }
            }
        }
        "do" => {
            let intent = params.get("intent").and_then(|p| p.as_str()).unwrap_or("");
            if intent.is_empty() {
                return serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32602, "message": "Missing 'intent' param"},
                    "id": id
                });
            }
            match engine.do_command(intent).await {
                Ok(Some(action)) => serde_json::json!({
                    "executed": true,
                    "action": action
                }),
                Ok(None) => serde_json::json!({"executed": false}),
                Err(e) => {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {"code": -32603, "message": format!("Do error: {}", e)},
                        "id": id
                    });
                }
            }
        }
        "list" => {
            match engine.list_reflexes() {
                Ok(reflexes) => {
                    let list: Vec<serde_json::Value> = reflexes.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "intent": r.intent,
                            "action": r.action,
                            "confidence": r.confidence,
                            "invoked_count": r.invoked_count,
                        })
                    }).collect();
                    serde_json::json!({"reflexes": list})
                }
                Err(e) => {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {"code": -32603, "message": format!("List error: {}", e)},
                        "id": id
                    });
                }
            }
        }
        _ => {
            return serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32601, "message": format!("Method not found: {}", method)},
                "id": id
            });
        }
    };

    serde_json::json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    })
}

// ─── Trait helper for padding ────────────────────────────────────────────────

trait PadToWidth {
    fn pad_to_width(&self, width: usize) -> String;
}

impl PadToWidth for String {
    fn pad_to_width(&self, width: usize) -> String {
        if self.len() >= width {
            self[..width].to_string()
        } else {
            let padding = width - self.chars().count();
            format!("{}{}", self, " ".repeat(padding))
        }
    }
}

// ─── Main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli.log_level);

    let db_path = cli.db.unwrap_or_else(default_db_path);
    let engine = create_engine(&db_path).await?;

    let overall_start = Instant::now();

    let is_long_running = matches!(cli.command, Commands::Rpc { .. } | Commands::Bench);

    let result = match cli.command {
        Commands::Status => cmd_status(&engine).await,
        Commands::Teach { intent, action } => cmd_teach(&engine, intent, action).await,
        Commands::Do { intent } => cmd_do(&engine, &intent).await,
        Commands::Match { intent } => cmd_match(&engine, &intent).await,
        Commands::Pack { output } => cmd_pack(&engine, output).await,
        Commands::Unpack { nail } => cmd_unpack(&engine, &nail).await,
        Commands::Bench => cmd_bench(&engine).await,
        Commands::ShellInfo => cmd_shell_info(&engine).await,
        Commands::Reflexes { verbose } => cmd_reflexes(&engine, verbose).await,
        Commands::Rpc { port } => cmd_rpc(&engine, port).await,
    };

    let elapsed = overall_start.elapsed();

    // Don't print timing for RPC (long-running) or Bench (has its own timing)
    if !is_long_running {
        println!(
            "\n  {} {}",
            "⏱".dimmed(),
            format!("Completed in {:.2}ms", elapsed.as_secs_f64() * 1000.0).dimmed()
        );
    }

    result
}
