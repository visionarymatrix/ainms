use std::sync::Arc;
use std::time::Duration;

use agent_collectors::AppUsageTracker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Desktop Application Usage Tracker ===");
    println!("Sampling every 3 seconds for 30 seconds...\n");

    let tracker = AppUsageTracker::new(3);
    let tracker = tracker.spawn_sampler();

    let tracker_clone = Arc::clone(&tracker);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        tracker_clone.stop();
    });

    while tracker.is_running() {
        tokio::time::sleep(Duration::from_secs(3)).await;

        if let Some(app) = tracker.get_current_app() {
            if let Some(entry) = tracker.get_usage_for_app(&app) {
                println!(
                    "Active: {:<30} ({:.1}s so far)",
                    app, entry.duration_secs
                );
            }
        }
    }

    println!("\n{}", tracker.get_summary());

    Ok(())
}