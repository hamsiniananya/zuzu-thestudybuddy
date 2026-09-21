use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use keyring::Entry;
use tauri::{Manager, State};

struct ZuzuMemory {
    messages: Mutex<Vec<serde_json::Value>>,
}

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
        "Visual Studio Code" => vec![
            local_app_data.map(|path| path.join("Programs\\Microsoft VS Code\\Code.exe")),
            program_files.map(|path| path.join("Microsoft VS Code\\Code.exe")),
        ],
        "Google Chrome" => vec![
            local_app_data.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
            program_files.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
            program_files_x86.map(|path| path.join("Google\\Chrome\\Application\\chrome.exe")),
        ],
        "Microsoft Edge" => vec![
            program_files.map(|path| path.join("Microsoft\\Edge\\Application\\msedge.exe")),
            program_files_x86.map(|path| path.join("Microsoft\\Edge\\Application\\msedge.exe")),
        ],
        "Windows File Explorer" => vec![
            windows.map(|path| path.join("explorer.exe")),
        ],
        "Notepad" => vec![
            windows.map(|path| path.join("System32\\notepad.exe")),
        ],
        _ => return Err("Unsupported application".to_string()),
    };

    candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .ok_or_else(|| format!("Could not find {}", application))
}

#[tauri::command]
fn open_application(application: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
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
    open_application
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
