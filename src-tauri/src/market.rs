use serde::Serialize;

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
    let response = reqwest::get(&url).await?;
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
