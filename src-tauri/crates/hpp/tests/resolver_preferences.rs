use hghub_hpp::{ipc, PluginManager};
use serde_json::{json, Value};
use std::sync::Arc;

const SOURCE: &str = r#"
export const meta={id:'fixture.routing',name:'路由测试',version:'1.0.0',protocol:1};
export default ()=>({media:{
  getDetail({id}){return {id,title:id,playlists:[{id:'main',episodes:[]}]};},
  resolvePlayback(p){return {streams:[{kind:'page',
    url:'https://'+(p.mediaId==='other'?'other.example':p.mediaId==='sub'?'sub.page.example':'page.example')+'/private-path?token=secret-token',
    ...(p.mediaId==='pinned'?{resolverId:'fixture.resolver.a'}:{})}]};}
}});
"#;
fn resolver(id: &str, host: &str) -> String {
    format!(
        r#"export const meta={{id:'{id}',name:'{id}',version:'1.0.0',protocol:1,kind:'resolver',matches:['{host}']}};
      export default hg=>({{resolver:{{async resolve(p){{await hg.storage.set('last',p);return {{streams:[{{url:'https://media.example/movie.mp4',format:'progressive'}}]}};}}}}}});"#
    )
}
async fn prepare(
    manager: &Arc<PluginManager>,
    media: &str,
    selection: Value,
) -> hghub_hpp::Result<Value> {
    let mut params = selection;
    params["id"] = json!("fixture.routing");
    params["requestId"] = json!(uuid::Uuid::new_v4().to_string());
    params["args"] =
        json!({"mediaId":media,"episodeId":"real","reason":"refresh","codecs":["h264"]});
    let result = ipc::dispatch(manager, "media.prepare".into(), params).await?;
    manager
        .sessions
        .release(result["sessionId"].as_str().unwrap());
    Ok(result)
}
async fn command(manager: &Arc<PluginManager>, action: &str, params: Value) -> Value {
    ipc::dispatch(manager, action.into(), params).await.unwrap()
}
async fn install(manager: &Arc<PluginManager>) {
    for code in [
        SOURCE.into(),
        resolver("fixture.resolver.a", "page.example"),
        resolver("fixture.resolver.b", "page.example"),
        resolver("fixture.resolver.other", "other.example"),
    ] {
        assert_eq!(
            manager.import_code(code, None, false).await.unwrap().status,
            "ready"
        );
    }
}

#[tokio::test]
async fn remembered_resolver_is_exact_host_scoped_persistent_and_subordinate_to_explicit_choices() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    install(&manager).await;
    let interaction = prepare(&manager, "one", json!({})).await.unwrap_err();
    assert_eq!(interaction.code, "INTERACTION_REQUIRED");
    let choices = interaction.playback_choices.unwrap();
    assert_eq!(choices["resolverRoute"]["host"], "page.example");
    assert_eq!(
        choices["resolverRoute"]["resolvers"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let visible = choices.to_string();
    assert!(
        !visible.contains("private-path")
            && !visible.contains("secret-token")
            && !visible.contains("resolver.other")
    );

    // An unremembered manual selection only applies to that playback route.
    let chosen = prepare(
        &manager,
        "one",
        json!({"resolverId":"fixture.resolver.b","resolverHost":"page.example"}),
    )
    .await
    .unwrap();
    assert_eq!(chosen["providerId"], "fixture.resolver.b");
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        json!({})
    );
    command(
        &manager,
        "resolver.preferences.set",
        json!({"host":"PAGE.Example.","pluginId":"fixture.resolver.b"}),
    )
    .await;
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        json!({"page.example":"fixture.resolver.b"})
    );
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap()["providerId"],
        "fixture.resolver.b"
    );
    assert_eq!(
        prepare(
            &manager,
            "pinned",
            json!({"resolverId":"fixture.resolver.b"})
        )
        .await
        .unwrap()["providerId"],
        "fixture.resolver.a"
    );
    assert_eq!(
        prepare(
            &manager,
            "one",
            json!({"resolverId":"fixture.resolver.a","resolverHost":"page.example"})
        )
        .await
        .unwrap()["providerId"],
        "fixture.resolver.a"
    );
    // A refreshed source can change page hosts. A previous manual choice must not follow it.
    assert_eq!(
        prepare(
            &manager,
            "other",
            json!({"resolverId":"fixture.resolver.b","resolverHost":"page.example"})
        )
        .await
        .unwrap()["providerId"],
        "fixture.resolver.other"
    );
    assert_eq!(
        prepare(&manager, "sub", json!({})).await.unwrap_err().code,
        "INTERACTION_REQUIRED"
    );

    drop(manager);
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap()["providerId"],
        "fixture.resolver.b"
    );
    command(
        &manager,
        "resolver.preferences.set",
        json!({"host":"other.example","pluginId":"fixture.resolver.other"}),
    )
    .await;
    command(
        &manager,
        "resolver.preferences.set",
        json!({"host":"page.example","pluginId":null}),
    )
    .await;
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap_err().code,
        "INTERACTION_REQUIRED"
    );
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        json!({"other.example":"fixture.resolver.other"})
    );
    command(&manager, "resolver.preferences.clear", json!({})).await;
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        json!({})
    );
}

