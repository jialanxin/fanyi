use anyhow::{Context, Result};
use colored::ColoredString;
use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

/// 免 API key 的公共查询接口（参考 afc163/fanyi 的做法，开箱即用无需配置）。
const ICIBA_URL: &str = "https://dict-mobile.iciba.com/interface/index.php";
const YOUDAO_URL: &str = "https://dict.youdao.com/jsonapi";

// ---------- iciba ----------

#[derive(Debug, Deserialize)]
struct IcibaMean {
    #[serde(default)]
    part: Option<String>,
    #[serde(default)]
    means: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct IcibaItem {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    paraphrase: Option<String>,
    #[serde(default)]
    means: Vec<IcibaMean>,
}

#[derive(Debug, Deserialize)]
struct IcibaResponse {
    #[serde(default)]
    message: Vec<IcibaItem>,
}

fn gray_suffix(source: &str) -> ColoredString {
    format!("  ~  {source}").truecolor(128, 128, 128)
}

/// 将 iciba 响应渲染为输出文本（纯函数，便于离线测试）。
fn render_iciba(word: &str, resp: &IcibaResponse) -> String {
    let mut out = String::new();
    if resp.message.is_empty() {
        out.push_str(&format!("未找到 {word} 的释义{}\n", gray_suffix("iciba.com")));
        return out;
    }

    // 优先取与查询词完全一致的条目，否则取第一条
    let lower = word.to_lowercase();
    let matched = resp
        .message
        .iter()
        .find(|item| {
            item.key
                .as_deref()
                .map(|k| k.eq_ignore_ascii_case(&lower))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| &resp.message[0]);

    let matched_key = matched.key.as_deref().unwrap_or(word);
    out.push_str(&format!(
        "{} {}\n",
        Colorize::green(matched_key),
        gray_suffix("iciba.com")
    ));

    let meanings: Vec<String> = matched
        .means
        .iter()
        .filter_map(|mean| {
            let meaning = mean.means.join("；");
            if meaning.is_empty() {
                return None;
            }
            match &mean.part {
                Some(part) if !part.is_empty() => {
                    Some(format!("{} {}", Colorize::magenta(part.as_str()), meaning))
                }
                _ => Some(meaning),
            }
        })
        .collect();

    if !meanings.is_empty() {
        out.push('\n');
        for line in &meanings {
            out.push_str(&format!("{}{}\n", " - ".truecolor(128, 128, 128), line));
        }
    }

    // 相关联想词：其余带释义的条目
    let related: Vec<&IcibaItem> = resp
        .message
        .iter()
        .filter(|item| {
            !std::ptr::eq(*item, matched)
                && item.key.is_some()
                && item
                    .paraphrase
                    .as_deref()
                    .map(|p| !p.is_empty())
                    .unwrap_or(false)
        })
        .collect();

    if !related.is_empty() {
        out.push('\n');
        for item in related {
            out.push_str(&format!(
                "{}  {}\n",
                Colorize::cyan(item.key.as_deref().unwrap_or_default()),
                item.paraphrase
                    .as_deref()
                    .unwrap_or_default()
                    .truecolor(128, 128, 128)
            ));
        }
    }

    out
}

async fn lookup_iciba(client: &Client, word: &str) -> Result<String> {
    let resp: IcibaResponse = client
        .get(ICIBA_URL)
        .query(&[
            ("c", "word"),
            ("m", "getsuggest"),
            ("nums", "10"),
            ("is_need_mean", "1"),
            ("word", word),
        ])
        .send()
        .await
        .context("访问 iciba 失败，请检查网络")?
        .error_for_status()
        .context("iciba 返回错误状态")?
        .json()
        .await
        .context("解析 iciba 响应失败")?;
    Ok(render_iciba(word, &resp))
}

// ---------- youdao ----------

fn is_chinese(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

/// 从 jsonapi 词条中提取一条释义文本（l.i 数组元素可能是字符串或 {"#text": ...}）。
fn sense_text(item: &Value) -> Option<String> {
    let parts = item.get("l")?.get("i")?.as_array()?;
    let text: String = parts
        .iter()
        .filter_map(|piece| {
            if let Some(s) = piece.as_str() {
                Some(s.to_string())
            } else {
                piece.get("#text")?.as_str().map(|s| s.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// 将有道 jsonapi 响应渲染为输出文本（纯函数，便于离线测试）。
fn render_youdao(word: &str, data: &Value) -> String {
    let mut out = String::new();
    // 英译中用 ec，中译英用 ce
    let dict = if is_chinese(word) { &data["ce"] } else { &data["ec"] };
    let words = match dict.get("word").and_then(Value::as_array) {
        Some(w) if !w.is_empty() => w,
        _ => {
            out.push_str(&format!(
                "未找到 {word} 的释义{}\n",
                gray_suffix("youdao.com")
            ));
            return out;
        }
    };
    let entry = &words[0];

    let phrase = entry["returnPhrase"]
        .as_str()
        .or_else(|| entry["return-phrase"]["l"]["i"].as_str())
        .or_else(|| entry["return-phrase"].as_str())
        .unwrap_or(word);

    let phonetic = if is_chinese(word) {
        entry["phone"].as_str().map(|p| format!("[{p}]"))
    } else {
        let us = entry["usphone"].as_str();
        let uk = entry["ukphone"].as_str();
        match (us, uk) {
            (Some(us), Some(uk)) => Some(format!("英 [{uk}]  美 [{us}]")),
            (Some(us), None) => Some(format!("美 [{us}]")),
            (None, Some(uk)) => Some(format!("英 [{uk}]")),
            (None, None) => None,
        }
    };

    match phonetic {
        Some(p) => out.push_str(&format!(
            "{} {} {}\n",
            Colorize::yellow(phrase),
            Colorize::blue(p.as_str()),
            gray_suffix("youdao.com")
        )),
        None => out.push_str(&format!(
            "{} {}\n",
            Colorize::yellow(phrase),
            gray_suffix("youdao.com")
        )),
    }

    if let Some(trs) = entry.get("trs").and_then(Value::as_array) {
        let mut lines: Vec<ColoredString> = Vec::new();
        for tr in trs {
            if let Some(items) = tr.get("tr").and_then(Value::as_array) {
                for item in items {
                    if let Some(text) = sense_text(item) {
                        lines.push(Colorize::green(text.as_str()));
                    }
                }
            }
        }
        if !lines.is_empty() {
            out.push('\n');
            for line in lines {
                out.push_str(&format!("{}{}\n", " - ".truecolor(128, 128, 128), line));
            }
        }
    }

    out
}

async fn lookup_youdao(client: &Client, word: &str) -> Result<String> {
    let data: Value = client
        .get(YOUDAO_URL)
        .query(&[("q", word)])
        .send()
        .await
        .context("访问 youdao 失败，请检查网络")?
        .error_for_status()
        .context("youdao 返回错误状态")?
        .json()
        .await
        .context("解析 youdao 响应失败")?;
    Ok(render_youdao(word, &data))
}

// ---------- main ----------

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1].trim().is_empty() {
        eprintln!("用法: fanyi <要翻译的单词或词组>");
        std::process::exit(1);
    }
    let input = args[1..].join(" ");

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("fanyi-rs")
        .build()?;

    // 并发查询两个数据源，按固定顺序输出，避免交错
    let (youdao_out, iciba_out) = tokio::join!(
        lookup_youdao(&client, &input),
        lookup_iciba(&client, &input)
    );

    println!();
    match youdao_out {
        Ok(text) => print!("{text}"),
        Err(err) => println!("{err:#}{}", gray_suffix("youdao.com")),
    }
    println!("\n   {}\n", "------".truecolor(128, 128, 128));
    match iciba_out {
        Ok(text) => print!("{text}"),
        Err(err) => println!("{err:#}{}", gray_suffix("iciba.com")),
    }
    println!("\n   {}\n", "------".truecolor(128, 128, 128));
    Ok(())
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_iciba_meanings_and_related() {
        let payload = r#"{
            "message": [
                {"key": "good", "paraphrase": "好的", "means": [
                    {"part": "adj.", "means": ["好的", "优良的"]},
                    {"part": "n.", "means": ["好处"]}
                ]},
                {"key": "goods", "paraphrase": "商品", "means": []},
                {"key": "goose", "paraphrase": "鹅", "means": []}
            ]
        }"#;
        let resp: IcibaResponse = serde_json::from_str(payload).unwrap();
        let text = render_iciba("good", &resp);
        assert!(text.contains("good"));
        assert!(text.contains("adj. 好的；优良的"));
        assert!(text.contains("n. 好处"));
        assert!(text.contains("goods"));
        assert!(text.contains("商品"));
        assert!(text.contains("iciba.com"));
    }

    #[test]
    fn renders_iciba_not_found() {
        let resp: IcibaResponse = serde_json::from_str(r#"{"message": []}"#).unwrap();
        let text = render_iciba("zzzz", &resp);
        assert!(text.contains("未找到"));
    }

    #[test]
    fn renders_youdao_english_word() {
        let data: Value = serde_json::from_str(
            r#"{"ec": {"word": [{
                "return-phrase": {"l": {"i": "good"}},
                "usphone": "ɡʊd", "ukphone": "ɡʊd",
                "trs": [{"tr": [{"l": {"i": ["adj. 好的"]}}, {"l": {"i": ["n. 好处"]}}]}]
            }]}}"#,
        )
        .unwrap();
        let text = render_youdao("good", &data);
        assert!(text.contains("英 [ɡʊd]  美 [ɡʊd]"));
        assert!(text.contains("adj. 好的"));
        assert!(text.contains("n. 好处"));
        assert!(text.contains("youdao.com"));
    }

    #[test]
    fn renders_youdao_chinese_word_uses_ce() {
        let data: Value = serde_json::from_str(
            r#"{"ce": {"word": [{
                "return-phrase": {"l": {"i": "好"}},
                "phone": "hǎo",
                "trs": [{"tr": [{"l": {"i": ["good; fine"]}}]}]
            }]}, "ec": {"word": []}}"#,
        )
        .unwrap();
        let text = render_youdao("好", &data);
        assert!(text.contains("[hǎo]"));
        assert!(text.contains("good; fine"));
    }

    #[test]
    fn renders_youdao_not_found() {
        let data: Value = serde_json::from_str(r#"{}"#).unwrap();
        let text = render_youdao("qqqq", &data);
        assert!(text.contains("未找到"));
    }
}
