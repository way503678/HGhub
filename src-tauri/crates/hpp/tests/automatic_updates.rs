use axum::{extract::State, routing::get, Router};
use hghub_hpp::PluginManager;
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

async fn fixture(
    State((hits, code)): State<(Arc<AtomicUsize>, Arc<String>)>,
    headers: axum::http::HeaderMap,
) -> String {
    hits.fetch_add(1, Ordering::SeqCst);
    code.replace(
        "REPLACE",
        &format!("http://{}/update.js", headers["host"].to_str().unwrap()),
    )
}

#[tokio::test]
async fn startup_check_queues_by_policy_defers_during_playback_and_installs_when_idle() {
    let hits = Arc::new(AtomicUsize::new(0));
    let code = Arc::new(r#"export const meta={id:'fixture.automatic',name:'自动更新虚构源',version:'1.1.0',protocol:1,allowInsecureHttp:true,updateUrl:'REPLACE'};
      export default ()=>({media:{getDetail({id}){return {id,title:id,playlists:[{id:'main',episodes:[]}]};},resolvePlayback(){return {streams:[{url:'http://media.example/video.mp4',format:'progressive'}]};}}});"#.to_string());
    let app = Router::new()
        .route("/update.js", get(fixture))
        .with_state((hits.clone(), code.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/update.js", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let root = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::with_mode(root.path().into(), Some(false)).unwrap());
    let original = format!(
        r#"export const meta={{id:'fixture.automatic',name:'自动更新虚构源',version:'1.0.0',protocol:1,allowInsecureHttp:true,updateUrl:'{base}'}};
      export default ()=>({{media:{{getDetail({{id}}){{return {{id,title:id,playlists:[{{id:'main',episodes:[]}}]}};}},resolvePlayback(){{return {{streams:[{{url:'http://media.example/video.mp4',format:'progressive'}}]}};}}}}}});"#
    );
    assert_eq!(
        manager
            .import_code(original, None, false)
            .await
            .unwrap()
            .status,
        "ready"
    );
    manager
        .update_policy_set(hghub_hpp::manager::UpdatePolicy {
            check_on_startup: true,
            auto_install: true,
        })
        .unwrap();
    let active = manager
        .prepare_media(
            "fixture.automatic",
            json!({"mediaId":"one","episodeId":"real","reason":"play","codecs":["h264"]}),
            0,
            None,
            "playback".into(),
        )
        .await
        .unwrap();
    let state = manager.startup_updates("startup").await.unwrap();
    assert_eq!(manager.list().await[0].meta.version, "1.0.0");
    assert_eq!(
        state
            .items
            .iter()
            .find(|v| v.id == "fixture.automatic")
            .unwrap()
            .status,
        "deferred"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    manager.sessions.release(&active.session_id);
    let installed = manager
        .apply_pending_updates(None, "idle", true)
        .await
        .unwrap();
    assert!(installed.changed);
    assert_eq!(manager.list().await[0].meta.version, "1.1.0");
    assert_eq!(
        installed
            .items
            .iter()
            .find(|v| v.id == "fixture.automatic")
            .unwrap()
            .status,
        "installed"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "installation reuses already downloaded update bytes"
    );
    assert_eq!(
        manager
            .check_updates(vec!["fixture.automatic".into()], "manual-a")
            .await
            .unwrap()
            .statuses[0]
            .status,
        "current"
    );
    assert_eq!(
        manager
            .check_updates(vec!["fixture.automatic".into()], "manual-b")
            .await
            .unwrap()
            .statuses[0]
            .status,
        "current"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        3,
        "manual checks bypass the automatic 24-hour interval"
    );
    server.abort();
}

#[tokio::test]
async fn startup_policy_off_skips_network_and_automatic_install_off_keeps_updates_available() {
    let hits = Arc::new(AtomicUsize::new(0));
    let code = Arc::new("export const meta={id:'fixture.off',name:'关闭的虚构更新',version:'1.1.0',protocol:1,allowInsecureHttp:true,updateUrl:'REPLACE'};export default()=>({media:{getDetail(){return {id:'x',title:'x',playlists:[{id:'main',episodes:[]}]};},resolvePlayback(){return {streams:[{url:'http://media.example/a',format:'progressive'}]};}}});".into());
    let app = Router::new()
        .route("/update.js", get(fixture))
        .with_state((hits.clone(), code));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/update.js", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::with_mode(dir.path().into(), Some(false)).unwrap());
    let original = format!("export const meta={{id:'fixture.off',name:'关闭的虚构更新',version:'1.0.0',protocol:1,allowInsecureHttp:true,updateUrl:'{base}'}};export default()=>({{media:{{getDetail(){{return {{id:'x',title:'x',playlists:[{{id:'main',episodes:[]}}]}};}},resolvePlayback(){{return {{streams:[{{url:'http://media.example/a',format:'progressive'}}]}};}}}}}});");
    manager.import_code(original, None, false).await.unwrap();
    let state = manager.startup_updates("disabled-startup").await.unwrap();
    assert!(state.items.is_empty());
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    manager
        .update_policy_set(hghub_hpp::manager::UpdatePolicy {
            check_on_startup: false,
            auto_install: false,
        })
        .unwrap();
    let check = manager
        .check_updates(vec!["fixture.off".into()], "manual")
        .await
        .unwrap();
    assert_eq!(check.statuses[0].status, "available");
    let state = manager
        .apply_pending_updates(None, "no-auto-install", true)
        .await
        .unwrap();
    assert!(!state.changed);
    assert_eq!(manager.list().await[0].meta.version, "1.0.0");
    assert_eq!(state.items[0].status, "available");
    server.abort();
}
