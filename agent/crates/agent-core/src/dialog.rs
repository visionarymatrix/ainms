#![allow(dead_code)]
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum DialogAnswer {
    Yes,
    No,
}

#[derive(Debug, Clone)]
pub struct PromptResult {
    pub text: Option<String>,
}

// ── Public API ──────────────────────────────────────────────────────────────

pub fn ask(title: &str, message: &str) -> Result<DialogAnswer> {
    match std::env::consts::OS {
        "linux" => ask_linux(title, message),
        "windows" => ask_windows(title, message),
        "macos" => ask_macos(title, message),
        other => {
            warn!("Unsupported OS '{}'; falling back to stdout", other);
            ask_stdout(title, message)
        }
    }
}

pub fn notify(title: &str, message: &str) -> Result<()> {
    match std::env::consts::OS {
        "linux" => notify_linux(title, message),
        "windows" => notify_windows(title, message),
        "macos" => notify_macos(title, message),
        other => {
            warn!("Unsupported OS '{}'; falling back to stdout", other);
            info!("NOTIFICATION: [{}] {}", title, message);
            Ok(())
        }
    }
}

pub fn prompt(title: &str, message: &str) -> Result<PromptResult> {
    match std::env::consts::OS {
        "linux" => prompt_linux(title, message),
        "windows" => prompt_windows(title, message),
        "macos" => prompt_macos(title, message),
        other => {
            warn!("Unsupported OS '{}'; falling back to stdout", other);
            prompt_stdout(title, message)
        }
    }
}

// ── Windows (Win32 API) ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn init_visual_styles() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        use windows::Win32::UI::Controls::{InitCommonControlsEx, INITCOMMONCONTROLSEX, ICC_STANDARD_CLASSES};
        let icex = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        unsafe {
            let _ = InitCommonControlsEx(&icex);
        }
    });
}

#[cfg(target_os = "windows")]
fn ask_windows(title: &str, message: &str) -> Result<DialogAnswer> {
    init_visual_styles();
    if is_session_zero() {
        info!("Running in Session 0, spawning dialog helper in user session");
        return spawn_dialog_helper_in_user_session("ask", title, message);
    }
    ask_windows_direct(title, message)
}

#[cfg(target_os = "windows")]
fn notify_windows(title: &str, message: &str) -> Result<()> {
    init_visual_styles();
    if is_session_zero() {
        info!("Running in Session 0, spawning dialog helper in user session");
        let _ = spawn_dialog_helper_in_user_session("notify", title, message);
        return Ok(());
    }
    notify_windows_direct(title, message)
}

#[cfg(target_os = "windows")]
fn prompt_windows(title: &str, message: &str) -> Result<PromptResult> {
    init_visual_styles();
    if is_session_zero() {
        info!("Running in Session 0, spawning dialog helper in user session");
        return spawn_prompt_helper_in_user_session(title, message);
    }
    prompt_windows_direct(title, message)
}

#[cfg(target_os = "windows")]
fn ask_windows_direct(title: &str, message: &str) -> Result<DialogAnswer> {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO};

    let title_w = encode_wide(title);
    let message_w = encode_wide(message);

    unsafe {
        let result = MessageBoxW(
            None,
            windows::core::PCWSTR(message_w.as_ptr()),
            windows::core::PCWSTR(title_w.as_ptr()),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        );
        if result == IDYES {
            Ok(DialogAnswer::Yes)
        } else {
            Ok(DialogAnswer::No)
        }
    }
}

