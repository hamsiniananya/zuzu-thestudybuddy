#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::from_filename("../.env").ok();

    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![ask_zuzu])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn ask_zuzu(message: String) -> Result<String, String> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "OPENROUTER_API_KEY is not set".to_string())?;

    let client = reqwest::Client::new();

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
    "messages": [
        {
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
        },
        {
            "role": "user",
            "content": message
        }
    ]
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

        return Err(format!(
            "OpenRouter error {}: {}",
            status, body
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Could not read OpenRouter response: {}", e))?;

    data["choices"][0]["message"]["content"]
        .as_str()
        .map(|text| text.to_string())
        .ok_or_else(|| "Zuzu received an empty AI response.".to_string())
}