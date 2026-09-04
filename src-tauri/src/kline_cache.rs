// K 线数据磁盘持久化缓存（L2）。
// 存储位置：app_data_dir()/kline_cache/{code}_{period}.json
// 文件格式：{ "code", "period", "fetched_at"(unix_ms), "bars": [...] }
// 容错原则：读/写/解析任何失败均不阻断主流程（当作无缓存处理），仅 eprintln 记录。
// 原语均以缓存目录 &Path 为参数（AppHandle 仅在 cache_dir 中触碰 Tauri 运行时），
// 使 market.rs 的 fetch 层保持与 Tauri 运行时解耦、可独立测试。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::market::KlineBar;

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedKline {
    pub code: String,
    pub period: String,
    pub fetched_at: u64,
    pub bars: Vec<KlineBar>,
}

// 解析缓存根目录（app_data_dir/kline_cache），目录不存在则创建。
// 失败转为 String 错误，由命令层传 None 等价禁用缓存。
pub fn cache_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取 app_data_dir 失败: {}", e))?
        .join("kline_cache");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建缓存目录失败: {}: {}", dir.display(), e))?;
    Ok(dir)
}

fn cache_file(dir: &Path, code: &str, period: &str) -> PathBuf {
    dir.join(format!("{}_{}.json", code, period))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// 读取缓存；文件不存在或解析失败一律返回 None（当作无缓存回源）。
pub fn read_cache(dir: &Path, code: &str, period: &str) -> Option<CachedKline> {
    let file = cache_file(dir, code, period);
    let raw = match std::fs::read(&file) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("[kline_cache] 读取失败 {}: {}", file.display(), e);
            return None;
        }
    };
    match serde_json::from_slice::<CachedKline>(&raw) {
        Ok(entry) => Some(entry),
        Err(e) => {
            eprintln!("[kline_cache] 解析失败 {}: {}", file.display(), e);
            None
        }
    }
}

// 缓存是否仍新鲜：fetched_at 距今不超过 ttl_ms（fetched_at=0 视为无效陈旧数据）。
pub fn is_fresh(entry: &CachedKline, ttl_ms: u64) -> bool {
    entry.fetched_at > 0 && now_ms().saturating_sub(entry.fetched_at) <= ttl_ms
}

// 原子写缓存：先写 .tmp 再 rename 覆盖，防进程中断写坏半截文件。
// 任何失败仅记录日志并静默返回（缓存写失败不应影响主流程，不 panic、不返回 Err）。
pub fn write_cache_atomic(dir: &Path, code: &str, period: &str, bars: Vec<KlineBar>) {
    let entry = CachedKline {
        code: code.to_string(),
        period: period.to_string(),
        fetched_at: now_ms(),
        bars,
    };
    let json = match serde_json::to_vec(&entry) {
        Ok(json) => json,
        Err(e) => {
            eprintln!(
                "[kline_cache] 序列化失败 code={} period={}: {}",
                code, period, e
            );
            return;
        }
    };
    let file = cache_file(dir, code, period);
    let tmp = dir.join(format!("{}_{}.json.tmp", code, period));
    if let Err(e) = std::fs::write(&tmp, &json) {
        eprintln!("[kline_cache] 写入临时文件失败 {}: {}", tmp.display(), e);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &file) {
        eprintln!(
            "[kline_cache] rename 失败 {} -> {}: {}",
            tmp.display(),
            file.display(),
            e
        );
        // 清理残留 tmp，避免缓存目录堆积垃圾文件
        let _ = std::fs::remove_file(&tmp);
    }
}

// 按周期差异化 TTL：day 盘中最后一根 bar 持续变化用最短；week/month 变化频率低放宽。
pub fn period_ttl_ms(period: &str) -> u64 {
    match period {
        "week" => 600_000,    // 10 分钟
        "month" => 1_800_000, // 30 分钟
        _ => 60_000,          // day 及未知周期统一 60 秒
    }
}
