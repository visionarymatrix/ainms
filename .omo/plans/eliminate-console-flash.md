# Eliminate Console Window Flash on Windows

## Problem

When `agent-core.exe` is spawned via `CreateProcessAsUserW` (service mode) for
dialogs or screenshots, a black console window briefly flashes on screen before
`FreeConsole()` detaches from it. This happens because the exe is compiled as
a `CONSOLE` subsystem binary — Windows creates the console window at process
creation time, before any user code runs. `FreeConsole()` is a band-aid that
closes the window after it already appeared.

## Root Cause

The Windows PE header contains a `subsystem` field:
- `CONSOLE` (default for Rust binaries) → Windows **creates a console window
  immediately** on process creation
- `WINDOWS` → Windows **never creates a console window**

The flash is not preventable from code — it's OS behavior at process creation.

## Solution: Switch to WINDOWS Subsystem

Add `#![windows_subsystem = "windows"]` to `main.rs`. This marks the exe as a
GUI application in the PE header. No console window is ever created by the OS.

### Tradeoff & Fix

A `WINDOWS` subsystem process does not automatically inherit the parent's
console. When running interactively from cmd/PowerShell, `info!()/warn!()` log
output would be invisible.

**Fix**: Call `AttachConsole(ATTACH_PARENT_PROCESS)` at the very start of
`main()`. This re-attaches the process to the parent terminal's console.
stdout/stderr work normally, logs appear in the terminal.

## Scenarios After Fix

| Scenario                              | Console created? | Logs visible? |
|---------------------------------------|-------------------|----------------|
| `agent-core` from cmd/PowerShell      | No                | Yes — `AttachConsole` reattaches |
| `agent-core --dialog-notify` (schtasks) | No             | No (desired) |
| `agent-core --take-screenshot` (service) | No            | No (desired) |
| `agent-core service` (Windows Service) | No              | No (logs via tracing/Event Log) |

## Implementation Steps

### 1. Add `#![windows_subsystem = "windows"]` to `main.rs`

Top of file, before any other attributes:

```rust
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
```

Use `cfg_attr` so it only applies on Windows. Linux/macOS builds are unaffected.

### 2. Add `AttachConsole(ATTACH_PARENT_PROCESS)` at the start of `main()`

```rust
fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    let args = Args::parse();
    // ... rest of main
}
```

`AttachConsole` fails silently (returns `Err`) if there's no parent console
(e.g. schtasks, service mode) — that's fine, we just ignore it.

### 3. Remove `FreeConsole()` calls

No longer needed since no console is ever created:

- `dialog.rs` → `run_dialog_helper()`: remove `FreeConsole()` block
- `main.rs` → `run_take_screenshot()`: remove `FreeConsole()` block

### 4. Remove `CREATE_NO_WINDOW` flag (optional)

With `WINDOWS` subsystem, `CREATE_NO_WINDOW` is redundant — no console is
created regardless. However, keeping it is harmless and communicates intent.
Decision: keep it for clarity.

### 5. No Cargo.toml changes needed

`Win32_System_Console` feature is already in `agent-core/Cargo.toml` (added
for `FreeConsole`). It now provides `AttachConsole` and `ATTACH_PARENT_PROCESS`
instead.

## Files Changed

| File | Change |
|------|--------|
| `agent/crates/agent-core/src/main.rs` | Add `#![cfg_attr(...)]`, add `AttachConsole`, remove `FreeConsole` |
| `agent/crates/agent-core/src/dialog.rs` | Remove `FreeConsole` from `run_dialog_helper` |

## Verification

1. `cargo build -p agent-core` — 0 warnings
2. Run `agent-core.exe` from PowerShell — logs visible
3. Run `agent-core --dialog-notify` via schtasks — no console flash, dialog appears
4. Run `agent-core --dialog-ask` via schtasks — no console flash, result file written
5. Run `agent-core --dialog-prompt` via schtasks — no console flash, text input works
6. `cargo test -p agent-core` — 9/9 tests pass