#[cfg(target_os = "windows")]
fn notify_windows_direct(title: &str, message: &str) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

    let title_w = encode_wide(title);
    let message_w = encode_wide(message);

    unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(message_w.as_ptr()),
            windows::core::PCWSTR(title_w.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn prompt_windows_direct(title: &str, message: &str) -> Result<PromptResult> {
    use windows::Win32::UI::WindowsAndMessaging::*;

    let title_w = encode_wide(title);
    let message_w = encode_wide(message);

    let template = build_prompt_dialog_template(&title_w, &message_w);

    unsafe {
        let result = DialogBoxIndirectParamW(
            None,
            template.as_ptr() as *const _,
            None,
            Some(prompt_dialog_proc),
            windows::Win32::Foundation::LPARAM(0),
        );

        if result == 1 {
            let path = dialog_result_path();
            if path.exists() {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let _ = std::fs::remove_file(&path);
                if text.is_empty() {
                    Ok(PromptResult { text: None })
                } else {
                    Ok(PromptResult { text: Some(text) })
                }
            } else {
                Ok(PromptResult { text: None })
            }
        } else {
            Ok(PromptResult { text: None })
        }
    }
}

#[cfg(target_os = "windows")]
fn build_prompt_dialog_template(title_w: &[u16], message_w: &[u16]) -> Vec<u8> {
    // DLGTEMPLATE memory layout (DWORD-aligned):
    //   DLGTEMPLATE (18 bytes) + menu(2) + class(2) + title(var) + font(var)
    //   then DLGITEMTEMPLATE × cdit, each DWORD-aligned
    //
    // DLGTEMPLATE: style(4) + exStyle(4) + cdit(2) + x(2) + y(2) + cx(2) + cy(2)
    // menu/class/title: each is 0x0000 or 0xFFFF+atom or null-term wstring
    //
    // DLGITEMTEMPLATE: style(4) + exStyle(4) + x(2) + y(2) + cx(2) + cy(2) + id(2)
    //   then class(0xFFFF+atom or wstring) + title(wstring or 0xFFFF+id) + extra(2)
    //
    // Layout (dialog units, will be dynamically resized in WM_INITDIALOG):
    //   cx=250, cy=80  (initial size, gets resized after text measurement)
    //   STATIC  ID=101: x=7,  y=7,  cx=236, cy=16
    //   EDIT    ID=102: x=7,  y=30, cx=236, cy=14
    //   Submit  ID=103: x=55, y=50, cx=60,  cy=16
    //   Dismiss ID=104: x=125,y=50, cx=60,  cy=16

    let mut t: Vec<u8> = Vec::new();

    fn push16(t: &mut Vec<u8>, v: u16) { t.extend_from_slice(&v.to_le_bytes()); }
    fn push32(t: &mut Vec<u8>, v: u32) { t.extend_from_slice(&v.to_le_bytes()); }
    fn push_wstr(t: &mut Vec<u8>, s: &[u16]) { for &c in s { push16(t, c); } }
    fn align4(t: &mut Vec<u8>) { while t.len() % 4 != 0 { t.push(0); } }

    // ── DLGTEMPLATE header ──
    push32(&mut t, 0x80C800C0); // style: WS_POPUP|WS_VISIBLE|WS_CAPTION|WS_SYSMENU|DS_CENTER|DS_MODALFRAME|DS_SHELLFONT
    push32(&mut t, 0x00000001); // exStyle: WS_EX_DLGMODALFRAME
    push16(&mut t, 4);          // cdit
    push16(&mut t, 0);          // x
    push16(&mut t, 0);          // y
    push16(&mut t, 250);        // cx (initial width, resized dynamically)
    push16(&mut t, 80);         // cy (initial height, resized dynamically)
    push16(&mut t, 0);          // menu: none
    push16(&mut t, 0);          // class: default
    push_wstr(&mut t, title_w); // title
    push16(&mut t, 8);          // font point size (DS_SHELLFONT)
    push_wstr(&mut t, encode_wide("Segoe UI").as_slice());

    align4(&mut t);

    // ── DLGITEM 1: STATIC text, ID=101 ──
    push32(&mut t, 0x50020000); // WS_CHILD|WS_VISIBLE|SS_NOPREFIX
    push32(&mut t, 0);          // exStyle
    push16(&mut t, 7); push16(&mut t, 7);   // x,y
    push16(&mut t, 236); push16(&mut t, 16); // cx,cy (resized dynamically)
    push16(&mut t, 101); // id
    push16(&mut t, 0xFFFF); push16(&mut t, 0x0082); // class=STATIC atom
    push_wstr(&mut t, message_w); // title=message
    push16(&mut t, 0); // extra

    align4(&mut t);

    // ── DLGITEM 2: EDIT, ID=102 ──
    push32(&mut t, 0x50810080); // WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_BORDER|ES_AUTOHSCROLL
    push32(&mut t, 0);          // exStyle
    push16(&mut t, 7); push16(&mut t, 30);
    push16(&mut t, 236); push16(&mut t, 14);
    push16(&mut t, 102);
    push16(&mut t, 0xFFFF); push16(&mut t, 0x0081); // class=EDIT atom
    push16(&mut t, 0); // title=empty
    push16(&mut t, 0); // extra

    align4(&mut t);

    // ── DLGITEM 3: BUTTON "Submit" (default push), ID=103 ──
    push32(&mut t, 0x50010001); // WS_CHILD|WS_VISIBLE|WS_TABSTOP|BS_DEFPUSHBUTTON
    push32(&mut t, 0);
    push16(&mut t, 55); push16(&mut t, 50);
    push16(&mut t, 60); push16(&mut t, 16);
    push16(&mut t, 103);
    push16(&mut t, 0xFFFF); push16(&mut t, 0x0080); // class=BUTTON atom
    push_wstr(&mut t, encode_wide("Submit").as_slice());
    push16(&mut t, 0);

    align4(&mut t);

    // ── DLGITEM 4: BUTTON "Dismiss", ID=104 ──
    push32(&mut t, 0x50010000); // WS_CHILD|WS_VISIBLE|WS_TABSTOP|BS_PUSHBUTTON
    push32(&mut t, 0);
    push16(&mut t, 125); push16(&mut t, 50);
    push16(&mut t, 60); push16(&mut t, 16);
    push16(&mut t, 104);
    push16(&mut t, 0xFFFF); push16(&mut t, 0x0080); // class=BUTTON atom
    push_wstr(&mut t, encode_wide("Dismiss").as_slice());
    push16(&mut t, 0);

    t
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn prompt_dialog_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    _lparam: windows::Win32::Foundation::LPARAM,
) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::*;

    match msg {
        WM_INITDIALOG => {
            use windows::Win32::Graphics::Gdi as Gdi;

            let _ = SetForegroundWindow(hwnd);

            let hstatic = match GetDlgItem(hwnd, 101) {
                Ok(h) if !h.is_invalid() => h,
                _ => { return TRUE.0 as isize; }
            };

            let mut rc_static: RECT = std::mem::zeroed();
            let _ = GetWindowRect(hstatic, &mut rc_static);

            let mut text_buf = [0u16; 4096];
            let len = GetWindowTextW(hstatic, &mut text_buf);
            let text_len = len as usize;

            let hdc = Gdi::GetDC(hwnd);
            if hdc.is_invalid() {
                return TRUE.0 as isize;
            }

            let hfont_lresult = SendMessageW(hstatic, WM_GETFONT, WPARAM(0), LPARAM(0));
            let hfont_old = if hfont_lresult.0 != 0 {
                let hfont = Gdi::HFONT(hfont_lresult.0 as *mut _);
                Some(Gdi::SelectObject(hdc, Gdi::HGDIOBJ(hfont.0)))
            } else {
                None
            };

            let mut rc_calc: RECT = std::mem::zeroed();
            rc_calc.right = rc_static.right - rc_static.left;
            let dt_flags = Gdi::DT_WORDBREAK | Gdi::DT_CALCRECT | Gdi::DT_NOPREFIX;
            Gdi::DrawTextW(hdc, &mut text_buf[..text_len], &mut rc_calc, dt_flags);

            if let Some(old) = hfont_old {
                Gdi::SelectObject(hdc, old);
            }
            let _ = Gdi::ReleaseDC(hwnd, hdc);

            let ideal_text_h = (rc_calc.bottom - rc_calc.top) as i32;
            let current_text_h = (rc_static.bottom - rc_static.top) as i32;
            let extra = (ideal_text_h - current_text_h).max(0);

            if extra > 0 {
                let margin_x: i32 = 14;
                let gap_text_edit: i32 = 7;
                let edit_h: i32 = 22;
                let gap_edit_btn: i32 = 7;
                let btn_h: i32 = 24;
                let margin_bottom: i32 = 7;
                let new_cy = margin_x + ideal_text_h + gap_text_edit + edit_h + gap_edit_btn + btn_h + margin_bottom;

                let mut rc_dlg: RECT = std::mem::zeroed();
                rc_dlg.right = 250;
                rc_dlg.bottom = new_cy;
                let _ = MapDialogRect(hwnd, &mut rc_dlg);

                let mut rc_win: RECT = std::mem::zeroed();
                let _ = GetWindowRect(hwnd, &mut rc_win);
                let cur_w = rc_win.right - rc_win.left;

                let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, cur_w, rc_dlg.bottom, SWP_NOMOVE | SWP_NOZORDER);

                let mut rc_s: RECT = std::mem::zeroed();
                let _ = GetWindowRect(hstatic, &mut rc_s);
                let _ = SetWindowPos(hstatic, HWND::default(), 0, 0,
                    rc_s.right - rc_s.left, rc_s.bottom - rc_s.top + extra,
                    SWP_NOMOVE | SWP_NOZORDER);

                if let Ok(hedit) = GetDlgItem(hwnd, 102) {
                    if !hedit.is_invalid() {
                        let mut rc_e: RECT = std::mem::zeroed();
                        let _ = GetWindowRect(hedit, &mut rc_e);
                        let _ = SetWindowPos(hedit, HWND::default(),
                            rc_e.left - rc_win.left, rc_e.top - rc_win.top + extra,
                            0, 0, SWP_NOSIZE | SWP_NOZORDER);
                    }
                }

                for id in [103i32, 104i32] {
                    if let Ok(hbtn) = GetDlgItem(hwnd, id) {
                        if !hbtn.is_invalid() {
                            let mut rc_b: RECT = std::mem::zeroed();
                            let _ = GetWindowRect(hbtn, &mut rc_b);
                            let _ = SetWindowPos(hbtn, HWND::default(),
                                rc_b.left - rc_win.left, rc_b.top - rc_win.top + extra,
                                0, 0, SWP_NOSIZE | SWP_NOZORDER);
                        }
                    }
                }

                let mut rc_new: RECT = std::mem::zeroed();
                let _ = GetWindowRect(hwnd, &mut rc_new);
                let w = rc_new.right - rc_new.left;
                let h = rc_new.bottom - rc_new.top;
                let cx_screen = GetSystemMetrics(SM_CXSCREEN);
                let cy_screen = GetSystemMetrics(SM_CYSCREEN);
                let _ = SetWindowPos(hwnd, HWND::default(),
                    (cx_screen - w) / 2, (cy_screen - h) / 2,
                    w, h, SWP_NOZORDER | SWP_FRAMECHANGED);
            }

            TRUE.0 as isize
        }
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as u32;
            match cmd_id {
                103 => {
                    let mut buf = [0u16; 2048];
                    let len = GetDlgItemTextW(hwnd, 102, &mut buf);
                    let text = String::from_utf16_lossy(&buf[..len as usize]);
                    let path = dialog_result_path();
                    let _ = std::fs::write(&path, text.trim());
                    let _ = EndDialog(hwnd, 1);
                    1
                }
                104 => {
                    let _ = EndDialog(hwnd, 0);
                    0
                }
                _ => FALSE.0 as isize,
            }
        }
        WM_CLOSE => {
            let _ = EndDialog(hwnd, 0);
            0
        }
        _ => FALSE.0 as isize,
    }
}

