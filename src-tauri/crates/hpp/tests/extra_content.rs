use hghub_hpp::{media::PlaybackSelection, model, PluginManager};
use serde_json::json;
use std::sync::Arc;

#[test]
fn live_article_results_validate_and_isolate_bad_rows() {
    let channels = model::validate_result(
        "live.listChannels",
        json!({"items":[
        {"id":"one","name":"One","number":0,"groupId":"group:opaque","unknown":true},
        {"id":"bad","name":"Bad","number":-1}, {"name":"No ID"}
    ],"next":"opaque"}),
        false,
    )
    .unwrap();
    assert_eq!(channels["items"].as_array().unwrap().len(), 1);
    assert!(channels["items"][0].get("unknown").is_none());
    let programmes = model::validate_result(
        "live.getProgrammes",
        json!([
            {"title":"Valid","startAt":0,"endAt":1000},
            {"title":"Backwards","startAt":1000,"endAt":0}, {"title":"Missing"}
        ]),
        false,
    )
    .unwrap();
    assert_eq!(programmes.as_array().unwrap().len(), 1);
    let article = model::validate_result("articles.getDetail", json!({"id":"one","title":"One","contentHtml":"<script>inert data</script>","interaction":{"liked":false,"unknown":true},"unknown":true}), false).unwrap();
    assert_eq!(article["contentHtml"], "<script>inert data</script>");
    assert!(article.get("unknown").is_none());
    assert!(article["interaction"].get("unknown").is_none());
    assert!(model::validate_result(
        "articles.getDetail",
        json!({"id":"one","title":"One","streams":[]}),
        false
    )
    .is_ok());
    assert!(model::validate_result(
        "articles.getDetail",
        json!({"id":"one","title":"One","contentHtml":{}}),
        false
    )
    .is_err());
    let large = model::validate_result(
        "articles.getDetail",
        json!({"id":"one","title":"One","contentHtml":"果".repeat(200_000)}),
        false,
    )
    .unwrap();
    assert!(large["contentHtml"].as_str().unwrap().len() <= 512 * 1024);
}

#[tokio::test]
async fn pure_live_initializes_and_uses_shared_proxy_without_vod_namespace() {
    let dir = tempfile::tempdir().unwrap();
    let host = Arc::new(PluginManager::with_mode(dir.path().into(), Some(false)).unwrap());
    let code = include_str!("fixtures/extra-content.js")
        .replace("__FIXTURE_BASE__", "http://127.0.0.1:1234");
    let p = host.import_code(code.clone(), None, false).await.unwrap();
    assert_eq!(p.status, "ready");
    assert!(!p.methods.iter().any(|m| m.starts_with("media.")));
    for method in [
        "live.getGroups",
        "live.listChannels",
        "live.resolve",
        "live.getProgrammes",
        "articles.list",
        "articles.getDetail",
    ] {
        assert!(p.methods.iter().any(|m| m == method));
    }
    for (method, params) in [
        (
            "live.resolve",
            json!({"channelId":"channel:one","reason":"play","codecs":["h264"]}),
        ),
        (
            "articles.getDetail",
            json!({"articleId":"article:html","reason":"play","codecs":["h264"]}),
        ),
    ] {
        let prepared = host
            .prepare_content_selected(
                "fixture.extra",
                method,
                params,
                PlaybackSelection::default(),
                method.into(),
            )
            .await
            .unwrap();
        assert_eq!(prepared.provider_id, "fixture.extra");
        assert!(prepared.url.starts_with("http://127.0.0.1:"));
        assert!(prepared.choices["source"]["streams"][0]
            .get("url")
            .is_none());
        host.sessions.release(&prepared.session_id);
    }
    let no_resolve = code.replace("async resolve(args)", "async notResolve(args)");
    assert_eq!(
        host.import_code(no_resolve, None, false)
            .await
            .unwrap()
            .status,
        "failed"
    );
    let vod_without_media = code.replace(
        "    live: {",
        "    search: {search(){return {items:[]}}}, live: {",
    );
    assert_eq!(
        host.import_code(vod_without_media, None, false)
            .await
            .unwrap()
            .status,
        "failed"
    );
}
