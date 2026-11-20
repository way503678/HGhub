use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use hghub_hpp::PluginManager;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

#[tokio::test]
async fn cache_retry_stale_and_cookie_opt_out() {
    let counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let router = Router::new()
        .route(
            "/:key",
            get(
                |State(counts): State<Arc<Mutex<HashMap<String, usize>>>>,
                 Path(key): Path<String>| async move {
                    let mut counts = counts.lock().unwrap();
                    let count = counts.entry(key.clone()).or_default();
                    *count += 1;
                    let status = if key == "flaky" && *count == 1 || key == "stale" && *count > 1 {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::OK
                    };
                    (status, Json(json!({"count":count})))
                },
            ),
        )
        .with_state(counts.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let m = PluginManager::with_mode(dir.path().into(), Some(false)).unwrap();
    let code = format!(
        r#"
      export const meta={{id:"fixture.cache",name:"缓存测试",version:"1.0.0",protocol:1,allowInsecureHttp:true}};
      export default hg=>({{
        media:{{getDetail(){{return {{id:"one",title:"one",playlists:[{{id:"main",episodes:[]}}]}};}},resolvePlayback(){{return {{streams:[{{url:"https://fixture.invalid/a.mp4",format:"progressive"}}]}};}}}},
        search:{{async search({{query}}){{const r=await hg.http.fetch({{url:"{base}/"+query,responseType:"json",credentials:query==="cookie"?"include":"omit",cache:{{ttlSec:query==="stale"?0.01:60,staleIfErrorSec:60}},retry:query==="flaky"?1:0}});return {{items:[{{id:"one",title:r.fromCache+":"+r.data.count}}]}};}}}}
      }});
    "#
    );
    assert_eq!(
        m.import_code(code, None, false).await.unwrap().status,
        "ready"
    );
    async fn call(m: &PluginManager, q: &str) -> Value {
        m.call(
            "fixture.cache",
            "search.search",
            json!({"query":q}),
            uuid::Uuid::new_v4().to_string(),
        )
        .await
        .unwrap()["items"][0]["title"]
            .clone()
    }
    assert_eq!(call(&m, "cache").await, "false:1");
    assert_eq!(call(&m, "cache").await, "true:1");
    assert_eq!(call(&m, "cookie").await, "false:1");
    assert_eq!(call(&m, "cookie").await, "false:2");
    assert_eq!(call(&m, "flaky").await, "false:2");
    assert_eq!(call(&m, "stale").await, "false:1");
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(call(&m, "stale").await, "true:1");
    assert_eq!(counts.lock().unwrap()["cache"], 1);
    server.abort();
}
