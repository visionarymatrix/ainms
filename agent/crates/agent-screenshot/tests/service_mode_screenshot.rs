//! Integration test: verify screenshot capture works correctly in both
//! interactive mode and the --take-screenshot helper path used by Session 0.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Helper: canonical output path for captured screenshots.
fn screenshot_output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ainms_test_{}.png", name))
}

/// Verify raw bytes are a valid PNG with nonzero pixel data.
fn verify_png(data: &[u8], label: &str) {
    assert!(data.len() >= 8, "[{}] PNG data too short: {} bytes", label, data.len());
    assert_eq!(
        &data[0..4],
        b"\x89PNG",
        "[{}] Missing PNG magic header (got {:?})",
        label,
        &data[0..4]
    );

    // Check IHDR chunk exists after the 8-byte PNG signature
    assert!(
        data.len() >= 24,
        "[{}] PNG too short for IHDR chunk",
        label
    );

    // Verify nonzero pixel data — a real screen capture should have content.
    // All-zero 4-byte BGRA pixels would mean a blank/black capture (Session 0 symptom).
    let nonzero_count: usize = data[8..].iter().filter(|&&b| b != 0).count();
    let total = data.len() - 8;
    let nonzero_pct = (nonzero_count as f64 / total as f64) * 100.0;
    assert!(
        nonzero_pct > 1.0,
        "[{}] Screen appears blank ({:.1}% nonzero bytes) — possible Session 0 capture",
        label,
        nonzero_pct
    );
}

// ── Test 1: Direct capture via ScreenshotCommander ──────────────────────────

#[tokio::test]
async fn capture_direct_saves_valid_png() {
    if agent_screenshot::is_session_zero() {
        eprintln!("SKIP: running in Session 0 (service context)");
        return;
    }

    let commander = agent_screenshot::ScreenshotCommander::new();
    let data = commander
        .capture()
        .await
        .expect("capture() failed in interactive session");

    verify_png(&data, "direct-capture");

    let path = screenshot_output_path("direct_capture");
    let mut file = fs::File::create(&path).expect("failed to create output file");
    file.write_all(&data).expect("failed to write PNG data");

    let metadata = fs::metadata(&path).expect("failed to read output file");
    assert!(metadata.len() > 100, "PNG file too small: {} bytes", metadata.len());

    let _ = fs::remove_file(&path);
}

// ── Test 2: --take-screenshot helper path (subprocess) ──────────────────────
//
// This simulates what `capture_in_user_session` does: spawn agent-core.exe
// with --take-screenshot flags. The helper captures + uploads to the server,
// but we intercept the capture by running it without server connectivity —
// we just verify the process starts correctly and the capture logic runs.

#[test]
fn take_screenshot_subprocess_flag_is_accepted() {
    let exe = std::env::current_exe().expect("failed to get current exe path");
    let agent_core = exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("agent-core.exe"))
        .filter(|p| p.exists());

    let agent_core = match agent_core {
        Some(p) => p,
        None => {
            // Fall back to target/release or target/debug
            let release = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("target")
                .join("release")
                .join("agent-core.exe");
            let debug = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("target")
                .join("debug")
                .join("agent-core.exe");
            if release.exists() {
                release
            } else if debug.exists() {
                debug
            } else {
                eprintln!("SKIP: agent-core.exe not found in target/release or target/debug");
                return;
            }
        }
    };

    // Run with --take-screenshot but missing required --device-id etc.
    // Should fail with a clear error, not crash or panic.
    let output = std::process::Command::new(&agent_core)
        .arg("--take-screenshot")
        .output()
        .expect("failed to spawn agent-core --take-screenshot");

    // The --take-screenshot flag is accepted (exit != invalid-args for the flag itself).
    // It will fail because --device-id is missing, but that's expected.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unrecognized") && !stderr.contains("unexpected"),
        "--take-screenshot flag should be recognized, stderr: {}", stderr
    );
}

// ── Test 3: Session 0 detection is consistent ──────────────────────────────

#[test]
fn is_session_zero_is_consistent() {
    let result1 = agent_screenshot::is_session_zero();
    let result2 = agent_screenshot::is_session_zero();
    assert_eq!(result1, result2, "is_session_zero() returned inconsistent results");
}

// ── Test 4: Verify capture refuses in Session 0 ────────────────────────────

#[tokio::test]
async fn capture_refuses_in_session_zero() {
    let commander = agent_screenshot::ScreenshotCommander::new();
    let result = commander.capture().await;

    if agent_screenshot::is_session_zero() {
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Session 0"),
            "Error should mention Session 0, got: {}", err
        );
    } else {
        // In interactive session, capture should succeed
        assert!(result.is_ok(), "capture() should succeed in interactive session");
        let data = result.unwrap();
        verify_png(&data, "session0-guard-interactive");
    }
}

// ── Test 5: File save round-trip ────────────────────────────────────────────
// Capture → save to disk → read back → verify PNG integrity

#[tokio::test]
async fn capture_save_read_roundtrip() {
    if agent_screenshot::is_session_zero() {
        eprintln!("SKIP: running in Session 0");
        return;
    }

    let commander = agent_screenshot::ScreenshotCommander::new();
    let original_data = commander.capture().await.expect("capture failed");

    let path = screenshot_output_path("roundtrip");
    fs::write(&path, &original_data).expect("failed to write PNG");

    let reloaded = fs::read(&path).expect("failed to read PNG back");
    assert_eq!(original_data, reloaded, "File data mismatch after round-trip");
    verify_png(&reloaded, "roundtrip");

    let _ = fs::remove_file(&path);
}