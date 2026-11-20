use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Response, Uri},
    routing::get,
    Router,
};
use hghub_hpp::PluginManager;
use serde_json::json;
use std::sync::{Arc, Mutex};
const SOURCE: &str = include_str!("fixtures/synthetic.js");

#[tokio::test]
async fn resolver_owns_transforms_headers_cookies_and_subtitles() {
    let requests = Arc::new(Mutex::new(Vec::<(String, HeaderMap)>::new()));
    let router = Router::new()
        .route(
            "/*path",
            get(
                |State(seen): State<Arc<Mutex<Vec<(String, HeaderMap)>>>>,
                 uri: Uri,
                 headers: HeaderMap| async move {
                    seen.lock().unwrap().push((uri.path().into(), headers));
                    let body = if uri.path() == "/playlist.m3u8" {
                        "#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXTINF:1,\nsegment.ts\n#EXT-X-ENDLIST\n"
                    } else {
                        "fixture-resource"
                    };
                    Response::new(Body::from(body))
                },
            ),
        )
        .with_state(requests.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let m = Arc::new(PluginManager::with_mode(dir.path().into(), Some(false)).unwrap());
    let source=SOURCE.replace("allowInsecureHttp: true,","allowInsecureHttp: true,defaultHeaders:{'X-Provider':'source','Cookie':'source=default'},").replace("return {streams:[{url:\"https://media.example/movie.mp4\",format:\"progressive\"}]};",&format!(r#"return {{streams:[{{kind:"page",url:"{base}/page",headers:{{Authorization:"source-secret",Cookie:"source=secret"}}}}],subtitles:[{{url:"{base}/source.vtt",format:"vtt"}}]}};"#));
    let resolver = format!(
        r#"
      export const meta={{id:"fixture.resolver",name:"虚构解析器",kind:"resolver",version:"1.0.0",protocol:1,matches:["127.0.0.1"],allowInsecureHttp:true,defaultHeaders:{{"X-Provider":"resolver"}}}};
      export default hg=>({{
        resolver:{{async resolve(params){{
          if(params.headers.authorization||params.headers.cookie)throw new Error("source credentials leaked");
          await hg.cookies.set({{url:"{base}",cookie:{{name:"resolver",value:"owned"}}}});
          return {{streams:[{{url:"{base}/playlist.m3u8",format:"hls",transforms:{{segment:{{type:"plugin",name:"mark"}}}}}}],subtitles:[{{url:"{base}/resolver.vtt",format:"vtt",default:true}}]}};
        }}}},
        transformResource({{name,data}}){{if(name!=="mark")throw new Error("wrong transform");return {{data:hg.codec.utf8Encode("resolver:"+hg.codec.utf8Decode(data))}};}}
      }});
    "#
    );
    for code in [source, resolver] {
        let p = m.import_code(code, None, false).await.unwrap();
        assert_eq!(p.status, "ready", "{:?}", p.error);
    }
    let prepared = m
        .prepare_media(
            "fixture.source",
            json!({"mediaId":"one","episodeId":"ep1","reason":"play","codecs":["h264"]}),
            0,
            None,
            "prepare".into(),
        )
        .await
        .unwrap();
    assert_eq!(prepared.provider_id, "fixture.resolver");
    let client = reqwest::Client::new();
    let playlist = client
        .get(&prepared.url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let segment = playlist.lines().find(|l| l.starts_with("http")).unwrap();
    assert_eq!(
        client
            .get(segment)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "resolver:fixture-resource"
    );
    for track in prepared.playback["subtitles"].as_array().unwrap() {
        assert_eq!(
            client
                .get(track["url"].as_str().unwrap())
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }
    for (path, headers) in requests.lock().unwrap().iter() {
        if path == "/source.vtt" {
            assert_eq!(headers["x-provider"], "source");
            assert_eq!(headers["cookie"], "source=default");
        } else {
            assert_eq!(headers["x-provider"], "resolver");
            assert_eq!(headers["cookie"], "resolver=owned");
            assert!(!headers.contains_key("authorization"));
        }
    }
    m.set_enabled("fixture.resolver", false).await.unwrap();
    assert_eq!(
        client.get(&prepared.url).send().await.unwrap().status(),
        404
    );
    server.abort();
}

#[tokio::test]
async fn persistent_runtime_fault_requires_explicit_retry() {
    let dir = tempfile::tempdir().unwrap();
    let m = PluginManager::open(dir.path().into()).unwrap();
    m.import_code(SOURCE.into(), None, false).await.unwrap();
    for n in 0..3 {
        assert_eq!(
            m.call(
                "fixture.source",
                "search.search",
                json!({"query":"spin"}),
                format!("spin-{n}")
            )
            .await
            .unwrap_err()
            .code,
            "TIMEOUT"
        );
    }
    assert_eq!(m.list().await[0].status, "failed");
    assert!(m
        .call(
            "fixture.source",
            "search.search",
            json!({"query":"good"}),
            "no-auto-restart".into()
        )
        .await
        .is_err());
    assert_eq!(m.retry("fixture.source").await.unwrap().status, "ready");
    assert_eq!(
        m.call(
            "fixture.source",
            "search.search",
            json!({"query":"good"}),
            "recovered".into()
        )
        .await
        .unwrap()["items"][0]["title"],
        "good"
    );
}
