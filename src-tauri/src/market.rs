use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Serialize, Clone)]
pub struct IndexData {
    pub code: String,
    pub name: String,
    pub current: f64,
    pub prev_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub amount: f64,
    pub market: String,
    pub update_time: String,
}

const INDEX_CODES: &str = "sh000001,sz399001,sz399006,sh000300,sh000016,sh000905,r_hkHSI,r_hkHSTECH,r_us.DJI,r_us.IXIC,r_us.INX";

fn parse_f64(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    s.parse::<f64>().unwrap_or(0.0)
}

fn market_type(code: &str) -> String {
    if code.starts_with("sh") || code.starts_with("sz") {
        "a".to_string()
    } else if code.starts_with("r_hk") {
        "hk".to_string()
    } else if code.starts_with("r_us") {
        "us".to_string()
    } else {
        "unknown".to_string()
    }
}

pub async fn fetch_all_indices() -> Result<Vec<IndexData>, Box<dyn std::error::Error>> {
    let url = format!("https://qt.gtimg.cn/q={}", INDEX_CODES);
    let response = http_client().get(&url).send().await?;
    let bytes = response.bytes().await?;
    let (text, _, _) = encoding_rs::GBK.decode(&bytes);

    let mut results = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 提取双引号内的内容: v_sh000001="...";
        let parts: Vec<&str> = line.split('"').collect();
        if parts.len() < 2 {
            continue;
        }
        let content = parts[1];

        // 提取指数代码: v_sh000001= 中的 sh000001
        let eq_parts: Vec<&str> = line.split('=').collect();
        if eq_parts.is_empty() {
            continue;
        }
        let code_raw = eq_parts[0].trim(); // v_sh000001
        let index_code = if code_raw.starts_with("v_") {
            &code_raw[2..]
        } else {
            continue;
        };

        let fields: Vec<&str> = content.split('~').collect();
        if fields.len() < 38 {
            continue;
        }

        let data = IndexData {
            code: index_code.to_string(),
            name: fields[1].to_string(),
            current: parse_f64(fields[3]),
            prev_close: parse_f64(fields[4]),
            open: parse_f64(fields[5]),
            high: parse_f64(fields[33]),
            low: parse_f64(fields[34]),
            change: parse_f64(fields[31]),
            change_pct: parse_f64(fields[32]),
            volume: parse_f64(fields[36]),
            amount: parse_f64(fields[37]),
            market: market_type(index_code),
            update_time: fields[30].to_string(),
        };

        results.push(data);
    }

    Ok(results)
}

// ===== 指数详情页：分时与K线数据（腾讯 ifzq 系 UTF-8 JSON 接口）=====

#[derive(Serialize, Clone)]
pub struct MinutePoint {
    pub time: String,
    pub price: f64,
    pub avg_price: f64,
    pub volume: f64,
}

#[derive(Serialize, Clone)]
pub struct MinuteData {
    pub code: String,
    pub date: String,
    pub prev_close: f64,
    pub points: Vec<MinutePoint>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KlineBar {
    pub date: String,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

#[derive(Serialize, Clone)]
pub struct KlineData {
    pub code: String,
    pub period: String,
    pub bars: Vec<KlineBar>,
}

// 进程级 reqwest 客户端单例（连接池复用，避免每次请求重建 TCP/TLS 握手）。
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client init failed")
    })
}

// ===== IPC 输入白名单校验 =====
// code/period/count 由前端经 IPC 传入后被 format! 直接拼入上游 URL，
// 必须在入口做白名单校验，阻断 URL 注入与非法参数。
// 注：内部代码含 r_ 前缀（如 r_hkHSI、r_us.DJI），故除字母数字与点号外还允许下划线。
const MAX_KLINE_COUNT: u32 = 2000;

fn validate_code(code: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !code.is_empty()
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    {
        Ok(())
    } else {
        Err(format!(
            "非法指数代码: {:?}（仅允许字母、数字、点号及 r_ 前缀下划线）",
            code
        )
        .into())
    }
}

fn validate_period(period: &str) -> Result<(), Box<dyn std::error::Error>> {
    match period {
        "day" | "week" | "month" => Ok(()),
        other => Err(format!("非法K线周期: {:?}（仅支持 day/week/month）", other).into()),
    }
}

fn validate_kline_count(count: u32) -> Result<(), Box<dyn std::error::Error>> {
    if count >= 1 && count <= MAX_KLINE_COUNT {
        Ok(())
    } else {
        Err(format!(
            "K线数量超出范围: {}（须为 1~{}）",
            count, MAX_KLINE_COUNT
        )
        .into())
    }
}