#[cfg(target_os = "windows")]
fn is_session_zero() -> bool {
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    unsafe {
        let mut session_id = 0u32;
        if ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id).is_ok() {
            return session_id == 0;
        }
        false
    }
}

#[cfg(not(target_os = "windows"))]
fn is_session_zero() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn spawn_dialog_helper_in_user_session(dialog_type: &str, title: &str, message: &str) -> Result<DialogAnswer> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
    use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, CREATE_UNICODE_ENVIRONMENT, CREATE_NO_WINDOW,
        PROCESS_INFORMATION, STARTUPINFOW, STARTF_USESHOWWINDOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PWSTR;

    let result_path = dialog_result_path();
    let _ = std::fs::remove_file(&result_path);

    unsafe {
        let session_id = WTSGetActiveConsoleSessionId();
        if session_id == 0xFFFFFFFF {
            bail!("No active console session");
        }

        let mut user_token = windows::Win32::Foundation::HANDLE::default();
        if WTSQueryUserToken(session_id, &mut user_token).is_err() {
            bail!("WTSQueryUserToken failed (are we running as SYSTEM?)");
        }

        let mut env_block: *mut core::ffi::c_void = std::ptr::null_mut();
        if CreateEnvironmentBlock(&mut env_block, user_token, false).is_err() {
            let _ = CloseHandle(user_token);
            bail!("CreateEnvironmentBlock failed");
        }

        let exe_path = std::env::current_exe()?;
        let flag = format!("--dialog-{}", dialog_type);
        let title_escaped = title.replace('"', "\\\"");
        let msg_escaped = message.replace('"', "\\\"");
        let title_arg = format!(r#"--dialog-title "{}""#, title_escaped);
        let msg_arg = format!(r#"--dialog-message "{}""#, msg_escaped);
        let cmdline = format!(r#""{}" {} {} {}"#, exe_path.display(), flag, title_arg, msg_arg);
        let mut cmdline_wide: Vec<u16> = std::ffi::OsStr::new(&cmdline)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect();

        let mut startup_info: STARTUPINFOW = std::mem::zeroed();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup_info.dwFlags = STARTF_USESHOWWINDOW;
        startup_info.wShowWindow = SW_SHOWNORMAL.0 as u16;
        let mut proc_info: PROCESS_INFORMATION = std::mem::zeroed();

        let create_result = CreateProcessAsUserW(
            user_token,
            None,
            PWSTR(cmdline_wide.as_mut_ptr()),
            None,
            None,
            false,
            CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            Some(env_block),
            None,
            &startup_info,
            &mut proc_info,
        );

        if !env_block.is_null() {
            let _ = DestroyEnvironmentBlock(env_block);
        }
        let _ = CloseHandle(user_token);

        if create_result.is_err() {
            bail!("CreateProcessAsUserW failed for dialog helper");
        }

        let _ = CloseHandle(proc_info.hProcess);
        let _ = CloseHandle(proc_info.hThread);

        info!("Spawned dialog helper in user session {}", session_id);
    }

    let timeout = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if result_path.exists() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let content = std::fs::read_to_string(&result_path).unwrap_or_default();
            let _ = std::fs::remove_file(&result_path);
            if dialog_type == "ask" {
                return if content.trim() == "yes" {
                    Ok(DialogAnswer::Yes)
                } else {
                    Ok(DialogAnswer::No)
                };
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(DialogAnswer::No)
}

#[cfg(target_os = "windows")]
fn spawn_prompt_helper_in_user_session(title: &str, message: &str) -> Result<PromptResult> {
    spawn_dialog_helper_in_user_session("prompt", title, message)?;
    let result_path = dialog_result_path();
    if result_path.exists() {
        let text = std::fs::read_to_string(&result_path).unwrap_or_default();
        let _ = std::fs::remove_file(&result_path);
        if text.is_empty() {
            Ok(PromptResult { text: None })
        } else {
            Ok(PromptResult { text: Some(text) })
        }
    } else {
        Ok(PromptResult { text: None })
    }
}

#[cfg(target_os = "windows")]
fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0u16)).collect()
}

