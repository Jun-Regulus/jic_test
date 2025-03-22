use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use regex::Regex;
use lazy_static::lazy_static;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigValue {
    String(String),
    Map(HashMap<String, ConfigValue>),
}

lazy_static! {
    static ref CONFIG_REGEX: Regex = Regex::new(r"^\s*([a-zA-Z0-9._-]+)\s*=\s*(.+?)\s*$").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"^\s*#").unwrap();
}

fn parse_config_file(file_path: &Path) -> io::Result<HashMap<String, ConfigValue>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut config = HashMap::new();

    for line in reader.lines().flatten() {
        let trimmed_line = line.trim();
        if COMMENT_REGEX.is_match(trimmed_line) || trimmed_line.is_empty() {
            continue; // コメント行・空行をスキップ
        }

        if let Some(captures) = CONFIG_REGEX.captures(trimmed_line) {
            let key = captures[1].to_string();
            let raw_value = captures[2].trim().to_string();
            insert_config_value(&mut config, &key, ConfigValue::String(raw_value));
        }
    }

    Ok(config)
}

fn insert_config_value(config: &mut HashMap<String, ConfigValue>, key: &str, value: ConfigValue) {
    let keys: Vec<&str> = key.split('.').collect();
    let mut map = config;

    for sub_key in &keys[..keys.len() - 1] {
        map = map.entry(sub_key.to_string())
            .or_insert_with(|| ConfigValue::Map(HashMap::new()))
            .as_map_mut()
            .expect("型の不一致");
    }

    map.insert(keys.last().unwrap().to_string(), value);
}

fn collect_text_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if path.is_dir() {
        return Ok(fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| p.is_file())
            .collect());
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "パスが見つかりません"))
}

fn get_text_files(args: &[String]) -> Vec<PathBuf> {
    if args.is_empty() {
        eprintln!("ファイルを指定してください。");
        std::process::exit(1);
    }

    args.iter()
        .flat_map(|arg| collect_text_files(Path::new(arg)).unwrap_or_default())
        .collect()
}

fn format_as_json(config: &HashMap<String, ConfigValue>) -> serde_json::Value {
    let mut json_obj = serde_json::Map::new();
    for (key, value) in config {
        match value {
            ConfigValue::String(s) => {
                json_obj.insert(key.clone(), json!(s));
            }
            ConfigValue::Map(m) => {
                json_obj.insert(key.clone(), format_as_json(m));
            }
        }
    }
    serde_json::Value::Object(json_obj)
}

fn main() {
    let text_files = get_text_files(&env::args().skip(1).collect::<Vec<_>>());

    for file_path in text_files {
        println!("=== ファイル: {} ===", file_path.display());
        match parse_config_file(&file_path) {
            Ok(config) => {
                let json_output = format_as_json(&config);
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            }
            Err(e) => eprintln!("ファイルの読み込みエラー: {} ({})", e, file_path.display()),
        }
    }
}

impl ConfigValue {
    fn as_map_mut(&mut self) -> Option<&mut HashMap<String, ConfigValue>> {
        if let ConfigValue::Map(m) = self {
            Some(m)
        } else {
            None
        }
    }
}
