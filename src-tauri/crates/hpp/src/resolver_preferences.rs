//! Host-owned routing preferences; never exposed through a plugin's hg environment.
use crate::{store::atomic_write, PluginError, Result};
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex};

pub(crate) fn hostname(raw: &str) -> Result<String> {
    if raw.is_empty()
        || raw
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "/\\@?#%*".contains(c))
    {
        return Err(PluginError::invalid(
            "必须提供完整主机名，不能包含地址、端口或通配符",
        ));
    }
    let host = url::Host::parse(raw)
        .map_err(|_| PluginError::invalid("解析器偏好主机名无效"))?
        .to_string();
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host.len() > 253 || host.contains(':') && !host.starts_with('[') {
        return Err(PluginError::invalid("解析器偏好主机名无效"));
    }
    Ok(host)
}

pub(crate) struct ResolverPreferences {
    path: PathBuf,
    data: Mutex<BTreeMap<String, String>>,
}
impl ResolverPreferences {
    pub fn open(path: PathBuf) -> Result<Self> {
        let data: BTreeMap<String, String> = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?)
                .map_err(|_| PluginError::internal("解析器偏好损坏；原文件已保留"))?
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            path,
            data: Mutex::new(data),
        })
    }
    pub fn get(&self) -> BTreeMap<String, String> {
        self.data.lock().unwrap().clone()
    }
    pub fn set(&self, host: &str, plugin_id: Option<&str>) -> Result<()> {
        let host = hostname(host)?;
        let mut data = self.data.lock().unwrap();
        let mut next = data.clone();
        if let Some(id) = plugin_id {
            next.insert(host, id.into());
        } else {
            next.remove(&host);
        }
        atomic_write(&self.path, &serde_json::to_vec(&next).unwrap())?;
        *data = next;
        Ok(())
    }
    pub fn clear(&self) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        atomic_write(&self.path, b"{}")?;
        data.clear();
        Ok(())
    }
}
