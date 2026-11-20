use hghub_hpp::{library::Library, model, PluginManager};
use serde_json::json;

#[test]
fn library_keys_progress_and_installation_lifetimes_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    let library = Library::open(path.clone()).unwrap();
    for plugin in ["a", "b"] {
        library.operation("put", &json!({"pluginId":plugin,"item":{"id":"same","title":plugin},"favorite":true,"tracking":true})).unwrap();
    }
    for (playlist, episode, position) in [
        ("p1", "real-ep", 42),
        ("p2", "real-ep", 12),
        ("p1", "other-ep", 5),
    ] {
        library.operation("progress.set", &json!({"pluginId":"a","mediaId":"same","playlistId":playlist,"episodeId":episode,"positionSec":position})).unwrap();
    }
    library
        .operation(
            "put",
            &json!({"pluginId":"a","item":{"id":"same","title":"refreshed"}}),
        )
        .unwrap();
    assert_eq!(
        library
            .operation("get", &json!({"pluginId":"a","mediaId":"same"}))
            .unwrap()["favorite"],
        true
    );
    assert_eq!(
        library
            .operation("get", &json!({"pluginId":"b","mediaId":"same"}))
            .unwrap()["lastProgress"],
        serde_json::Value::Null
    );
    drop(library);
    let library = Library::open(path).unwrap();
    assert_eq!(
        library
            .operation(
                "progress.get",
                &json!({"pluginId":"a","mediaId":"same","playlistId":"p1","episodeId":"real-ep"})
            )
            .unwrap()["positionSec"],
        42
    );
    library.operation("history.clear", &json!({})).unwrap();
    let entries = library.operation("list", &json!({})).unwrap();
    assert!(entries
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["favorite"] == true && e.get("lastProgress").is_none()));
    assert_eq!(
        library
            .operation(
                "progress.get",
                &json!({"pluginId":"a","mediaId":"same","playlistId":"p1","episodeId":"real-ep"})
            )
            .unwrap(),
        serde_json::Value::Null
    );
}

#[test]
fn direct_feed_progress_has_no_playlist_and_never_collides_with_playlist_progress() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    let library = Library::open(path.clone()).unwrap();
    library
        .operation(
            "put",
            &json!({"pluginId":"feed","item":{"id":"one","title":"One"}}),
        )
        .unwrap();
    let direct = json!({"pluginId":"feed","mediaId":"one","episodeId":"real","positionSec":7});
    library.operation("progress.set", &direct).unwrap();
    let playlist = json!({"pluginId":"feed","mediaId":"one","playlistId":"main","episodeId":"real","positionSec":12});
    library.operation("progress.set", &playlist).unwrap();
    assert_eq!(
        library.operation("progress.get", &direct).unwrap()["positionSec"],
        7
    );
    assert_eq!(
        library.operation("progress.get", &playlist).unwrap()["positionSec"],
        12
    );
    assert!(library
        .operation("progress.get", &direct)
        .unwrap()
        .get("playlistId")
        .is_none());
    assert!(library.operation("progress.set", &json!({"pluginId":"feed","mediaId":"one","playlistId":"","episodeId":"real","positionSec":99})).is_err());
    drop(library);
    let library = Library::open(path).unwrap();
    assert_eq!(
        library.operation("progress.get", &direct).unwrap()["positionSec"],
        7
    );
    assert_eq!(
        library.operation("progress.get", &playlist).unwrap()["positionSec"],
        12
    );
}

