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

// 落盘诊断日志。真机沙盒中 /tmp 与 /var/mobile/Containers/Data/Application 均不可写，
// 只有 $HOME/Documents 可用，且能被 Xcode「Devices and Simulators → 下载容器」导出。
// stderr 由 Tauri 的 log_stdout() 转发到设备控制台，devicectl --console 可直接看到。
fn persist_log(name: &str, msg: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let dir = std::path::PathBuf::from(home).join("Documents");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(name), msg);
    }
    let _ = std::fs::write(std::path::Path::new("/tmp").join(name), msg);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 注意：mobile_entry_point 展开出的 stop_unwind 会在捕获 panic 后调用 abort()，
    // 因此 iOS 上任何 panic 都表现为「闪退」，日志是唯一的定位依据。
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}\n", info);
        eprintln!("{}", msg);
        persist_log("tauri_panic.log", &msg);
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
            persist_log("tauri_error.log", &msg);
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
