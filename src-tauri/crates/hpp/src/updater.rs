//! Host-owned update policy, subscriptions and a durable queue of already downloaded updates.
use super::distribution::{Candidate, SubscriptionPreview};
use super::*;
use sha2::{Digest, Sha256};

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePolicy {
    pub check_on_startup: bool,
    pub auto_install: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub location: String,
    pub name: String,
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotice {
    pub id: String,
    pub name: String,
    pub version: String,
    pub installed_version: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PluginError>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Saved {
    policy: UpdatePolicy,
    subscriptions: Vec<Subscription>,
    checked: BTreeMap<String, u64>,
    items: BTreeMap<String, UpdateNotice>,
    checks: Vec<super::distribution::UpdateStatus>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub policy: UpdatePolicy,
    pub subscriptions: Vec<Subscription>,
    pub items: Vec<UpdateNotice>,
    pub checks: Vec<super::distribution::UpdateStatus>,
    pub checking: bool,
    pub changed: bool,
}
#[derive(Serialize, Deserialize)]
struct Queued {
    candidate: Candidate,
    base_hash: String,
    instance: String,
}
pub(super) struct Updater {
    root: PathBuf,
    data: Mutex<Saved>,
    pub(super) work: tokio::sync::Mutex<()>,
    instance: String,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl Updater {
    pub(super) fn open(root: PathBuf) -> Result<Self> {
        let path = root.join("updates.json");
        let data = if path.exists() {
            serde_json::from_slice(&std::fs::read(path)?)
                .map_err(|_| PluginError::internal("插件更新记录损坏；原文件已保留"))?
        } else {
            Saved::default()
        };
        Ok(Self {
            root,
            data: Mutex::new(data),
            work: Default::default(),
            instance: uuid::Uuid::new_v4().to_string(),
        })
    }
    fn change(&self, change: impl FnOnce(&mut Saved)) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        let mut next = data.clone();
        change(&mut next);
        atomic_write(
            &self.root.join("updates.json"),
            &serde_json::to_vec(&next).unwrap(),
        )?;
        *data = next;
        Ok(())
    }
    pub(super) fn state(&self) -> UpdateState {
        let data = self.data.lock().unwrap();
        UpdateState {
            policy: data.policy.clone(),
            subscriptions: data.subscriptions.clone(),
            items: data.items.values().cloned().collect(),
            checks: data.checks.clone(),
            checking: self.work.try_lock().is_err(),
            changed: false,
        }
    }
    pub(super) fn policy(&self) -> UpdatePolicy {
        self.data.lock().unwrap().policy.clone()
    }
    fn queue_path(&self, id: &str) -> Result<PathBuf> {
        Meta::parse(json!({"id":id,"name":id,"version":"0.0.1","protocol":1}))?;
        Ok(self.root.join(".updates").join(format!("{id}.json")))
    }
    // Reserve before I/O, including failed/cancelled attempts. Manual checks never consult this gate.
    pub(super) fn reserve(
        &self,
        id: &str,
        address: &str,
        now: u64,
        automatic: bool,
    ) -> Result<bool> {
        let key = serde_json::to_string(&(id, address)).unwrap();
        let mut data = self.data.lock().unwrap();
        if automatic
            && data
                .checked
                .get(&key)
                .is_some_and(|last| now.saturating_sub(*last) < DAY_MS)
        {
            return Ok(false);
        }
        let mut next = data.clone();
        next.checked.insert(key, now);
        atomic_write(
            &self.root.join("updates.json"),
            &serde_json::to_vec(&next).unwrap(),
        )?;
        *data = next;
        Ok(true)
    }
    pub(super) fn policy_set(&self, policy: UpdatePolicy) -> Result<()> {
        self.change(|s| s.policy = policy)
    }
    pub(super) fn checks_set(&self, checks: Vec<super::distribution::UpdateStatus>) -> Result<()> {
        self.change(|s| s.checks = checks)
    }
    pub(super) fn subscription_remove(&self, id: &str) -> Result<()> {
        self.change(|s| s.subscriptions.retain(|v| v.id != id))
    }
    pub(super) fn subscription_get(&self, id: &str) -> Result<Subscription> {
        self.data
            .lock()
            .unwrap()
            .subscriptions
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or_else(|| PluginError::new("NOT_FOUND", "订阅已移除"))
    }
    pub(super) fn subscription_save(&self, preview: SubscriptionPreview) -> Result<()> {
        let mut url =
            url::Url::parse(&preview.location).map_err(|_| PluginError::invalid("订阅地址无效"))?;
        url.set_fragment(None);
        let location = url.to_string();
        self.change(|s| {
            if let Some(existing) = s.subscriptions.iter_mut().find(|v| v.location == location) {
                existing.name = preview.name;
                existing.description = preview.description;
            } else {
                s.subscriptions.push(Subscription {
                    id: uuid::Uuid::new_v4().to_string(),
                    location,
                    name: preview.name,
                    description: preview.description,
                });
            }
        })
    }
    fn notice(&self, id: &str, status: &str, error: Option<PluginError>) -> Result<()> {
        self.change(|s| {
            if let Some(item) = s.items.get_mut(id) {
                item.status = status.into();
                item.error = error;
            }
        })
    }
}

impl PluginManager {
    pub fn update_state(&self) -> UpdateState {
        self.updater.state()
    }
    pub fn update_policy_set(&self, policy: UpdatePolicy) -> Result<UpdateState> {
        self.updater.policy_set(policy)?;
        Ok(self.update_state())
    }
    pub fn subscription_remove(&self, id: &str) -> Result<UpdateState> {
        self.updater.subscription_remove(id)?;
        Ok(self.update_state())
    }
    pub async fn refresh_update_state(&self) -> Result<UpdateState> {
        let plugins = self.list().await;
        let mut next = self.updater.data.lock().unwrap().items.clone();
        let mut removed = vec![];
        for item in next.values_mut() {
            let plugin = plugins.iter().find(|p| p.meta.id == item.id);
            if plugin.is_none() {
                removed.push(item.id.clone());
            } else if matches!(item.status.as_str(), "available" | "deferred")
                && model::version(&plugin.unwrap().meta.version)? >= model::version(&item.version)?
            {
                item.status = "current".into();
                item.error = None;
            }
        }
        for id in removed {
            next.remove(&id);
            let _ = std::fs::remove_file(self.updater.queue_path(&id)?);
        }
        if next
            .values()
            .map(|v| (&v.id, &v.status))
            .collect::<Vec<_>>()
            != self
                .updater
                .data
                .lock()
                .unwrap()
                .items
                .values()
                .map(|v| (&v.id, &v.status))
                .collect::<Vec<_>>()
        {
            self.updater.change(|s| s.items = next)?;
        }
        Ok(self.update_state())
    }
    pub fn dismiss_update(&self, id: &str) -> Result<UpdateState> {
        self.updater.change(|s| {
            s.items.remove(id);
        })?;
        let _ = std::fs::remove_file(self.updater.queue_path(id)?);
        Ok(self.update_state())
    }
    pub async fn subscription_refresh(&self, id: &str, request_id: &str) -> Result<ImportPreview> {
        let subscription = self.updater.subscription_get(id)?;
        self.preview_import(vec![subscription.location], request_id)
            .await
    }
    pub(super) async fn capture_updates(&self, candidates: &[Candidate]) -> Result<()> {
        if candidates.is_empty() {
            return Ok(());
        }
        let _mutation = self.mutation.lock().await;
        let mut notices = vec![];
        for candidate in candidates {
            let plugin = self.get(&candidate.info.id).await?;
            let plugin = plugin.lock().await;
            if candidate.revision.as_ref() != Some(&plugin.revision) {
                continue;
            }
            let queued = Queued {
                candidate: candidate.clone(),
                instance: self.updater.instance.clone(),
                base_hash: hash(&std::fs::read(
                    self.root.join(&candidate.info.id).join("plugin.js"),
                )?),
            };
            atomic_write(
                &self.updater.queue_path(&candidate.info.id)?,
                &serde_json::to_vec(&queued).unwrap(),
            )?;
            notices.push(UpdateNotice {
                id: candidate.info.id.clone(),
                name: candidate.info.name.clone(),
                version: candidate.info.version.clone(),
                installed_version: plugin.entry.meta.version.clone(),
                status: "available".into(),
                error: None,
            });
        }
        self.updater.change(|s| {
            for notice in notices {
                s.items.insert(notice.id.clone(), notice);
            }
        })
    }
    pub async fn startup_updates(&self, request_id: &str) -> Result<UpdateState> {
        let Ok(_work) = self.updater.work.try_lock() else {
            return Ok(self.update_state());
        };
        if self.updater.policy().check_on_startup {
            let ids = self.list().await.into_iter().map(|p| p.meta.id).collect();
            let (items, _) = self.collect_updates(ids, request_id, true).await?;
            self.capture_updates(&items).await?;
        }
        let changed = self.apply_pending_inner(None, request_id, true).await?;
        drop(_work);
        let mut state = self.refresh_update_state().await?;
        state.changed = changed;
        Ok(state)
    }
    /// A tick only installs already downloaded candidates. It never triggers a new update check.
    pub async fn apply_pending_updates(
        &self,
        ids: Option<Vec<String>>,
        request_id: &str,
        automatic: bool,
    ) -> Result<UpdateState> {
        let Ok(_work) = self.updater.work.try_lock() else {
            return Ok(self.update_state());
        };
        let changed = self.apply_pending_inner(ids, request_id, automatic).await?;
        drop(_work);
        let mut state = self.refresh_update_state().await?;
        state.changed = changed;
        Ok(state)
    }
    async fn apply_pending_inner(
        &self,
        ids: Option<Vec<String>>,
        request_id: &str,
        automatic: bool,
    ) -> Result<bool> {
        if automatic && !self.updater.policy().auto_install {
            return Ok(false);
        }
        let items = self
            .updater
            .state()
            .items
            .into_iter()
            .filter(|v| {
                matches!(v.status.as_str(), "available" | "deferred")
                    && ids.as_ref().is_none_or(|ids| ids.contains(&v.id))
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Ok(false);
        }
        let op = self.distribution.begin(request_id)?;
        // Covers in-flight source/resolver preparation as well as registered playback. Images do not block.
        let gate = automatic.then(|| self.playback_gate.try_write());
        if automatic && (gate.as_ref().is_some_and(|g| g.is_err()) || self.sessions.has_playback())
        {
            for item in items {
                self.updater.notice(&item.id, "deferred", None)?;
            }
            return Ok(false);
        }
        let _gate = gate.map(|g| g.unwrap());
        let mut changed = false;
        for item in items {
            if op.cancel.is_cancelled() {
                return Err(PluginError::cancelled());
            }
            if automatic && !self.updater.policy().auto_install {
                break;
            }
            let result = async {
                let bytes = std::fs::read(self.updater.queue_path(&item.id)?)?;
                let mut queued: Queued = serde_json::from_slice(&bytes)
                    .map_err(|_| PluginError::invalid("暂存更新损坏，请重新检查"))?;
                let plugin = self.get(&item.id).await?;
                let (version, revision) = {
                    let p = plugin.lock().await;
                    (p.entry.meta.version.clone(), p.revision.clone())
                };
                if model::version(&version)? >= model::version(&item.version)? {
                    return Ok(None);
                }
                if queued.candidate.info.id != item.id
                    || queued.candidate.info.version != item.version
                    || queued.base_hash
                        != hash(&std::fs::read(self.root.join(&item.id).join("plugin.js"))?)
                {
                    return Err(PluginError::invalid("插件已变化，请重新检查更新"));
                }
                if queued.instance != self.updater.instance {
                    queued.candidate.revision = Some(revision);
                }
                let candidate = queued.candidate;
                let code = candidate
                    .code
                    .ok_or_else(|| PluginError::invalid("暂存更新缺少文件"))?;
                let meta = Self::inspect_document(&code).await?;
                if meta.id != item.id || meta.version != item.version {
                    return Err(PluginError::invalid("暂存更新身份不一致"));
                }
                let origin = candidate.origin;
                self.install_code(
                    code,
                    Some(origin.manifest.as_ref().unwrap_or(&origin.location).clone()),
                    Some(origin),
                    false,
                    None,
                    candidate.revision.as_deref(),
                    false,
                    Some(&op.cancel),
                )
                .await
                .map(Some)
            }
            .await;
            let (status, error) = match result {
                Ok(Some(plugin)) => {
                    changed = true;
                    if let Some(error) = plugin.error {
                        ("failed", Some(error))
                    } else {
                        ("installed", None)
                    }
                }
                Ok(None) => ("current", None),
                Err(error) if error.code == "CANCELLED" => return Err(error),
                Err(error) => ("failed", Some(error)),
            };
            self.updater.notice(&item.id, status, error)?;
            let _ = std::fs::remove_file(self.updater.queue_path(&item.id)?);
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_subscriptions_and_per_plugin_address_cooldowns_persist() {
        let dir = tempfile::tempdir().unwrap();
        let updater = Updater::open(dir.path().into()).unwrap();
        assert!(updater
            .reserve("fixture.one", "https://list.example/index.json", 1000, true)
            .unwrap());
        assert!(!updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + DAY_MS - 1,
                true
            )
            .unwrap());
        assert!(updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + 2 * DAY_MS,
                true
            )
            .unwrap());
        assert!(updater
            .reserve("fixture.two", "https://list.example/index.json", 1000, true)
            .unwrap());
        updater
            .policy_set(UpdatePolicy {
                check_on_startup: true,
                auto_install: true,
            })
            .unwrap();
        updater
            .subscription_save(SubscriptionPreview {
                location: "https://list.example/index.json#fragment".into(),
                name: "虚构列表".into(),
                description: "test".into(),
            })
            .unwrap();
        drop(updater);
        let updater = Updater::open(dir.path().into()).unwrap();
        assert!(updater.policy().check_on_startup && updater.policy().auto_install);
        let state = updater.state();
        assert_eq!(state.subscriptions.len(), 1);
        assert_eq!(
            state.subscriptions[0].location,
            "https://list.example/index.json"
        );
        assert_eq!(state.subscriptions[0].name, "虚构列表");
        // A wall-clock rollback must not bypass a persisted automatic cooldown.
        assert!(!updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + DAY_MS,
                true
            )
            .unwrap());
        assert!(!updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + 3 * DAY_MS - 1,
                true
            )
            .unwrap());
        assert!(updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + 3 * DAY_MS,
                true
            )
            .unwrap());
        assert!(updater
            .reserve(
                "fixture.one",
                "https://list.example/index.json",
                1000 + 3 * DAY_MS + 1,
                false
            )
            .unwrap());
        updater
            .subscription_remove(&state.subscriptions[0].id)
            .unwrap();
        assert!(updater.state().subscriptions.is_empty());
    }
}
