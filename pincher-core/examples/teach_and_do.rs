//! Example: teach a reflex and then execute it.
//!
//! Run with: cargo run --example teach_and_do

use pincher_core::PincherOS;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create an in-memory PincherOS instance for the demo.
    let mut os = PincherOS::in_memory()?;

    // Seed built-in reflexes.
    os.seed_builtins()?;

    // Teach a custom reflex.
    let id = os
        .teach("make a folder", "mkdir -p {path}", None)
        .await?;
    println!("[TAUGHT] reflex {} — trigger: \"make a folder\", command: \"mkdir -p {{path}}\"", id);

    // Do a command that should match.
    let result = os.do_command("make a folder called test").await?;
    println!("[DO] path: {:?}, embedding_ms: {}ms", result.path, result.embedding_ms);
    if let Some(ref matched) = result.matched {
        println!("  matched: \"{}\" (confidence {:.2}, similarity {:.2})",
            matched.trigger_text, matched.confidence, matched.similarity);
    }
    if let Some(ref exec) = result.execution {
        println!("  exit_code: {}, duration: {}ms", exec.exit_code, exec.duration_ms);
        if !exec.stdout.is_empty() {
            println!("  stdout: {}", exec.stdout.trim());
        }
    }

    // Check status.
    let status = os.status();
    println!("[STATUS] hostname={}, tier={:?}, reflexes={}, mode={:?}",
        status.hostname, status.device_tier, status.reflex_count, status.runtime_mode);

    Ok(())
}