#[tokio::test]
async fn unavailable_or_nonmatching_preferences_do_not_route_and_failed_writes_leave_memory_unchanged(
) {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    install(&manager).await;
    command(
        &manager,
        "resolver.preferences.set",
        json!({"host":"page.example","pluginId":"fixture.resolver.b"}),
    )
    .await;
    manager
        .set_enabled("fixture.resolver.b", false)
        .await
        .unwrap();
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap()["providerId"],
        "fixture.resolver.a"
    );
    assert_eq!(
        prepare(&manager, "one", json!({"resolverId":"fixture.resolver.b"}))
            .await
            .unwrap_err()
            .code,
        "UNSUPPORTED"
    );
    assert_eq!(
        prepare(
            &manager,
            "pinned",
            json!({"resolverId":"fixture.resolver.b"})
        )
        .await
        .unwrap()["providerId"],
        "fixture.resolver.a"
    );
    manager
        .set_enabled("fixture.resolver.b", true)
        .await
        .unwrap();
    command(
        &manager,
        "resolver.preferences.set",
        json!({"host":"page.example","pluginId":"fixture.resolver.other"}),
    )
    .await;
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap_err().code,
        "INTERACTION_REQUIRED"
    );
    manager.remove("fixture.resolver.other").await.unwrap();
    assert_eq!(
        prepare(&manager, "one", json!({})).await.unwrap_err().code,
        "INTERACTION_REQUIRED"
    );
    for id in [json!("fixture.routing"), json!("missing"), json!(4)] {
        assert_eq!(
            ipc::dispatch(
                &manager,
                "resolver.preferences.set".into(),
                json!({"host":"page.example","pluginId":id})
            )
            .await
            .unwrap_err()
            .code,
            "INVALID_ARGUMENT"
        );
    }
    for host in [
        "",
        "https://page.example",
        "page.example:443",
        "*.example",
        "page.example/private",
        "a@example",
        "page.example\n",
        "%65xample.com",
    ] {
        assert_eq!(
            ipc::dispatch(
                &manager,
                "resolver.preferences.set".into(),
                json!({"host":host,"pluginId":"fixture.resolver.a"})
            )
            .await
            .unwrap_err()
            .code,
            "INVALID_ARGUMENT",
            "{host}"
        );
    }
    let before = command(&manager, "resolver.preferences.get", json!({})).await;
    let path = dir.path().join("resolver-preferences.json");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap(); // deterministic atomic replacement failure on all platforms
    assert!(
        ipc::dispatch(&manager, "resolver.preferences.clear".into(), json!({}))
            .await
            .is_err()
    );
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        before
    );
    assert!(ipc::dispatch(
        &manager,
        "resolver.preferences.set".into(),
        json!({"host":"page.example","pluginId":null})
    )
    .await
    .is_err());
    assert_eq!(
        command(&manager, "resolver.preferences.get", json!({})).await,
        before
    );
}
