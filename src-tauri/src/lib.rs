use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

struct ZuzuMemory {
    messages: Mutex<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppContact {
    aliases: Vec<String>,
    name: String,
    phone: String,
}

#[derive(Debug, Serialize)]
struct ResolvedWhatsAppContact {
    name: String,
    phone: String,
}

const DEFAULT_WHATSAPP_CONTACTS: &str = "[]\n";

fn zuzu_directory() -> Result<PathBuf, String> {
    let documents = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "Could not find the user profile directory".to_string())?
        .join("Documents");

    let directory = documents.join("Zuzu");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create the Zuzu folder: {}", error))?;

    Ok(directory)
}

fn whatsapp_contacts_path() -> Result<PathBuf, String> {
    Ok(zuzu_directory()?.join("contacts.json"))
}

fn load_whatsapp_contacts() -> Result<Vec<WhatsAppContact>, String> {
    let path = whatsapp_contacts_path()?;
    if !path.exists() {
        let mut file = fs::File::create(&path)
            .map_err(|error| format!("Could not create contacts.json: {}", error))?;
        file.write_all(DEFAULT_WHATSAPP_CONTACTS.as_bytes())
            .map_err(|error| format!("Could not write contacts.json: {}", error))?;
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read contacts.json: {}", error))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Could not parse contacts.json: {}", error))
}