// 将内部指数代码归一化为 ifzq 系接口使用的格式。
// 2026-09 实测定稿：sh*/sz* 原样；r_hkHSI→hkHSI；r_us.DJI→us.DJI（统一去 r_ 前缀）。
// 美股保留点号格式：K线接口 us.DJI 可返回 320 根完整历史，usDJI 仅返回 1 根。
fn ifzq_code(code: &str) -> String {
    match code.strip_prefix("r_") {
        Some(stripped) => stripped.to_string(),
        None => code.to_string(),
    }
}

// 拉取指数分时数据（minute/query 接口，UTF-8 JSON，无需 GBK 解码）。
// 数据行格式（空白切分）："HHMM 价格 累计量 累计额"；
// 美股行仅前三列（无累计额），休市时常仅 1 条且 date 为空。
pub async fn fetch_minute_data(code: &str) -> Result<MinuteData, Box<dyn std::error::Error>> {
    validate_code(code)?;
    let ifzq = ifzq_code(code);
    let url = format!(
        "https://web.ifzq.gtimg.cn/appstock/app/minute/query?code={}",
        ifzq
    );
    let response = http_client().get(&url).send().await?;
    let bytes = response.bytes().await?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;

    let node = value
        .get("data")
        .and_then(|d| d.get(&ifzq))
        .ok_or_else(|| format!("分时响应缺少 data.{} 节点: code={}", ifzq, code))?;

    let data_node = node
        .get("data")
        .ok_or_else(|| format!("分时响应缺少 data.{}.data 节点: code={}", ifzq, code))?;

    let date = data_node
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let rows = data_node
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("分时响应缺少 data.{}.data.data 数组: code={}", ifzq, code))?;

    let mut points: Vec<MinutePoint> = Vec::with_capacity(rows.len());
    let mut prev_cum_volume = 0.0f64;

    for (i, row) in rows.iter().enumerate() {
        let line = match row.as_str() {
            Some(s) => s,
            None => continue,
        };
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        // "HHMM" → "HH:MM"（is_ascii 保护，避免非 ASCII 切片 panic）
        let hhmm = fields[0];
        let time = if hhmm.len() >= 4 && hhmm.is_ascii() {
            format!("{}:{}", &hhmm[0..2], &hhmm[2..4])
        } else {
            hhmm.to_string()
        };
        let price = parse_f64(fields[1]);
        let cum_volume = parse_f64(fields[2]);
        // 累计量 → 分钟增量：首条为 0，后续防乱序负数取 0
        let volume = if i == 0 {
            0.0
        } else {
            (cum_volume - prev_cum_volume).max(0.0)
        };
        // 均价 = 累计额 / 累计量；除零或累计额列缺失（美股）取 0.0
        let avg_price = if fields.len() >= 4 && cum_volume > 0.0 {
            parse_f64(fields[3]) / cum_volume
        } else {
            0.0
        };
        prev_cum_volume = cum_volume;
        points.push(MinutePoint {
            time,
            price,
            avg_price,
            volume,
        });
    }

    if points.is_empty() {
        return Err(format!("分时数据为空: code={}", ifzq).into());
    }

    // 前收盘：2026-09 实测 qt[86] 不可用（A股恒为 '0'，港股数组长度 78、美股 71 均越界），
    // 真实前收盘在 qt[4]（三市场一致，与 qt.gtimg.cn 的 fields[4] 相互印证）。
    // 按计划以 [86] 为主索引、[4] 兜底，均静默容错。
    let qt_at = |i: usize| -> f64 {
        node.get("qt")
            .and_then(|q| q.get(&ifzq))
            .and_then(|q| q.as_array())
            .and_then(|arr| arr.get(i))
            .and_then(|v| v.as_str())
            .map(parse_f64)
            .unwrap_or(0.0)
    };
    let prev_close = {
        let by_86 = qt_at(86);
        if by_86 > 0.0 {
            by_86
        } else {
            qt_at(4)
        }
    };

    Ok(MinuteData {
        code: code.to_string(),
        date,
        prev_close,
        points,
    })
}