fn dialog_result_path() -> PathBuf {
    std::env::temp_dir().join("ainms_dialog_result.txt")
}

// ── Linux ────────────────────────────────────────────────────────────

fn ask_linux(title: &str, message: &str) -> Result<DialogAnswer> {
    use std::process::Command;

    let out = Command::new("zenity")
        .args(["--question", "--title", title, "--text", message])
        .output();

    match out {
        Ok(o) => {
            if o.status.success() {
                return Ok(DialogAnswer::Yes);
            }
            if o.status.code() == Some(1) {
                return Ok(DialogAnswer::No);
            }
        }
        Err(e) => warn!("zenity failed: {}", e),
    }

    let out = Command::new("kdialog")
        .args(["--yesno", message, "--title", title])
        .output();

    match out {
        Ok(o) => {
            if o.status.success() {
                return Ok(DialogAnswer::Yes);
            }
            if o.status.code() == Some(1) {
                return Ok(DialogAnswer::No);
            }
        }
        Err(e) => warn!("kdialog failed: {}", e),
    }

    ask_stdout(title, message)
}

fn notify_linux(title: &str, message: &str) -> Result<()> {
    use std::process::Command;

    match Command::new("notify-send")
        .args([title, message])
        .output()
    {
        Ok(o) if o.status.success() => Ok(()),
        Ok(_) => {
            warn!("notify-send exited with error; trying zenity fallback");
            Command::new("zenity")
                .args(["--info", "--title", title, "--text", message])
                .output()
                .context("zenity fallback failed")?;
            Ok(())
        }
        Err(e) => {
            warn!("notify-send failed: {}; using stdout fallback", e);
            info!("NOTIFICATION: [{}] {}", title, message);
            Ok(())
        }
    }
}