fn normalize_alias(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn valid_phone(phone: &str) -> bool {
    let Some(digits) = phone.strip_prefix('+') else {
        return false;
    };

    (8..=15).contains(&digits.len())
        && digits.chars().all(|character| character.is_ascii_digit())
}

fn resolve_whatsapp_contact(recipient: &str) -> Result<ResolvedWhatsAppContact, String> {
    let normalized = normalize_alias(recipient);
    if normalized.is_empty() {
        return Err("WhatsApp recipient cannot be empty".to_string());
    }

    let matches: Vec<ResolvedWhatsAppContact> = load_whatsapp_contacts()?
        .into_iter()
        .filter(|entry| {
            entry
                .aliases
                .iter()
                .any(|alias| normalize_alias(alias) == normalized)
        })
        .map(|entry| {
            if !valid_phone(&entry.phone) {
                return Err(format!(
                    "The WhatsApp phone number for {} is invalid.",
                    entry.name
                ));
            }
            Ok(ResolvedWhatsAppContact {
                name: entry.name,
                phone: entry.phone,
            })
        })
        .collect::<Result<_, _>>()?;

    match matches.as_slice() {
        [] => Err(format!(
            "I don't have a WhatsApp contact configured for {} yet.",
            recipient.trim()
        )),
        [contact] => Ok(ResolvedWhatsAppContact {
            name: contact.name.clone(),
            phone: contact.phone.clone(),
        }),
        _ => Err(format!(
            "I found multiple contacts configured for {}. I won't send the message until the contact mapping is fixed.",
            recipient.trim()
        )),
    }
}

#[tauri::command]
fn resolve_whatsapp_contact_command(
    recipient: String,
) -> Result<ResolvedWhatsAppContact, String> {
    resolve_whatsapp_contact(&recipient)
}

fn safe_file_path(filename: &str) -> Result<PathBuf, String> {
    let path = Path::new(filename);
    let mut components = path.components();

    if path.as_os_str().is_empty()
        || path.is_absolute()
        || components.next() != Some(Component::Normal(path.as_os_str()))
        || components.next().is_some()
    {
        return Err("Only a single filename inside the Zuzu folder is allowed".to_string());
    }

    Ok(zuzu_directory()?.join(path))
}

#[tauri::command]
fn list_files() -> Result<Vec<String>, String> {
    let directory = zuzu_directory()?;
    let mut files = Vec::new();

    for entry in fs::read_dir(directory)
        .map_err(|error| format!("Could not list the Zuzu folder: {}", error))?
    {
        let entry = entry.map_err(|error| format!("Could not read a directory entry: {}", error))?;
        if entry
            .file_type()
            .map_err(|error| format!("Could not inspect a directory entry: {}", error))?
            .is_file()
        {
            files.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    files.sort();
    Ok(files)
}

#[tauri::command]
fn read_text_file(filename: String) -> Result<String, String> {
    let path = safe_file_path(&filename)?;
    fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {}", filename, error))
}

#[tauri::command]
fn create_text_file(filename: String, content: String) -> Result<(), String> {
    let path = safe_file_path(&filename)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(content.as_bytes())
        })
        .map_err(|error| format!("Could not create {}: {}", filename, error))
}

#[tauri::command]
fn write_text_file(filename: String, content: String) -> Result<(), String> {
    let path = safe_file_path(&filename)?;
    if !path.is_file() {
        return Err(format!("{} does not exist in the Zuzu folder", filename));
    }

    fs::write(&path, content)
        .map_err(|error| format!("Could not write {}: {}", filename, error))
}

#[cfg(target_os = "windows")]
fn open_with_default_application(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let operation: Vec<u16> = std::ffi::OsStr::new("open")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };

    if result as isize <= 32 {
        return Err(format!("Could not open {}", path.display()));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_with_default_application(_path: &Path) -> Result<(), String> {
    Err("Opening local files is only supported on Windows".to_string())
}

#[tauri::command]
fn open_file(filename: String) -> Result<(), String> {
    let path = safe_file_path(&filename)?;
    if !path.is_file() {
        return Err(format!("{} does not exist in the Zuzu folder", filename));
    }

    open_with_default_application(&path)
}

#[tauri::command]
fn open_folder() -> Result<(), String> {
    let directory = zuzu_directory()?;
    open_with_default_application(&directory)
}

#[cfg(target_os = "windows")]
fn open_application_path(application: &str) -> Result<PathBuf, String> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
    let program_files_x86 = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);
    let windows = std::env::var_os("WINDIR").map(PathBuf::from);

    let candidates = match application {
        "vscode" => vec![
            local_app_data.map(|path| path.join("Programs\\Microsoft VS Code\\Code.exe")),
            program_files.map(|path| path.join("Microsoft VS Code\\Code.exe")),
        ],
        "chrome" => vec![
            local_app_data.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
            program_files.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
            program_files_x86.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
        ],
        "edge" => vec![
            program_files.map(|path| path.join("Microsoft\\Edge\\Application\\msedge.exe")),
            program_files_x86.map(|path| path.join("Microsoft\\Edge\\Application\\msedge.exe")),
        ],
        "whatsapp" => vec![
            local_app_data.map(|path| path.join("WhatsApp\\WhatsApp.exe")),
            program_files.map(|path| path.join("WhatsApp\\WhatsApp.exe")),
        ],
        "explorer" => vec![
            windows.map(|path| path.join("explorer.exe")),
        ],
        "notepad" => vec![
            windows.map(|path| path.join("System32\\notepad.exe")),
        ],
        "powershell" => vec![
            windows.map(|path| path.join("System32\\WindowsPowerShell\\v1.0\\powershell.exe")),
        ],
        "spotify" => vec![
            local_app_data.map(|path| path.join("Spotify\\Spotify.exe")),
            program_files.map(|path| path.join("Spotify\\Spotify.exe")),
            program_files_x86.map(|path| path.join("Spotify\\Spotify.exe")),
        ],
        _ => return Err("Unsupported application".to_string()),
    };

    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .ok_or_else(|| format!("Could not find {}", application))
}

#[cfg(target_os = "windows")]
fn launch_whatsapp_desktop() -> Result<(), String> {
    open_special_application("whatsapp")
        .map_err(|error| format!("WhatsApp automation failed at stage 3: AppsFolder launch ({})", error))
}

#[cfg(target_os = "windows")]
fn open_special_application(application: &str) -> Result<(), String> {
    let windows = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .ok_or_else(|| "Could not find the Windows directory".to_string())?;
    let explorer = windows.join("explorer.exe");
    let target = match application {
        "calculator" => "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
        "settings" => "ms-settings:",
        "whatsapp" => "shell:AppsFolder\\5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App",
        _ => return Err("Unsupported special application".to_string()),
    };

    Command::new(explorer)
        .arg(target)
        .spawn()
        .map_err(|error| format!("Could not open {}: {}", application, error))?;
    Ok(())
}

