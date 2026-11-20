use hghub_hpp::{model, PluginManager};
use serde_json::json;
use std::{sync::Arc, time::Duration};
const SOURCE: &str = include_str!("fixtures/synthetic.js");

#[tokio::test]
async fn import_call_validate_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let manager = PluginManager::open(dir.path().into()).unwrap();
    let info = manager
        .import_code(SOURCE.into(), None, false)
        .await
        .unwrap();
    assert_eq!(info.status, "ready", "{:?}", info.error);
    let page = manager
        .call(
            &info.meta.id,
            "search.search",
            json!({"query":"hello"}),
            "search-1".into(),
        )
        .await
        .unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["title"], "hello");
    let failure = manager
        .call(
            &info.meta.id,
            "search.search",
            json!({"query":"retry-hint"}),
            "error-hint".into(),
        )
        .await
        .unwrap_err();
    assert_eq!(failure.retryable, Some(true));
    assert_eq!(failure.retry_after_sec, Some(3));
    assert!(page["items"][0].get("unknown").is_none());
    for (query, title) in [
        ("bytes", "0001ff"),
        ("crypto", "hello"),
        ("html", "测试 标题"),
        ("scope", "owned:scope-call"),
    ] {
        let p = manager
            .call(
                &info.meta.id,
                "search.search",
                json!({"query":query}),
                "scope-call".into(),
            )
            .await
            .unwrap();
        assert_eq!(p["items"][0]["title"], title);
    }
    manager
        .settings_set(&info.meta.id, "label".into(), json!("用户名称"))
        .await
        .unwrap();
    drop(manager);
    let manager = PluginManager::open(dir.path().into()).unwrap();
    manager.initialize_enabled().await;
    let detail = manager
        .call(
            "fixture.source",
            "media.getDetail",
            json!({"id":"one"}),
            "detail-1".into(),
        )
        .await
        .unwrap();
    assert_eq!(detail["title"], "用户名称");
}
#[tokio::test]
async fn concurrent_calls_keep_scope_and_cancel_independently() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    manager
        .import_code(SOURCE.into(), None, false)
        .await
        .unwrap();
    let m = manager.clone();
    let slow = tokio::spawn(async move {
        m.call(
            "fixture.source",
            "search.search",
            json!({"query":"slow"}),
            "slow-call".into(),
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    let fast = manager
        .call(
            "fixture.source",
            "search.search",
            json!({"query":"fast"}),
            "fast-call".into(),
        )
        .await
        .unwrap();
    assert_eq!(fast["items"][0]["title"], "fast");
    manager.cancel("slow-call").await;
    assert_eq!(slow.await.unwrap().unwrap_err().code, "CANCELLED");
    let (a, b) = tokio::join!(
        manager.call(
            "fixture.source",
            "search.search",
            json!({"query":"scope"}),
            "a".into()
        ),
        manager.call(
            "fixture.source",
            "search.search",
            json!({"query":"scope"}),
            "b".into()
        )
    );
    assert_eq!(a.unwrap()["items"][0]["title"], "owned:a");
    assert_eq!(b.unwrap()["items"][0]["title"], "owned:b");
}
#[tokio::test]
async fn loop_is_interrupted_other_plugins_keep_running() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    manager
        .import_code(SOURCE.into(), None, false)
        .await
        .unwrap();
    manager
        .import_code(
            SOURCE.replace("fixture.source", "fixture.other"),
            None,
            false,
        )
        .await
        .unwrap();
    let m = manager.clone();
    let spin = tokio::spawn(async move {
        m.call(
            "fixture.source",
            "search.search",
            json!({"query":"spin"}),
            "spin".into(),
        )
        .await
    });
    let result = tokio::time::timeout(
        Duration::from_millis(800),
        manager.call(
            "fixture.other",
            "search.search",
            json!({"query":"fast"}),
            "other".into(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result["items"][0]["title"], "fast");
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(4), spin)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .code,
        "TIMEOUT"
    );
    let result = manager
        .call(
            "fixture.source",
            "search.search",
            json!({"query":"restored"}),
            "restored".into(),
        )
        .await
        .unwrap();
    assert_eq!(result["items"][0]["title"], "restored");
}
#[tokio::test]
async fn reject_invalid_modules_and_incompatible_requirements() {
    let dir = tempfile::tempdir().unwrap();
    let m = PluginManager::open(dir.path().into()).unwrap();
    assert!(m
        .import_code(format!("{SOURCE}\nexport const extra=1;"), None, false)
        .await
        .is_err());
    assert!(m
        .import_code(
            format!("import x from 'file:///etc/passwd';\n{SOURCE}"),
            None,
            false
        )
        .await
        .is_err());
    let code = SOURCE.replace(
        "protocol: 1,",
        "protocol: 1, requires:[{type:'feature',name:'webview'}],",
    );
    let info = m.import_code(code, None, false).await.unwrap();
    assert_eq!(info.status, "incompatible");
    assert!(model::version("1.2.bad").is_err());
    assert!(model::version("01.2.3").is_err());
}