fn prompt_linux(title: &str, message: &str) -> Result<PromptResult> {
    use std::process::Command;

    let out = Command::new("zenity")
        .args(["--entry", "--title", title, "--text", message])
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if text.is_empty() {
                Ok(PromptResult { text: None })
            } else {
                Ok(PromptResult { text: Some(text) })
            }
        }
        Ok(_) => Ok(PromptResult { text: None }),
        Err(e) => {
            warn!("zenity --entry failed: {}", e);
            prompt_stdout(title, message)
        }
    }
}

// ── macOS ───────────────────────────────────────────────────────────

fn ask_macos(title: &str, message: &str) -> Result<DialogAnswer> {
    use std::process::Command;

    let script = format!(
        "display dialog \"{}\" with title \"{}\" buttons {{\"No\", \"Yes\"}} default button \"Yes\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );

    let out = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("osascript dialog failed")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("Yes") || out.status.success() {
        Ok(DialogAnswer::Yes)
    } else {
        Ok(DialogAnswer::No)
    }
}

fn notify_macos(title: &str, message: &str) -> Result<()> {
    use std::process::Command;

    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("osascript notification failed")?;

    Ok(())
}

fn prompt_macos(title: &str, message: &str) -> Result<PromptResult> {
    use std::process::Command;

    let script = format!(
        "display dialog \"{}\" with title \"{}\" default answer \"\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );

    let out = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if let Some(start) = stdout.find("text returned:") {
                let text = stdout[start + 15..].trim().trim_matches('"').to_string();
                if text.is_empty() {
                    Ok(PromptResult { text: None })
                } else {
                    Ok(PromptResult { text: Some(text) })
                }
            } else {
                Ok(PromptResult { text: None })
            }
        }
        _ => Ok(PromptResult { text: None }),
    }
}

