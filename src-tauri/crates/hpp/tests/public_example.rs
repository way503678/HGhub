use hghub_hpp::PluginManager;
use serde_json::json;

#[tokio::test]
async fn public_authoring_example_runs_in_real_host() {
    let dir = tempfile::tempdir().unwrap();
    let manager = PluginManager::open(dir.path().into()).unwrap();
    let code = include_str!("../../../../examples/plugin-hello.js");
    let installed = manager.import_code(code.into(), None, false).await.unwrap();
    assert_eq!(installed.status, "ready");
    let search = manager
        .call(
            "example.hello",
            "search.search",
            json!({"query":"demo"}),
            "public-search".into(),
        )
        .await
        .unwrap();
    assert_eq!(search["items"][0]["id"], "demo");
    let detail = manager
        .call(
            "example.hello",
            "media.getDetail",
            json!({"id":"demo"}),
            "public-detail".into(),
        )
        .await
        .unwrap();
    assert_eq!(detail["playlists"][0]["episodes"][0]["id"], "first");
    let error = manager
        .call(
            "example.hello",
            "media.resolvePlayback",
            json!({"episodeId":"first"}),
            "public-play".into(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "INVALID_ARGUMENT");
    manager
        .settings_set(
            "example.hello",
            "videoUrl".into(),
            json!("https://media.example.com/owned.mp4"),
        )
        .await
        .unwrap();
    let playback = manager
        .call(
            "example.hello",
            "media.resolvePlayback",
            json!({"mediaId":"demo","playlistId":"main","episodeId":"first"}),
            "configured-play".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        playback["streams"][0]["url"],
        "https://media.example.com/owned.mp4"
    );
    assert_eq!(playback["streams"][0]["format"], "progressive");
}
