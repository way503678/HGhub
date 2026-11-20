use axum::{extract::State, http::HeaderMap, routing::get, Router};
use hghub_hpp::PluginManager;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn refreshed_endpoints_replace_removed_selection_and_probe_without_credentials() {
    let seen: Arc<Mutex<Vec<HeaderMap>>> = Default::default();
    let router = Router::new()
        .route(
            "/",
            get(
                |State(seen): State<Arc<Mutex<Vec<HeaderMap>>>>, headers: HeaderMap| async move {
                    seen.lock().unwrap().push(headers);
                    "fixture"
                },
            ),
        )
        .with_state(seen.clone());
    let first = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let second = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let first_url = format!("http://{}", first.local_addr().unwrap());
    let second_url = format!("http://{}", second.local_addr().unwrap());
    let app = router.clone();
    let a = tokio::spawn(async move {
        axum::serve(first, app).await.unwrap();
    });
    let b = tokio::spawn(async move {
        axum::serve(second, router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::with_mode(dir.path().into(), Some(false)).unwrap());
    let source = format!(
        r#"
      export const meta={{id:"fixture.endpoints",name:"线路测试",version:"0.0.1",protocol:1,allowInsecureHttp:true,defaultHeaders:{{Authorization:"Bearer do-not-forward"}}}};
      export default hg=>({{
        endpoints:{{async list({{refresh}}){{if(refresh)await hg.storage.set("refreshed",true);return await hg.storage.get("refreshed")?[{{id:"second",url:"{second_url}"}}]:[{{id:"first",url:"{first_url}"}},{{id:"second",url:"{second_url}"}}];}}}},
        media:{{getDetail(){{return {{id:"one",title:hg.endpoint.current.url,playlists:[{{id:"p",episodes:[]}}]}};}},resolvePlayback(){{return {{streams:[{{url:hg.endpoint.url("/clip.mp4"),format:"progressive"}}]}};}}}}
      }});
    "#
    );
    let initial = manager.import_code(source, None, false).await.unwrap();
    let media = manager
        .prepare_media(
            "fixture.endpoints",
            json!({"mediaId":"one","episodeId":"one","reason":"play","codecs":["h264"]}),
            0,
            None,
            "media".into(),
        )
        .await
        .unwrap();
    let probes = manager
        .endpoint_probe("fixture.endpoints", "probe".into())
        .await
        .unwrap();
    assert_eq!(probes.as_array().unwrap().len(), 2);
    assert!(probes.as_array().unwrap().iter().all(|p| p["ok"] == true));
    assert_eq!(manager.list().await[0].endpoint["id"], "first");
    assert_eq!(manager.list().await[0].revision, initial.revision);
    manager
        .endpoint_list("fixture.endpoints", true, "refresh".into())
        .await
        .unwrap();
    assert_eq!(manager.list().await[0].endpoint["id"], "second");
    assert_ne!(manager.list().await[0].revision, initial.revision);
    let detail = manager
        .call(
            "fixture.endpoints",
            "media.getDetail",
            json!({"id":"one"}),
            "detail".into(),
        )
        .await
        .unwrap();
    assert_eq!(detail["title"], second_url);
    assert_eq!(reqwest::get(&media.url).await.unwrap().status(), 404);
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .all(|h| !h.contains_key("authorization") && !h.contains_key("cookie")));
    drop(manager);
    let manager = PluginManager::with_mode(dir.path().into(), Some(false)).unwrap();
    manager.initialize_enabled().await;
    assert_eq!(manager.list().await[0].endpoint["id"], "second");
    a.abort();
    b.abort();
}
