//! Tauri adapter; the development harness shares the exact same dispatcher.
use hghub_hpp::{PluginManager, Result};
use serde_json::Value;
use std::sync::Arc;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;

/// Document-provider access stays in the host adapter, never in a plugin API.
pub struct PickedDocuments(pub tauri::AppHandle);
impl hghub_hpp::manager::DocumentReader for PickedDocuments {
    fn read(&self, location: &str, limit: usize) -> Result<Vec<u8>> {
        use std::io::Read;
        let file: tauri_plugin_fs::FilePath = location
            .parse()
            .map_err(|_| hghub_hpp::PluginError::invalid("文件选择器返回了无效地址"))?;
        let mut bytes = Vec::new();
        let mut options = tauri_plugin_fs::OpenOptions::new();
        options.read(true);
        self.0
            .fs()
            .open(file, options)?
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

#[tauri::command]
pub async fn pick_plugin_files(app: tauri::AppHandle) -> Result<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("HPP 插件或列表", &["js", "mjs", "json"])
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|file| Ok(file.to_string()))
            .collect()
    })
    .await
    .map_err(|_| hghub_hpp::PluginError::internal("无法打开文件选择器"))?
}

pub struct PluginHost {
    dir: std::path::PathBuf,
    ready: tokio::sync::OnceCell<Arc<PluginManager>>,
}
impl PluginHost {
    pub fn new(dir: std::path::PathBuf) -> Self {
        Self {
            dir,
            ready: Default::default(),
        }
    }
}

#[tauri::command]
pub async fn hpp_manage(
    app: tauri::AppHandle,
    host: tauri::State<'_, PluginHost>,
    action: String,
    params: Value,
) -> Result<Value> {
    let manager = host
        .ready
        .get_or_try_init(|| async {
            let manager = PluginManager::open(host.dir.clone())?
                .with_document_reader(Arc::new(PickedDocuments(app.clone())));
            #[cfg(target_os = "windows")]
            let manager = match crate::windows_login::WindowsLogin::detect() {
                Some(provider) => manager.with_web_login(Arc::new(provider)),
                None => manager,
            };
            #[cfg(target_os = "android")]
            let manager = {
                use tauri::Manager;
                let provider = app.state::<crate::android_login::AndroidLogin>();
                if provider.supported().await? {
                    manager.with_web_login(Arc::new(provider.inner().clone()))
                } else {
                    manager
                }
            };
            Ok::<_, hghub_hpp::PluginError>(Arc::new(manager))
        })
        .await?;
    hghub_hpp::ipc::dispatch(manager, action, params).await
}
