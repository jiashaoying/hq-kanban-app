mod kline_cache;
mod market;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn fetch_indices() -> Result<Vec<market::IndexData>, String> {
    market::fetch_all_indices().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch_minute_data(code: String) -> Result<market::MinuteData, String> {
    market::fetch_minute_data(&code)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch_kline_data(
    app: tauri::AppHandle,
    code: String,
    period: String,
    count: Option<u32>,
    prefer_cache: Option<bool>,
) -> Result<market::KlineData, String> {
    let count = count.unwrap_or(320);
    // AppHandle 由 Tauri 自动注入，前端 invoke 契约不变。
    // 缓存目录获取失败等价于禁用缓存（传 None，不影响主流程）。
    // prefer_cache 经 Tauri 2 camelCase 映射自动对应 JS 端 preferCache，
    // 与 code/period/count 同一命名规则，无需额外处理。
    let cache_dir = kline_cache::cache_dir(&app).ok();
    market::fetch_kline_data(
        &code,
        &period,
        count,
        prefer_cache,
        cache_dir.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Set up panic hook to write to a file we can read
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}\n", info);
        eprintln!("{}", msg);
        // Try to write to simulator's shared directory
        let _ = std::fs::write("/tmp/tauri_panic.log", &msg);
        let _ = std::fs::write("/var/mobile/Containers/Data/Application/tauri_panic.log", &msg);
    }));

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            fetch_indices,
            fetch_minute_data,
            fetch_kline_data
        ])
        .run(tauri::generate_context!());

    match result {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("TAURI_APP_ERROR: {:?}\n", e);
            eprintln!("{}", msg);
            let _ = std::fs::write("/tmp/tauri_error.log", &msg);
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
