//! Development/test adapter: JSON lines over stdio, same dispatch as Tauri IPC.
//! Explicit --relaxed permits synthetic loopback fixtures without changing app preferences.
use hghub_hpp::{ipc, PluginManager};
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf, sync::Arc};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or("expected plugin data directory")?);
    std::fs::create_dir_all(&root)?;
    hghub_hpp::proxy::bind(&root);
    std::fs::create_dir_all(&root)?;
    hghub_hpp::proxy::bind(&root);
    let relaxed = args.any(|a| a == "--relaxed");
    let manager = Arc::new(PluginManager::with_mode(
        root,
        if relaxed { Some(false) } else { None },
    )?);
    manager.initialize_enabled().await;
    println!("{}", json!({"ready":true}));
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        let request: Value = serde_json::from_str(&line)?;
        let manager = manager.clone();
        tokio::spawn(async move {
            let result = ipc::dispatch(
                &manager,
                request["action"].as_str().unwrap_or("").into(),
                request["params"].clone(),
            )
            .await;
            let response = match result {
                Ok(value) => json!({"id":request["id"],"ok":true,"value":value}),
                Err(error) => json!({"id":request["id"],"ok":false,"error":error}),
            };
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{response}");
            let _ = out.flush();
        });
    }
    Ok(())
}
