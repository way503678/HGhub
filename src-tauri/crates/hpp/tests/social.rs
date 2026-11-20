use hghub_hpp::{model, PluginManager};
use serde_json::json;

#[test]
fn comment_pages_filter_invalid_rows_and_validate_interaction_states() {
    let result = model::validate_result("comments.list", json!({"items":[
        {"id":"one","author":{"name":"Viewer","unknown":true},"text":"<script>plain text</script>","liked":false,"likes":0,"replies":[
            {"id":"reply","author":{"name":"Reply"},"text":"Valid"},
            {"id":"bad-reply","author":{"name":"Bad"},"text":"Bad","liked":"yes"}
        ]},
        {"id":"bad","author":{"name":"Bad"},"text":"Bad","likes":-1},
        {"id":"one","author":{"name":"Duplicate"},"text":"Duplicate"}
    ],"next":"opaque-cursor","unknown":true}), false).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["replies"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["text"], "<script>plain text</script>");
    assert_eq!(result["next"], "opaque-cursor");
    assert!(result["items"][0]["author"].get("unknown").is_none());
    let detail = json!({"id":"one","title":"One","playlists":[{"id":"main","episodes":[]}],"interaction":{"liked":false,"favorited":true,"likedEpisodeIds":["ep"],"unknown":true}});
    let result = model::validate_result("media.getDetail", detail.clone(), false).unwrap();
    assert_eq!(result["interaction"]["liked"], false);
    assert!(result["interaction"].get("unknown").is_none());
    for bad in [
        json!(null),
        json!({"liked":"yes"}),
        json!({"likedEpisodeIds":[""]}),
    ] {
        let mut value = detail.clone();
        value["interaction"] = bad;
        assert!(model::validate_result("media.getDetail", value, false).is_err());
    }
    assert!(model::validate_result("comments.send", serde_json::Value::Null, false).is_ok());
    assert!(model::validate_result("interact.like", json!({"ok":true}), false).is_err());
}

#[tokio::test]
async fn social_methods_are_discovered_and_keep_provider_owned_parameters() {
    let dir = tempfile::tempdir().unwrap();
    let host = PluginManager::open(dir.path().into()).unwrap();
    let code =
        include_str!("fixtures/social.js").replace("__FIXTURE_BASE__", "https://media.example");
    let plugin = host.import_code(code, None, false).await.unwrap();
    assert_eq!(plugin.status, "ready");
    for method in [
        "comments.list",
        "comments.send",
        "comments.like",
        "interact.like",
        "interact.favorite",
        "interact.follow",
        "progress.report",
        "library.listFolders",
        "library.listFollowing",
    ] {
        assert!(plugin.methods.iter().any(|m| m == method));
    }
    host.call(
        "fixture.social",
        "auth.submit",
        json!({"methodId":"direct","values":{}}),
        "login".into(),
    )
    .await
    .unwrap();
    let target = json!({"kind":"replies","commentId":"parent:opaque"});
    let reply = host
        .call(
            "fixture.social",
            "comments.send",
            json!({"target":target,"text":"Hello"}),
            "send".into(),
        )
        .await
        .unwrap();
    assert_eq!(reply["text"], "Hello");
    host.call(
        "fixture.social",
        "interact.favorite",
        json!({"target":{"kind":"media","mediaId":"media:opaque"},"on":true}),
        "favorite".into(),
    )
    .await
    .unwrap();
    let events = host
        .call(
            "fixture.social",
            "ui.submit",
            json!({"actionId":"events","values":{}}),
            "read".into(),
        )
        .await
        .unwrap();
    let events: serde_json::Value =
        serde_json::from_str(events["message"].as_str().unwrap()).unwrap();
    assert!(events
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["method"] == "comments.send" && e["args"]["target"] == target));
    assert!(events
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["method"] == "interact.favorite"
            && e["args"]["target"]["mediaId"] == "media:opaque"));
}