// ── Fallback (stdout) ──────────────────────────────────────────────────

fn ask_stdout(title: &str, message: &str) -> Result<DialogAnswer> {
    println!("\n=== {} ===", title);
    println!("{}", message);

    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("[Y/N]? ");
        let _ = stdout.flush();
        let mut line = String::new();
        let _ = stdin.lock().read_line(&mut line);
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(DialogAnswer::Yes),
            "n" | "no" => return Ok(DialogAnswer::No),
            _ => println!("Please enter Y or N"),
        }
    }
}

fn prompt_stdout(title: &str, message: &str) -> Result<PromptResult> {
    println!("\n=== {} ===", title);
    println!("{}", message);

    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("Answer: ");
    let _ = stdout.flush();
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    let text = line.trim().to_string();
    if text.is_empty() {
        Ok(PromptResult { text: None })
    } else {
        Ok(PromptResult { text: Some(text) })
    }
}

// ── CLI helper entry point ───────────────────────────────────────────

pub fn run_dialog_helper(dialog_type: &str, title: &str, message: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    init_visual_styles();

    match dialog_type {
        "notify" => {
            #[cfg(target_os = "windows")]
            notify_windows_direct(title, message)?;
            Ok(())
        }
        "ask" => {
            #[cfg(target_os = "windows")]
            {
                let answer = ask_windows_direct(title, message)?;
                let result = match answer {
                    DialogAnswer::Yes => "yes",
                    DialogAnswer::No => "no",
                };
                std::fs::write(dialog_result_path(), result)?;
            }
            Ok(())
        }
        "prompt" => {
            #[cfg(target_os = "windows")]
            {
                let result = prompt_windows_direct(title, message)?;
                match result.text {
                    Some(text) => std::fs::write(dialog_result_path(), text)?,
                    None => std::fs::write(dialog_result_path(), "")?,
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let result = prompt_stdout(title, message)?;
                match result.text {
                    Some(text) => std::fs::write(dialog_result_path(), text)?,
                    None => std::fs::write(dialog_result_path(), "")?,
                }
            }
            Ok(())
        }
        other => bail!("Unknown dialog type: {}", other),
    }
}