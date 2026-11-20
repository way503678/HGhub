use crate::{wire::Transfer, PluginError, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex};

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut f = std::fs::File::create(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(Into::into)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    value: Transfer,
    expires: Option<u64>,
}
pub struct Store {
    path: PathBuf,
    data: Mutex<BTreeMap<String, Entry>>,
}
impl Store {
    pub fn open(path: PathBuf) -> Result<Self> {
        let data = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?)
                .map_err(|_| PluginError::internal("插件存储损坏；原文件已保留"))?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }
    pub fn operation(&self, op: &str, p: &Transfer) -> Result<Transfer> {
        let args = p.json();
        let key = args["key"].as_str().unwrap_or("");
        if matches!(op, "get" | "set" | "delete") && (key.is_empty() || key.len() > 256) {
            return Err(PluginError::invalid("存储 key 必须为 1–256 UTF-8 字节"));
        }
        let mut lock = self
            .data
            .lock()
            .map_err(|_| PluginError::internal("存储锁不可用"))?;
        let mut next = lock.clone();
        next.retain(|_, e| e.expires.is_none_or(|t| t > now()));
        let mut modified = false;
        let result = match op {
            "get" => {
                let mut result = BTreeMap::new();
                result.insert("found".into(), Transfer::Bool(next.contains_key(key)));
                if let Some(e) = next.get(key) {
                    result.insert("value".into(), e.value.clone());
                }
                Transfer::Object(result)
            }
            "keys" => Transfer::from_json(serde_json::json!(next
                .keys()
                .filter(|s| s.starts_with(args["prefix"].as_str().unwrap_or("")))
                .collect::<Vec<_>>())),
            "set" => {
                let value = p
                    .field("value")
                    .ok_or_else(|| PluginError::invalid("缺少存储 value"))?
                    .clone();
                value.validate(256 * 1024)?;
                let ttl = args["ttlSec"].as_f64();
                if ttl.is_some_and(|t| !t.is_finite() || t < 0.0) {
                    return Err(PluginError::invalid("ttlSec 无效"));
                }
                next.insert(
                    key.into(),
                    Entry {
                        value,
                        expires: ttl.map(|t| now().saturating_add((t * 1000.0) as u64)),
                    },
                );
                let usage = next
                    .iter()
                    .map(|(k, e)| {
                        k.len()
                            + serde_json::to_vec(&e.value.json())
                                .unwrap_or_default()
                                .len()
                            + binary_size(&e.value)
                    })
                    .sum::<usize>();
                if usage > 5 * 1024 * 1024 {
                    return Err(PluginError::new("STORAGE_FULL", "插件持久存储已满"));
                }
                modified = true;
                Transfer::Null
            }
            "delete" => {
                next.remove(key);
                modified = true;
                Transfer::Null
            }
            "clear" => {
                next.clear();
                modified = true;
                Transfer::Null
            }
            _ => return Err(PluginError::invalid("未知存储操作")),
        };
        if modified {
            atomic_write(
                &self.path,
                &serde_json::to_vec(&next).map_err(|_| PluginError::internal("无法序列化存储"))?,
            )?;
            *lock = next;
        }
        Ok(result)
    }
}
fn binary_size(v: &Transfer) -> usize {
    match v {
        Transfer::Bytes(v) => v.len(),
        Transfer::Array(v) => v.iter().map(binary_size).sum(),
        Transfer::Object(v) => v.values().map(binary_size).sum(),
        _ => 0,
    }
}
