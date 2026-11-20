//! Host-owned library, independent of plugin storage and installation lifetime.
use crate::{
    model,
    store::{atomic_write, now},
    PluginError, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Mutex,
};

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Data {
    entries: BTreeMap<String, Value>,
    progress: BTreeMap<String, Value>,
    #[serde(default)]
    associations: BTreeMap<String, BTreeSet<String>>,
}
pub struct Library {
    path: PathBuf,
    data: Mutex<Data>,
}
fn key(parts: &[&str]) -> String {
    serde_json::to_string(parts).unwrap()
}
fn ident<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256)
        .ok_or_else(|| PluginError::invalid(format!("{k} 标识无效")))
}
fn flag(v: &Value, k: &str) -> Result<Option<bool>> {
    v.get(k)
        .map(|v| {
            v.as_bool()
                .ok_or_else(|| PluginError::invalid(format!("{k} 必须为布尔值")))
        })
        .transpose()
}
fn progress_key(p: &Value) -> Result<String> {
    let playlist = if p.get("playlistId").is_some() {
        Some(ident(p, "playlistId")?)
    } else {
        None
    };
    Ok(serde_json::to_string(&json!([
        ident(p, "pluginId")?,
        ident(p, "mediaId")?,
        playlist,
        ident(p, "episodeId")?
    ]))
    .unwrap())
}
impl Library {
    pub fn open(path: PathBuf) -> Result<Self> {
        let data = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?)
                .map_err(|_| PluginError::internal("本地片库损坏，原文件已保留"))?
        } else {
            Data::default()
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }
    pub fn operation(&self, action: &str, p: &Value) -> Result<Value> {
        let mut lock = self
            .data
            .lock()
            .map_err(|_| PluginError::internal("片库锁不可用"))?;
        let mut data = lock.clone();
        let out = match action {
            "list" => json!(data.entries.values().collect::<Vec<_>>()),
            "get" => data
                .entries
                .get(&key(&[ident(p, "pluginId")?, ident(p, "mediaId")?]))
                .cloned()
                .unwrap_or(Value::Null),
            "sources.get" => {
                let k = key(&[ident(p, "pluginId")?, ident(p, "mediaId")?]);
                json!(data
                    .associations
                    .get(&k)
                    .into_iter()
                    .flatten()
                    .filter(|other| **other != k)
                    .filter_map(|other| data.entries.get(other))
                    .collect::<Vec<_>>())
            }
            "sources.link" => {
                // Validate the entire pair before changing either entry or its associations.
                let pair = ["from", "to"].map(|side| -> Result<(String, &str, Value)> {
                    let plugin = ident(&p[side], "pluginId")?;
                    let item = model::media_item(p[side]["item"].clone(), true)
                        .map_err(|_| PluginError::invalid(format!("{side} 片库条目无效")))?;
                    let k = key(&[plugin, ident(&item, "id")?]);
                    Ok((k, plugin, item))
                });
                let [from, to] = pair;
                let from = from?;
                let to = to?;
                if from.0 == to.0 {
                    return Err(PluginError::invalid("不能关联同一个来源条目"));
                }
                let mut group = BTreeSet::from([from.0.clone(), to.0.clone()]);
                for k in [&from.0, &to.0] {
                    if let Some(existing) = data.associations.get(k) {
                        group.extend(existing.iter().cloned());
                    }
                }
                for (k, plugin, item) in [from, to] {
                    let entry = data.entries.entry(k).or_insert_with(|| json!({
                        "pluginId":plugin,"mediaId":item["id"],"favorite":false,"tracking":false,"updatedAt":now()
                    }));
                    entry["item"] = item;
                }
                for k in &group {
                    data.associations.insert(k.clone(), group.clone());
                }
                Value::Null
            }
            "put" => {
                let plugin = ident(p, "pluginId")?;
                let item = model::media_item(p["item"].clone(), true)
                    .map_err(|_| PluginError::invalid("片库条目无效"))?;
                let id = ident(&item, "id")?;
                let k = key(&[plugin, id]);
                let mut entry=data.entries.get(&k).cloned().unwrap_or_else(||json!({"pluginId":plugin,"mediaId":id,"favorite":false,"tracking":false,"updatedAt":now()}));
                entry["item"] = item;
                entry["updatedAt"] = json!(now());
                for f in ["favorite", "tracking"] {
                    if let Some(value) = flag(p, f)? {
                        entry[f] = json!(value);
                    }
                }
                if let Some(v) = p.get("latestLabel") {
                    entry["latestLabel"] = json!(v
                        .as_str()
                        .ok_or_else(|| PluginError::invalid("latestLabel 必须为文本"))?
                        .chars()
                        .take(200)
                        .collect::<String>());
                }
                data.entries.insert(k, entry.clone());
                entry
            }
            "progress.set" => {
                let plugin = ident(p, "pluginId")?;
                let media = ident(p, "mediaId")?;
                let playlist = if p.get("playlistId").is_some() {
                    Some(ident(p, "playlistId")?)
                } else {
                    None
                };
                let episode = ident(p, "episodeId")?;
                for field in ["positionSec", "durationSec"] {
                    if (field == "positionSec" || p.get(field).is_some())
                        && !p[field].as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)
                    {
                        return Err(PluginError::invalid(format!("{field} 必须为非负秒数")));
                    }
                }
                let k = key(&[plugin, media]);
                let mut progress = json!({"pluginId":plugin,"mediaId":media,"episodeId":episode,"positionSec":p["positionSec"],"at":now()});
                if let Some(playlist) = playlist {
                    progress["playlistId"] = json!(playlist);
                }
                if let Some(v) = p.get("durationSec") {
                    progress["durationSec"] = v.clone();
                }
                if let Some(e) = data.entries.get_mut(&k) {
                    e["lastProgress"] = progress.clone();
                    e["updatedAt"] = json!(now());
                } else {
                    return Err(PluginError::invalid("请先保存片库条目"));
                }
                data.progress.insert(progress_key(p)?, progress.clone());
                progress
            }
            "progress.get" => data
                .progress
                .get(&progress_key(p)?)
                .cloned()
                .unwrap_or(Value::Null),
            "history.clear" => {
                data.progress.clear();
                for e in data.entries.values_mut() {
                    e.as_object_mut().unwrap().remove("lastProgress");
                }
                Value::Null
            }
            _ => return Err(PluginError::new("UNSUPPORTED", "未知片库操作")),
        };
        if !matches!(action, "list" | "get" | "progress.get" | "sources.get") {
            atomic_write(&self.path, &serde_json::to_vec(&data).unwrap())?;
            *lock = data;
        }
        Ok(out)
    }
}