// 拉取指数K线数据（fqkline/get 接口，UTF-8 JSON）+ L2 磁盘缓存编排：
// 新鲜缓存直接返回 → 回源拉取（成功覆盖写缓存）→ 网络失败回退陈旧缓存。
// prefer_cache == Some(true) 为「缓存先行」模式：完成白名单校验后读磁盘缓存，
// 存在即无论 TTL 直接返回（不发网络）；不存在返回 Err。供前端
// 「缓存先行渲染 + 网络回来更新」两阶段加载的第一阶段使用。
// 行字段序实测为 [日期, 开, 收, 高, 低, 量, ...附加字段]（注意是开收高低序），
// 元素均为字符串；响应数组 key 按 qfq{period} → {period} 链式兜底（实测请求 qfq 时返回裸 period）。
pub async fn fetch_kline_data(
    code: &str,
    period: &str,
    count: u32,
    prefer_cache: Option<bool>,
    cache_dir: Option<&std::path::Path>,
) -> Result<KlineData, Box<dyn std::error::Error>> {
    validate_code(code)?;
    validate_period(period)?;
    validate_kline_count(count)?;

    // 缓存先行模式：命中即返回（无视 TTL），未命中直接 Err，全程不发网络。
    // 缓存读放在白名单校验之后：code 会拼入缓存文件名，先校验可阻断路径穿越。
    if matches!(prefer_cache, Some(true)) {
        return match cache_dir.and_then(|dir| crate::kline_cache::read_cache(dir, code, period)) {
            Some(entry) => Ok(KlineData {
                code: entry.code,
                period: entry.period,
                bars: entry.bars,
            }),
            None => Err(format!("no cached kline: code={} period={}", code, period).into()),
        };
    }

    // L2 缓存命中且新鲜 → 直接返回，不发网络请求。
    if let Some(dir) = cache_dir {
        if let Some(entry) = crate::kline_cache::read_cache(dir, code, period) {
            if crate::kline_cache::is_fresh(&entry, crate::kline_cache::period_ttl_ms(period)) {
                return Ok(KlineData {
                    code: entry.code,
                    period: entry.period,
                    bars: entry.bars,
                });
            }
        }
    }

    match fetch_kline_remote(code, period, count).await {
        Ok(data) => {
            // 网络成功 → 整份覆盖写缓存（原子写，失败静默不影响返回）；
            // 35KB 级写入亚毫秒，直接同步 IO 无需 spawn_blocking，以简洁为准。
            if let Some(dir) = cache_dir {
                crate::kline_cache::write_cache_atomic(dir, code, period, data.bars.clone());
            }
            Ok(data)
        }
        Err(err) => {
            // 网络失败 → 回退陈旧缓存（无论过期与否，离线/弱网可用）；无缓存保持 Err。
            if let Some(dir) = cache_dir {
                if let Some(entry) = crate::kline_cache::read_cache(dir, code, period) {
                    eprintln!(
                        "[kline_cache] 网络失败，回退陈旧缓存: code={} period={} ({})",
                        code, period, err
                    );
                    return Ok(KlineData {
                        code: entry.code,
                        period: entry.period,
                        bars: entry.bars,
                    });
                }
            }
            Err(err)
        }
    }
}

// 原有网络请求与解析逻辑整体平移至此（URL 拼接、解析零改动），
// 由 fetch_kline_data 的缓存编排层调用，便于在 Ok/Err 分支做缓存写与陈旧回退。
async fn fetch_kline_remote(
    code: &str,
    period: &str,
    count: u32,
) -> Result<KlineData, Box<dyn std::error::Error>> {
    let ifzq = ifzq_code(code);
    let url = format!(
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={},{},,,{},qfq",
        ifzq, period, count
    );
    let response = http_client().get(&url).send().await?;
    let bytes = response.bytes().await?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;

    let node = value
        .get("data")
        .and_then(|d| d.get(&ifzq))
        .ok_or_else(|| format!("K线响应缺少 data.{} 节点: code={}", ifzq, code))?;

    let rows = ["qfq", ""]
        .iter()
        .filter_map(|prefix| node.get(format!("{}{}", prefix, period)))
        .find_map(|v| v.as_array())
        .ok_or_else(|| format!("K线响应缺少 {} 周期数组: code={}", period, ifzq))?;

    let mut bars: Vec<KlineBar> = Vec::with_capacity(rows.len());

    for row in rows {
        // 跳过嵌套数组以外的异常元素（响应节点下另有 prec/mx_price 等标量或数组字段）
        let fields = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        if fields.len() < 6 {
            continue;
        }
        // 元素实测均为字符串，仍兼容数字；忽略第 6 列之后的附加字段
        let num = |i: usize| -> f64 {
            fields
                .get(i)
                .and_then(|v| v.as_str().map(parse_f64).or_else(|| v.as_f64()))
                .unwrap_or(0.0)
        };
        // date 为空会使前端 lightweight-charts setData 运行时抛异常，跳过该行
        let date = fields[0].as_str().unwrap_or("");
        if date.is_empty() {
            continue;
        }
        bars.push(KlineBar {
            date: date.to_string(),
            open: num(1),
            close: num(2),
            high: num(3),
            low: num(4),
            volume: num(5),
        });
    }

    if bars.is_empty() {
        return Err(format!("K线数据为空: code={} period={}", ifzq, period).into());
    }

    Ok(KlineData {
        code: code.to_string(),
        period: period.to_string(),
        bars,
    })
}