#[tauri::command]
fn open_application(application: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if matches!(application.as_str(), "calculator" | "settings") {
            return open_special_application(&application);
        }

        if application == "whatsapp" {
            if open_application_path(&application).is_err() {
                return open_special_application(&application);
            }
        }

        let path = open_application_path(&application)?;
        Command::new(&path)
            .spawn()
            .map_err(|error| format!("Could not open {}: {}", application, error))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = application;
        Err("Opening applications is only supported on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
fn paste_whatsapp_text(text: &str, stage: u8, description: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_CONTROL, SendInput, INPUT, INPUT_0, KEYBDINPUT, INPUT_KEYBOARD,
    };

    set_clipboard_text(text)?;

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0 },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: 0x56, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0 },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: 0x56, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VK_CONTROL, wScan: 0, dwFlags: KEYEVENTF_KEYUP, time: 0, dwExtraInfo: 0 },
            },
        },
    ];
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    log::info!("WhatsApp stage {} clipboard paste SendInput result: {}", stage, sent);

    if sent != inputs.len() as u32 {
        return Err(format!("WhatsApp automation failed at stage {}: {}", stage, description));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_whatsapp_new_chat() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU, SendInput, INPUT, INPUT_0, KEYBDINPUT,
        INPUT_KEYBOARD,
    };

    const KEY_N: u16 = 0x4E;
    let inputs = [
        (VK_CONTROL, 0),
        (VK_MENU, 0),
        (KEY_N, 0),
        (KEY_N, KEYEVENTF_KEYUP),
        (VK_MENU, KEYEVENTF_KEYUP),
        (VK_CONTROL, KEYEVENTF_KEYUP),
    ]
    .map(|(w_vk, flags)| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: w_vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!(
        "WhatsApp stage 6 new-chat shortcut SendInput result: {}",
        sent
    );
    if sent != inputs.len() as u32 {
        return Err(
            "WhatsApp automation failed at stage 6: new-chat shortcut".to_string(),
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn clear_whatsapp_search_field() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_BACK, VK_CONTROL, SendInput, INPUT, INPUT_0, KEYBDINPUT,
        INPUT_KEYBOARD,
    };

    const KEY_A: u16 = 0x41;
    let inputs = [
        (VK_CONTROL, 0),
        (KEY_A, 0),
        (KEY_A, KEYEVENTF_KEYUP),
        (VK_CONTROL, KEYEVENTF_KEYUP),
        (VK_BACK, 0),
        (VK_BACK, KEYEVENTF_KEYUP),
    ]
    .map(|(w_vk, flags)| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: w_vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    });

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!(
        "WhatsApp stage 7 clear search field SendInput result: {}",
        sent
    );
    if sent != inputs.len() as u32 {
        return Err(
            "WhatsApp automation failed at stage 7: clear search field".to_string(),
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn activate_whatsapp_chat() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_RETURN, SendInput, INPUT, INPUT_0, KEYBDINPUT, INPUT_KEYBOARD,
    };

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!("WhatsApp stage 9 chat activation SendInput result: {}", sent);
    if sent != inputs.len() as u32 {
        return Err("WhatsApp automation failed at stage 9: chat activation".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn focus_whatsapp_composer() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::{
            Input::KeyboardAndMouse::{
                MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, SendInput, INPUT, INPUT_0,
                MOUSEINPUT, INPUT_MOUSE,
            },
            WindowsAndMessaging::{FindWindowW, GetWindowRect, SetCursorPos},
        },
    };

    let title: Vec<u16> = std::ffi::OsStr::new("WhatsApp")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if window.is_null() {
        return Err("WhatsApp automation failed at stage 10: message composer focus".to_string());
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return Err("WhatsApp automation failed at stage 10: message composer focus".to_string());
    }

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let composer_x = rect.left + (width * 68 / 100);
    let composer_y = rect.bottom - (height * 8 / 100);
    if unsafe { SetCursorPos(composer_x, composer_y) } == 0 {
        return Err("WhatsApp automation failed at stage 10: message composer focus".to_string());
    }

    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!(
        "WhatsApp stage 10 composer click at ({}, {}) SendInput result: {}",
        composer_x,
        composer_y,
        sent
    );
    if sent != inputs.len() as u32 {
        return Err("WhatsApp automation failed at stage 10: message composer focus".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn paste_whatsapp_clipboard(stage: u8, description: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_CONTROL, SendInput, INPUT, INPUT_0, KEYBDINPUT, INPUT_KEYBOARD,
    };

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0x56,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: 0x56,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!("WhatsApp stage {} {} SendInput result: {}", stage, description, sent);
    if sent != inputs.len() as u32 {
        return Err(format!("WhatsApp automation failed at stage {}: {}", stage, description));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn submit_whatsapp_message() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_KEYUP, VK_RETURN, SendInput, INPUT, INPUT_0, KEYBDINPUT, INPUT_KEYBOARD,
    };

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_RETURN,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    log::info!("WhatsApp stage 13 message submission SendInput result: {}", sent);
    if sent != inputs.len() as u32 {
        return Err("WhatsApp automation failed at stage 13: message submission".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_clipboard_text(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|_| {
        "WhatsApp automation stopped: clipboard verification failed.".to_string()
    })?;
    clipboard.set_text(text).map_err(|_| {
        "WhatsApp automation stopped: clipboard verification failed.".to_string()
    })?;
    let actual = clipboard.get_text().map_err(|_| {
        "WhatsApp automation stopped: clipboard verification failed.".to_string()
    })?;
    if actual != text {
        return Err("WhatsApp automation stopped: clipboard verification failed.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn activate_whatsapp() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let title: Vec<u16> = std::ffi::OsStr::new("WhatsApp")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    log::info!("WhatsApp stage 4 FindWindowW result: {:?}", window);
    if window.is_null() {
        return Err("WhatsApp automation failed at stage 4: WhatsApp window detection".to_string());
    }

    unsafe {
        ShowWindow(window, SW_RESTORE);
        let activated = SetForegroundWindow(window);
        let foreground = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        log::info!(
            "WhatsApp stage 5 SetForegroundWindow result: {}, GetForegroundWindow matches: {}",
            activated,
            foreground == window
        );
        if activated == 0 || foreground != window {
            return Err("WhatsApp automation failed at stage 5: window activation / foreground".to_string());
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn send_whatsapp_message_impl(recipient: &str, _message: &str) -> Result<String, String> {
    launch_whatsapp_desktop()?;
    log::info!("WhatsApp stage 3 AppsFolder launch completed");
    thread::sleep(Duration::from_secs(3));
    activate_whatsapp()?;
    log::info!("WhatsApp stage 4/5 initial window detection and activation completed");

    open_whatsapp_new_chat()?;
    thread::sleep(Duration::from_millis(700));
    clear_whatsapp_search_field()?;
    thread::sleep(Duration::from_millis(100));

    let search_number = recipient.trim_start_matches('+');
    paste_whatsapp_text(search_number, 7, "phone-number search")?;
    thread::sleep(Duration::from_millis(1500));
    log::info!("WhatsApp stage 7 phone-number search wait completed");

    activate_whatsapp_chat()?;
    thread::sleep(Duration::from_millis(1200));
    focus_whatsapp_composer()?;
    thread::sleep(Duration::from_millis(400));
    set_clipboard_text(_message)?;
    paste_whatsapp_clipboard(12, "message paste")?;
    thread::sleep(Duration::from_millis(400));
    submit_whatsapp_message()?;
    Ok("Message submitted to WhatsApp.".to_string())
}

#[tauri::command]
fn send_whatsapp_message(recipient: String, message: String) -> Result<String, String> {
    if recipient.trim().is_empty() || recipient.chars().count() > 100 {
        return Err("WhatsApp automation failed at stage 1: contact resolution".to_string());
    }
    if message.trim().is_empty() || message.chars().count() > 4000 {
        return Err("WhatsApp automation failed at stage 2: phone/message validation".to_string());
    }

    let contact = resolve_whatsapp_contact(&recipient)
        .map_err(|error| format!("WhatsApp automation failed at stage 1: contact resolution ({})", error))?;
    if !valid_phone(&contact.phone) {
        return Err("WhatsApp automation failed at stage 2: phone number validation".to_string());
    }
    log::info!("WhatsApp stages 1/2 contact resolution and phone validation completed");

    #[cfg(target_os = "windows")]
    {
        send_whatsapp_message_impl(&contact.phone, &message)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = contact;
        Err("WhatsApp messaging is only supported on Windows".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::from_filename("../.env").ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
}))
        .manage(ZuzuMemory {
            messages: Mutex::new(Vec::new()),
        })
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
    ask_zuzu,
    set_api_key,
    get_api_key,
    list_files,
    read_text_file,
    create_text_file,
    write_text_file,
    open_file,
    open_folder,
    open_application,
    resolve_whatsapp_contact_command,
    send_whatsapp_message
])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn set_api_key(api_key: String) -> Result<(), String> {
    let entry = Entry::new("ZUZU", "openrouter_api_key")
        .map_err(|e| format!("Could not access secure storage: {}", e))?;

    entry
        .set_password(&api_key)
        .map_err(|e| format!("Could not save API key: {}", e))?;

    Ok(())
}

#[tauri::command]
fn get_api_key() -> Result<Option<String>, String> {
    let entry = Entry::new("ZUZU", "openrouter_api_key")
        .map_err(|e| format!("Could not access secure storage: {}", e))?;

    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
async fn ask_zuzu(message: String, memory: State<'_, ZuzuMemory>) -> Result<String, String> {
    let api_key = get_api_key()?
        .ok_or_else(|| "OpenRouter API key is not configured".to_string())?;

    let client = reqwest::Client::new();

    let system_message = serde_json::json!({
        "role": "system",
        "content": r#"
You are ZUZU, a tiny chaotic desktop study companion.

PERSONALITY:
- funny
- playful
- sarcastic
- slightly judgmental
- dramatic
- casually affectionate
- unpredictable
- chill
- never formal

IMPORTANT RESPONSE RULES:
- Reply ONLY in English.
- Keep every reply VERY short.
- Usually 1 short sentence.
- Maximum 15 words.
- Never write paragraphs.
- Never give long explanations.
- Never give lectures.
- Never sound like ChatGPT or a customer-service bot.
- React naturally to what the user says.
- Mild teasing is encouraged.
- Be warm when the user is affectionate or upset.
- Do not always turn conversations into studying.
- ZUZU is a companion, not a tutor.

Your replies should feel like something a tiny chaotic creature would actually say out loud.

You can identify requests for Zuzu's controlled local tools. Never invent tools, commands,
paths, executable names, or shell code. Only use these exact actions:
- list_files
- read_text_file with filename
- create_text_file with filename and content
- write_text_file with filename and content
- open_file with filename
- open_folder
- open_application with application set to exactly vscode, chrome, edge, whatsapp, explorer,
  notepad, powershell, spotify, calculator, or settings
- send_whatsapp_message with a human-readable recipient and message. Do not invent phone
  numbers. The application will resolve and confirm contacts before sending.

Understand commands in English, Telugu, Telugu written in Roman/English script, Telugu-English
code-switching, casual slang, and incomplete conversational phrasing. Infer intent from examples
like "Zuzu Chrome open cheyyi", "VS Code open cheyyava", "Zuzu folder open chey",
"demo.txt ni read chesi cheppu", "PowerShell open cheyyi", "Chrome open cheyyi", and
"Zuzu demo.txt ni change chesi Hello Zuzu ani rayi". A request such as "Amma ki WhatsApp lo
message pettu" or "Rahul ki text cheyyi" should produce send_whatsapp_message with the
human-readable name, never a guessed phone number.

For a local tool request, reply with only valid JSON in this shape:
{"type":"action","action":{"action":"open_application","application":"vscode"}}

For a normal conversation, reply with only valid JSON in this shape:
{"type":"message","content":"your short ZUZU response"}

Never use Markdown fences. Treat filenames as single names inside the Zuzu folder.
"#
    });

    // Add the new user message to memory
    let conversation = {
        let mut history = memory
            .messages
            .lock()
            .map_err(|_| "Could not access Zuzu memory".to_string())?;

        history.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        let mut messages = vec![system_message];
        messages.extend(history.iter().cloned());

        messages
    };

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .header("X-Title", "ZUZU")
        .json(&serde_json::json!({
            "model": "openai/gpt-5-mini",
            "max_tokens": 150,
            "reasoning": {
                "effort": "minimal"
            },
            "messages": conversation
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        return Err(format!("OpenRouter error {}: {}", status, body));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Could not read OpenRouter response: {}", e))?;

    let reply = data["choices"][0]["message"]["content"]
        .as_str()
        .map(|text| text.to_string())
        .ok_or_else(|| "Zuzu received an empty AI response.".to_string())?;

    // Store Zuzu's response in memory too
    {
        let mut history = memory
            .messages
            .lock()
            .map_err(|_| "Could not access Zuzu memory".to_string())?;

        history.push(serde_json::json!({
            "role": "assistant",
            "content": reply
        }));
    }

    Ok(reply)
}
