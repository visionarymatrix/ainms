use std::time::Duration;

use agent_collectors::ChromeLauncher;
use agent_collectors::shortcuts;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Chrome Auto-Launcher Test ===\n");

    println!("[1] Finding Chrome installation...");
    match ChromeLauncher::find_chrome_path() {
        Some(path) => println!("  ✓ Chrome found at: {}", path.display()),
        None => println!("  ✗ Chrome not found"),
    }

    println!("\n[2] Checking Chrome process...");
    let running = ChromeLauncher::is_chrome_running();
    println!("  Chrome running: {}", running);

    println!("\n[3] Checking debug port 9222...");
    let launcher = ChromeLauncher::new();
    let port_open = launcher.is_debug_port_open();
    println!("  Debug port open: {}", port_open);

    println!("\n[4] Finding Chrome shortcuts...");
    let shortcuts = shortcuts::find_chrome_shortcuts();
    for s in &shortcuts {
        println!("  → {}", s.display());
    }

    println!("\n[5] Patching user shortcuts (no admin required)...");
    let results = shortcuts::patch_user_shortcuts_only();
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(()) => println!("  ✓ Shortcut {} patched", i + 1),
            Err(e) => println!("  ✗ Shortcut {} failed: {}", i + 1, e),
        }
    }

    println!("\n[6] Ensuring Chrome is debuggable...");
    launcher.ensure_chrome_debuggable();

    println!("\n[7] Waiting for Chrome to start...");
    std::thread::sleep(Duration::from_secs(5));

    let port_open_after = launcher.is_debug_port_open();
    println!("  Debug port open after fix: {}", port_open_after);

    if port_open_after {
        println!("\n=== SUCCESS ===");
        println!("Chrome is now running with debug port 9222");
        println!("Access tabs at: http://localhost:9222/json");
    } else {
        println!("\n=== Chrome started but debug port not detected yet ===");
        println!("Try: curl http://localhost:9222/json/version");
    }
}