#[test]
fn confirmed_sources_merge_associations_without_moving_favorites_or_progress() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    let library = Library::open(path.clone()).unwrap();
    for plugin in ["a", "b", "c", "d"] {
        library.operation("put", &json!({"pluginId":plugin,"item":{"id":"same","title":plugin},"favorite":plugin=="a","tracking":plugin=="c"})).unwrap();
        library.operation("progress.set", &json!({"pluginId":plugin,"mediaId":"same","playlistId":"main","episodeId":"actual","positionSec":if plugin=="a"{23}else{7}})).unwrap();
    }
    let link = |from, to| json!({"from":{"pluginId":from,"item":{"id":"same","title":from}},"to":{"pluginId":to,"item":{"id":"same","title":to}}});
    library.operation("sources.link", &link("a", "b")).unwrap();
    library.operation("sources.link", &link("c", "d")).unwrap();
    library.operation("sources.link", &link("b", "c")).unwrap();
    library.operation("sources.link", &link("a", "b")).unwrap();
    for plugin in ["a", "b", "c", "d"] {
        let sources = library
            .operation("sources.get", &json!({"pluginId":plugin,"mediaId":"same"}))
            .unwrap();
        assert_eq!(sources.as_array().unwrap().len(), 3);
        assert!(sources
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["pluginId"] != plugin));
        let entry = library
            .operation("get", &json!({"pluginId":plugin,"mediaId":"same"}))
            .unwrap();
        assert_eq!(entry["favorite"], plugin == "a");
        assert_eq!(entry["tracking"], plugin == "c");
        assert_eq!(
            entry["lastProgress"]["positionSec"],
            if plugin == "a" { 23 } else { 7 }
        );
    }
    drop(library);
    let library = Library::open(path.clone()).unwrap();
    assert_eq!(
        library
            .operation("sources.get", &json!({"pluginId":"a","mediaId":"same"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let before = std::fs::read(path.clone()).unwrap();
    for invalid in [
        link("a", "a"),
        json!({"from":{"pluginId":"new","item":{"id":"x","title":"valid"}},"to":{"pluginId":"other","item":{"id":"x"}}}),
    ] {
        assert!(library.operation("sources.link", &invalid).is_err());
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(
        library
            .operation("get", &json!({"pluginId":"new","mediaId":"x"}))
            .unwrap(),
        serde_json::Value::Null
    );
    library.operation("sources.link", &json!({"from":{"pluginId":"new","item":{"id":"x","title":"New"}},"to":{"pluginId":"a","item":{"id":"same","title":"A"}}})).unwrap();
    assert_eq!(
        library
            .operation("sources.get", &json!({"pluginId":"new","mediaId":"x"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn rejected_library_writes_preserve_memory_and_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    let lib = Library::open(path.clone()).unwrap();
    lib.operation(
        "put",
        &json!({"pluginId":"a","item":{"id":"one","title":"valid"},"favorite":true}),
    )
    .unwrap();
    let before = std::fs::read(&path).unwrap();
    for p in [
        json!({"pluginId":"a","item":{"id":"one","title":"changed"},"favorite":"bad"}),
        json!({"pluginId":"a","item":{"id":"one","title":{}}}),
    ] {
        assert!(lib.operation("put", &p).is_err());
    }
    assert!(lib.operation("progress.set",&json!({"pluginId":"a","mediaId":"one","playlistId":"main","episodeId":"ep","positionSec":-1})).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        lib.operation("get", &json!({"pluginId":"a","mediaId":"one"}))
            .unwrap()["item"]["title"],
        "valid"
    );
}

#[test]
fn structured_content_discards_bad_rows_and_unknown_fields() {
    let sections=model::validate_result("home.getSections",json!({"items":[{"id":"row","layout":"row","items":[{"id":"a","title":"Valid","year":2026,"unknown":true},{"id":"b","title":"Bad","year":{}}],"unknown":true},{"id":"bad","layout":"markup","items":[]}],"next":"cursor","unknown":true}),false).unwrap();
    assert_eq!(sections["items"].as_array().unwrap().len(), 1);
    assert_eq!(sections["items"][0]["items"].as_array().unwrap().len(), 1);
    assert!(sections.get("unknown").is_none());
    assert!(sections["items"][0].get("unknown").is_none());
    let cats=model::validate_result("catalog.getCategories",json!([{"id":"one","label":"One","children":[{"id":"two","label":"Two","children":[{"id":"three","label":"Three"}]}]},{"id":"good","label":"Good"}]),false).unwrap();
    assert_eq!(cats[0]["children"].as_array().unwrap().len(), 0);
    assert!(model::validate_result("search.getFilters", json!({}), false).is_err());
}

#[tokio::test]
async fn capability_discovery_and_removal_keep_local_library() {
    let dir = tempfile::tempdir().unwrap();
    let host = PluginManager::open(dir.path().into()).unwrap();
    let code = r#"export const meta={id:"fixture.content",name:"Content",version:"1.0.0",protocol:1};export default ()=>({media:{getDetail(){},resolvePlayback(){}},home:{getSections(){return {items:[]}}},catalog:{getCategories(){return []},list(){return {items:[]}}},search:{search(){return {items:[]}},getFilters(){return []}},library:{listMedia(){return {items:[]}}},updates:{check(){return []}}});"#;
    let first = host.import_code(code.into(), None, false).await.unwrap();
    assert_eq!(first.status, "ready");
    for method in [
        "home.getSections",
        "catalog.getCategories",
        "catalog.list",
        "search.getFilters",
        "library.listMedia",
        "updates.check",
    ] {
        assert!(first.methods.iter().any(|m| m == method));
    }
    host.library.operation("put",&json!({"pluginId":"fixture.content","item":{"id":"one","title":"One"},"favorite":true})).unwrap();
    let second = host.import_code(code.into(), None, false).await.unwrap();
    assert_ne!(first.revision, second.revision);
    host.remove("fixture.content").await.unwrap();
    assert_eq!(
        host.library
            .operation("list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let bad = code.replace(
        "getCategories(){return []},list(){return {items:[]}}",
        "getCategories(){return []}",
    );
    assert_eq!(
        host.import_code(bad, None, false).await.unwrap().status,
        "failed"
    );
}
