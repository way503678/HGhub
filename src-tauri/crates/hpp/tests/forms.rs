use hghub_hpp::{ipc, model, PluginManager};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

const SOURCE: &str = include_str!("fixtures/forms.js");
async fn host() -> (tempfile::TempDir, Arc<PluginManager>) {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    let info = manager
        .import_code(
            SOURCE.replace("__FIXTURE_BASE__", "https://fixture.example"),
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(info.status, "ready", "{:?}", info.error);
    for name in [
        "auth.getMethods",
        "auth.submit",
        "auth.getProfile",
        "auth.logout",
        "ui.getActions",
        "ui.submit",
    ] {
        assert!(info.methods.iter().any(|m| m == name));
    }
    (dir, manager)
}
async fn command(manager: &Arc<PluginManager>, action: &str, p: Value) -> Value {
    ipc::dispatch(manager, action.into(), p).await.unwrap()
}
async fn submit(
    manager: &Arc<PluginManager>,
    kind: &str,
    id: &str,
    values: Value,
) -> hghub_hpp::Result<Value> {
    ipc::dispatch(manager, "form.submit".into(), json!({"id":"fixture.forms","target":{"kind":kind,"id":id},"values":values,"requestId":"form-submit"})).await
}

#[tokio::test]
async fn auth_multistep_ui_logout_and_reopen() {
    let (dir, manager) = host().await;
    let methods = command(
        &manager,
        "form.actions",
        json!({"id":"fixture.forms","target":"auth","requestId":"methods"}),
    )
    .await;
    assert!(methods.get("ignored").is_none());
    assert_eq!(methods["methods"][0]["form"]["fields"][2]["value"], "first");
    assert!(methods["methods"][1].get("form").is_none());
    let error = submit(
        &manager,
        "auth",
        "password",
        json!({"account":"viewer","password":"wrong"}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "AUTH_INVALID");
    let next = submit(
        &manager,
        "auth",
        "password",
        json!({"account":"viewer","password":"synthetic-password"}),
    )
    .await
    .unwrap();
    assert_eq!(next["status"], "next");
    assert_eq!(next["form"]["fields"][0]["value"], "second");
    let ok = submit(&manager, "auth", "password", json!({"account":"viewer","password":"synthetic-password","challenge":"second","email":"viewer@example.com","number":"12.5","region":"local","otp":"123456","captcha":"HGhub"})).await.unwrap();
    assert_eq!(ok["user"]["name"], "虚构观众");
    assert!(ok["user"].get("ignored").is_none());
    assert_eq!(
        submit(&manager, "ui", "alias", json!({"alias":"新名称"}))
            .await
            .unwrap()["status"],
        "next"
    );
    assert_eq!(
        submit(
            &manager,
            "ui",
            "alias",
            json!({"alias":"新名称","confirm":"yes"})
        )
        .await
        .unwrap()["message"],
        "虚构别名已保存：新名称"
    );
    drop(manager);
    let manager = Arc::new(PluginManager::open(dir.path().into()).unwrap());
    manager.initialize_enabled().await;
    assert_eq!(
        manager
            .call(
                "fixture.forms",
                "auth.getProfile",
                json!({}),
                "profile".into()
            )
            .await
            .unwrap()["id"],
        "synthetic-user"
    );
    manager
        .call("fixture.forms", "auth.logout", json!({}), "logout".into())
        .await
        .unwrap();
    assert!(manager
        .call(
            "fixture.forms",
            "auth.getProfile",
            json!({}),
            "profile".into()
        )
        .await
        .unwrap()
        .is_null());
    assert_eq!(
        submit(&manager, "ui", "alias", json!({}))
            .await
            .unwrap_err()
            .code,
        "AUTH_REQUIRED"
    );
    let logs = serde_json::to_string(&manager.logs("fixture.forms").await.unwrap()).unwrap();
    assert!(!logs.contains("synthetic-password"));
}

#[tokio::test]
async fn namespace_completeness_and_input_types_are_enforced() {
    let (_dir, manager) = host().await;
    for (old, replacement) in [
        ("async logout()", "async missingLogout()"),
        ("getActions()", "missingActions()"),
    ] {
        let info = manager
            .import_code(
                SOURCE
                    .replace("fixture.forms", "fixture.broken")
                    .replace(old, replacement),
                None,
                false,
            )
            .await
            .unwrap();
        assert_eq!(info.status, "failed");
        assert!(info.methods.is_empty());
    }
    let e = submit(&manager, "auth", "direct", json!({"numeric":42}))
        .await
        .unwrap_err();
    assert_eq!(e.code, "INVALID_ARGUMENT");
    let e = submit(&manager, "bad", "direct", json!({}))
        .await
        .unwrap_err();
    assert_eq!(e.code, "INVALID_ARGUMENT");
    assert!(manager
        .call(
            "fixture.forms",
            "auth.getProfile",
            json!({}),
            "profile".into()
        )
        .await
        .unwrap()
        .is_null());
}

#[tokio::test]
async fn cancelled_form_does_not_commit_and_auth_states_stay_plugin_local() {
    let (dir, manager) = host().await;
    manager
        .import_code(
            SOURCE.replace("fixture.forms", "fixture.other"),
            None,
            false,
        )
        .await
        .unwrap();
    submit(&manager, "auth", "direct", json!({})).await.unwrap();
    assert!(manager
        .call(
            "fixture.other",
            "auth.getProfile",
            json!({}),
            "other-profile".into()
        )
        .await
        .unwrap()
        .is_null());
    let m = manager.clone();
    let call = tokio::spawn(async move { submit(&m, "ui", "cancel", json!({})).await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    manager.cancel("form-submit").await;
    assert_eq!(call.await.unwrap().unwrap_err().code, "CANCELLED");
    assert_eq!(
        submit(&manager, "ui", "expire", json!({}))
            .await
            .unwrap_err()
            .code,
        "AUTH_EXPIRED"
    );
    assert!(manager
        .call(
            "fixture.forms",
            "auth.getProfile",
            json!({}),
            "profile".into()
        )
        .await
        .unwrap()
        .is_null());
    submit(&manager, "auth", "direct", json!({})).await.unwrap();
    submit(&manager, "ui", "logout-failure", json!({}))
        .await
        .unwrap();
    assert_eq!(
        manager
            .call("fixture.forms", "auth.logout", json!({}), "logout".into())
            .await
            .unwrap_err()
            .code,
        "UPSTREAM_ERROR"
    );
    assert!(manager
        .call(
            "fixture.forms",
            "auth.getProfile",
            json!({}),
            "profile".into()
        )
        .await
        .unwrap()
        .is_null());
    tokio::time::sleep(Duration::from_millis(5200)).await;
    let storage = std::fs::read_to_string(dir.path().join("fixture.forms/storage.json")).unwrap();
    assert!(!storage.contains("cancelled-operation"));
}

#[test]
fn form_results_are_strict_but_independent_descriptors_are_tolerated() {
    let valid = json!({"id":"good","label":"可用","form":{"fields":[{"key":"otp","type":"otp","required":true,"unknown":1}]}});
    let invalid = json!({"id":"bad","label":"坏表单","form":{"fields":[{"key":"a","type":"text"},{"key":"a","type":"hidden","value":"challenge"}]}});
    let result = model::validate_result("ui.getActions", json!([invalid, valid]), false).unwrap();
    assert_eq!(result.as_array().unwrap().len(), 1);
    assert!(result[0]["form"]["fields"][0].get("unknown").is_none());
    for result in [
        json!({"status":"ok"}),
        json!({"status":"wrong"}),
        json!({"status":"next","form":{"fields":[{"key":"x","type":"unknown"}]}}),
    ] {
        assert_eq!(
            model::validate_result("auth.submit", result, false)
                .unwrap_err()
                .code,
            "INTERNAL"
        );
    }
    assert!(model::validate_result(
        "auth.getMethods",
        json!({"methods":[],"registerUrl":"javascript:alert(1)"}),
        false
    )
    .is_err());
    assert!(model::validate_result(
        "auth.getProfile",
        json!({"id":"x","name":"x","stats":{"count":-1}}),
        false
    )
    .is_err());
}

#[tokio::test]
async fn rejected_form_logs_field_path_without_returning_or_logging_values() {
    let (_dir, manager) = host().await;
    let code = SOURCE.replace("return { status: \"ok\", user: user() };", "return { status: \"next\", form: {fields:[{key:\"password\",type:\"future-field\",value:\"secret-value\"}]} };");
    manager.import_code(code, None, false).await.unwrap();
    assert_eq!(
        submit(&manager, "auth", "direct", json!({}))
            .await
            .unwrap_err()
            .code,
        "INTERNAL"
    );
    let logs = serde_json::to_string(&manager.logs("fixture.forms").await.unwrap()).unwrap();
    assert!(logs.contains("auth.submit.form.fields[0].type"));
    assert!(!logs.contains("secret-value"));
}

#[tokio::test]
async fn resolver_auth_failure_identifies_the_resolver_instead_of_the_source() {
    let (_dir, manager) = host().await;
    manager
        .import_code(
            SOURCE
                .replace("__FIXTURE_BASE__", "https://fixture.example")
                .replace("format: \"progressive\"", "kind: \"page\""),
            None,
            false,
        )
        .await
        .unwrap();
    let resolver = r#"
      export const meta = {id:"fixture.resolver",name:"虚构解析器",version:"1.0.0",protocol:1,kind:"resolver",matches:["*"]};
      export default hg => ({resolver:{resolve(){throw new hg.PluginError("AUTH_EXPIRED","解析器登录失效",{details:{pluginId:"fixture.forms"}});}}});
    "#;
    manager
        .import_code(resolver.into(), None, false)
        .await
        .unwrap();
    let error = manager
        .prepare_media(
            "fixture.forms",
            json!({"mediaId":"one","episodeId":"one","reason":"play","codecs":["h264"]}),
            0,
            Some("fixture.resolver".into()),
            "resolver-auth".into(),
        )
        .await
        .err()
        .expect("resolver login expiry must fail playback preparation");
    assert_eq!(error.code, "AUTH_EXPIRED");
    assert_eq!(error.plugin_id.as_deref(), Some("fixture.resolver"));
    assert_eq!(
        serde_json::to_value(error).unwrap()["pluginId"],
        "fixture.resolver"
    );
}
