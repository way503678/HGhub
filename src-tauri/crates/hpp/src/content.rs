//! Content capability validation. A bad independent row is dropped with its path.
use crate::{
    model::{self, id, keep, object, required, text},
    PluginError, Result,
};
use serde_json::{Map, Value};

pub(crate) fn supports(m: &str) -> bool {
    matches!(
        m,
        "home.getSections"
            | "live.getGroups"
            | "live.listChannels"
            | "live.getProgrammes"
            | "articles.list"
            | "articles.getDetail"
            | "catalog.getCategories"
            | "catalog.getFilters"
            | "search.getFilters"
            | "library.listMedia"
            | "library.listFolders"
            | "library.listFollowing"
            | "progress.report"
            | "updates.check"
            | "comments.list"
            | "comments.send"
            | "comments.like"
            | "interact.like"
            | "interact.favorite"
            | "interact.follow"
    )
}
fn count(o: &Map<String, Value>, k: &str) -> Result<()> {
    if let Some(v) = o.get(k) {
        if !v
            .as_f64()
            .is_some_and(|n| n.is_finite() && n >= 0.0 && n.fract() == 0.0)
        {
            return Err(PluginError::internal(format!("{k}: 必须为非负整数")));
        }
    }
    Ok(())
}
fn boolean(o: &Map<String, Value>, k: &str) -> Result<()> {
    if o.get(k).is_some_and(|v| !v.is_boolean()) {
        return Err(PluginError::internal(format!("{k}: 必须为布尔值")));
    }
    Ok(())
}
pub(crate) fn interaction(v: Value) -> Result<Value> {
    let mut o = object(v)?;
    for key in ["liked", "favorited"] {
        boolean(&o, key)?;
    }
    if let Some(v) = o.get("likedEpisodeIds") {
        let ids = v
            .as_array()
            .ok_or_else(|| PluginError::internal("likedEpisodeIds: 必须为数组"))?;
        for value in ids {
            let s = value
                .as_str()
                .ok_or_else(|| PluginError::internal("likedEpisodeIds: 必须为标识数组"))?;
            if s.is_empty() || s.len() > 256 {
                return Err(PluginError::internal(
                    "likedEpisodeIds: 标识必须为 1–256 字节",
                ));
            }
        }
    }
    keep(&mut o, &["liked", "favorited", "likedEpisodeIds"]);
    Ok(Value::Object(o))
}
fn rows(
    v: Value,
    max: usize,
    path: &str,
    warnings: &mut Vec<String>,
    mut parse: impl FnMut(Value, &mut Vec<String>) -> Result<Value>,
) -> Result<Value> {
    let a = v
        .as_array()
        .ok_or_else(|| PluginError::internal(format!("{path}: 必须为数组")))?;
    if a.len() > max {
        warnings.push(format!("{path}: 超量条目已丢弃"));
    }
    let mut out = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for (i, v) in a.iter().take(max).enumerate() {
        match parse(v.clone(), warnings) {
            Ok(v) if v.get("id").is_none() || ids.insert(v["id"].to_string()) => out.push(v),
            Ok(_) => warnings.push(format!("{path}[{i}].id: 重复条目已丢弃")),
            Err(e) => warnings.push(format!("{path}[{i}]: {}", e.message)),
        }
    }
    Ok(Value::Array(out))
}
fn page(
    v: Value,
    path: &str,
    warnings: &mut Vec<String>,
    parse: impl FnMut(Value, &mut Vec<String>) -> Result<Value>,
) -> Result<Value> {
    let mut o = object(v)?;
    let items = rows(
        o.remove("items").unwrap_or(Value::Null),
        100,
        path,
        warnings,
        parse,
    )?;
    keep(&mut o, &["next", "total"]);
    if o.get("next")
        .is_some_and(|v| !v.is_null() && !v.is_string())
    {
        return Err(PluginError::internal(format!(
            "{path}.next: 必须为字符串或 null"
        )));
    }
    count(&o, "total")?;
    o.insert("items".into(), items);
    Ok(Value::Object(o))
}
fn option(v: Value) -> Result<Value> {
    let mut o = object(v)?;
    id(&o, "id")?;
    required(&o, "label")?;
    text(&mut o, "label", 200)?;
    keep(&mut o, &["id", "label"]);
    Ok(Value::Object(o))
}
fn filters(v: Value, w: &mut Vec<String>) -> Result<Value> {
    rows(v, 100, "filters", w, |v, w| {
        let mut o = object(v)?;
        id(&o, "id")?;
        required(&o, "label")?;
        text(&mut o, "label", 200)?;
        boolean(&o, "multiple")?;
        let opts = rows(
            o.remove("options").unwrap_or(Value::Null),
            100,
            "options",
            w,
            |v, _| option(v),
        )?;
        keep(&mut o, &["id", "label", "multiple"]);
        o.insert("options".into(), opts);
        Ok(Value::Object(o))
    })
}
fn category(v: Value, depth: usize, w: &mut Vec<String>) -> Result<Value> {
    let mut o = object(v)?;
    id(&o, "id")?;
    required(&o, "label")?;
    text(&mut o, "label", 200)?;
    if let Some(v) = o.remove("children") {
        if depth >= 2 {
            return Err(PluginError::internal("分类至多两层"));
        }
        o.insert(
            "children".into(),
            rows(v, 100, "children", w, |v, w| category(v, depth + 1, w))?,
        );
    }
    if let Some(v) = o.remove("filters") {
        o.insert("filters".into(), filters(v, w)?);
    }
    if let Some(v) = o.remove("sorts") {
        o.insert("sorts".into(), rows(v, 100, "sorts", w, |v, _| option(v))?);
    }
    keep(&mut o, &["id", "label", "children", "filters", "sorts"]);
    Ok(Value::Object(o))
}
fn author(v: Value, http: bool) -> Result<Value> {
    let mut o = object(v)?;
    id(&o, "id")?;
    required(&o, "name")?;
    text(&mut o, "name", 200)?;
    text(&mut o, "description", 5000)?;
    count(&o, "followers")?;
    if let Some(v) = o.remove("avatar") {
        o.insert("avatar".into(), model::resource(v, http)?);
    }
    keep(
        &mut o,
        &["id", "name", "avatar", "description", "followers"],
    );
    Ok(Value::Object(o))
}
fn comment(v: Value, http: bool, depth: usize, w: &mut Vec<String>) -> Result<Value> {
    if depth > 30 {
        return Err(PluginError::internal("回复嵌套过深"));
    }
    let mut o = object(v)?;
    id(&o, "id")?;
    required(&o, "text")?;
    text(&mut o, "text", 5000)?;
    let mut a = object(o.remove("author").unwrap_or(Value::Null))?;
    required(&a, "name")?;
    text(&mut a, "name", 200)?;
    if a.contains_key("id") {
        id(&a, "id")?;
    }
    if let Some(v) = a.remove("avatar") {
        a.insert("avatar".into(), model::resource(v, http)?);
    }
    keep(&mut a, &["id", "name", "avatar"]);
    o.insert("author".into(), Value::Object(a));
    for k in ["createdAt", "likes", "replyCount"] {
        count(&o, k)?;
    }
    for k in ["liked", "pinned"] {
        boolean(&o, k)?;
    }
    text(&mut o, "region", 200)?;
    if let Some(v) = o.remove("replies") {
        o.insert(
            "replies".into(),
            rows(v, 100, "replies", w, |v, w| comment(v, http, depth + 1, w))?,
        );
    }
    keep(
        &mut o,
        &[
            "id",
            "author",
            "text",
            "createdAt",
            "likes",
            "liked",
            "replyCount",
            "replies",
            "pinned",
            "region",
        ],
    );
    Ok(Value::Object(o))
}
pub(crate) fn validate(m: &str, v: Value, http: bool, w: &mut Vec<String>) -> Result<Value> {
    match m {
        "live.getGroups" => rows(v, 100, m, w, |v, _| option(v)),
        "live.listChannels" => page(v, m, w, |v, _| {
            let mut o = object(v)?;
            id(&o, "id")?;
            required(&o, "name")?;
            text(&mut o, "name", 200)?;
            text(&mut o, "now", 5000)?;
            count(&o, "number")?;
            if o.contains_key("groupId") {
                id(&o, "groupId")?;
            }
            if let Some(v) = o.remove("logo") {
                o.insert("logo".into(), model::resource(v, http)?);
            }
            keep(&mut o, &["id", "name", "logo", "number", "groupId", "now"]);
            Ok(Value::Object(o))
        }),
        "live.getProgrammes" => rows(v, 100, m, w, |v, _| {
            let mut o = object(v)?;
            required(&o, "title")?;
            text(&mut o, "title", 200)?;
            text(&mut o, "description", 5000)?;
            for key in ["startAt", "endAt"] {
                count(&o, key)?;
                if !o.contains_key(key) {
                    return Err(PluginError::internal("节目单缺少时间"));
                }
            }
            if o["endAt"].as_f64() <= o["startAt"].as_f64() {
                return Err(PluginError::internal("节目结束必须晚于开始"));
            }
            keep(&mut o, &["title", "description", "startAt", "endAt"]);
            Ok(Value::Object(o))
        }),
        "articles.list" => page(v, m, w, |v, w| article(v, http, false, w)),
        "articles.getDetail" => article(v, http, true, w),
        "home.getSections" => page(v, m, w, |v, w| {
            let mut o = object(v)?;
            id(&o, "id")?;
            if !matches!(
                o["layout"].as_str(),
                Some("banner" | "row" | "grid" | "list")
            ) {
                return Err(PluginError::internal("layout 无效"));
            }
            if o.get("imageAspect")
                .is_some_and(|v| !matches!(v.as_str(), Some("portrait" | "landscape")))
            {
                return Err(PluginError::internal("imageAspect 无效"));
            }
            boolean(&o, "more")?;
            text(&mut o, "title", 200)?;
            let items = rows(
                o.remove("items").unwrap_or(Value::Null),
                100,
                "items",
                w,
                |v, _| model::media_item(v, http),
            )?;
            keep(&mut o, &["id", "title", "layout", "imageAspect", "more"]);
            o.insert("items".into(), items);
            Ok(Value::Object(o))
        }),
        "catalog.getCategories" => rows(v, 100, m, w, |v, w| category(v, 1, w)),
        "catalog.getFilters" => {
            let mut o = object(v)?;
            keep(&mut o, &["filters", "sorts"]);
            if let Some(v) = o.remove("filters") {
                o.insert("filters".into(), filters(v, w)?);
            }
            if let Some(v) = o.remove("sorts") {
                o.insert("sorts".into(), rows(v, 100, "sorts", w, |v, _| option(v))?);
            }
            Ok(Value::Object(o))
        }
        "search.getFilters" => filters(v, w),
        "library.listMedia" => page(v, m, w, |v, _| model::media_item(v, http)),
        "library.listFollowing" => page(v, m, w, |v, _| author(v, http)),
        "library.listFolders" => rows(v, 100, m, w, |v, _| {
            let mut o = object(v)?;
            id(&o, "id")?;
            required(&o, "label")?;
            text(&mut o, "label", 200)?;
            count(&o, "count")?;
            keep(&mut o, &["id", "label", "count"]);
            Ok(Value::Object(o))
        }),
        "updates.check" => rows(v, 50, m, w, |v, _| {
            let mut o = object(v)?;
            id(&o, "mediaId")?;
            for k in ["episodeCount", "updatedAt"] {
                count(&o, k)?;
            }
            text(&mut o, "latestLabel", 200)?;
            keep(
                &mut o,
                &["mediaId", "episodeCount", "updatedAt", "latestLabel"],
            );
            Ok(Value::Object(o))
        }),
        "comments.list" => page(v, m, w, |v, w| comment(v, http, 0, w)),
        "comments.send" if !v.is_null() => comment(v, http, 0, w),
        "comments.send" | "comments.like" | "interact.like" | "interact.favorite"
        | "interact.follow" | "progress.report"
            if v.is_null() =>
        {
            Ok(Value::Null)
        }
        _ => Err(PluginError::internal(format!("{m}: 必须无返回值"))),
    }
}

fn article(v: Value, http: bool, detail: bool, w: &mut Vec<String>) -> Result<Value> {
    let mut o = object(v)?;
    id(&o, "id")?;
    required(&o, "title")?;
    text(&mut o, "title", 200)?;
    text(&mut o, "summary", 5000)?;
    count(&o, "publishedAt")?;
    if o.contains_key("mediaId") {
        id(&o, "mediaId")?;
    }
    if let Some(v) = o.remove("cover") {
        o.insert("cover".into(), model::resource(v, http)?);
    }
    if let Some(v) = o.remove("author") {
        o.insert("author".into(), author(v, http)?);
    }
    if let Some(v) = o.remove("tags") {
        let tags = rows(v, 20, "tags", w, |v, _| {
            v.as_str()
                .map(|s| Value::String(s.chars().take(30).collect()))
                .ok_or_else(|| PluginError::internal("tag 必须为文本"))
        })?;
        o.insert("tags".into(), tags);
    }
    if let Some(v) = o.remove("stats") {
        let mut s = object(v)?;
        keep(&mut s, &["likes", "comments", "favorites", "hot"]);
        for key in ["likes", "comments", "favorites", "hot"] {
            count(&s, key)?;
        }
        o.insert("stats".into(), Value::Object(s));
    }
    if detail {
        for key in ["contentHtml", "contentText"] {
            if let Some(v) = o.get_mut(key) {
                let s = v
                    .as_str()
                    .ok_or_else(|| PluginError::internal(format!("{key} 必须为文本")))?;
                if s.len() > 512 * 1024 {
                    let mut end = 512 * 1024;
                    while !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    *v = Value::String(s[..end].to_string());
                    w.push(format!("{key}: 正文已截断"));
                }
            }
        }
        if let Some(v) = o.remove("streams") {
            if v.as_array().is_some_and(Vec::is_empty) {
                o.insert("streams".into(), v);
            } else {
                let streams = model::playback(serde_json::json!({"streams":v}), http, false)?;
                o.insert("streams".into(), streams["streams"].clone());
            }
        }
        if let Some(v) = o.remove("interaction") {
            let mut state = object(interaction(v)?)?;
            keep(&mut state, &["liked", "favorited"]);
            o.insert("interaction".into(), Value::Object(state));
        }
        keep(
            &mut o,
            &[
                "id",
                "title",
                "summary",
                "cover",
                "tags",
                "author",
                "stats",
                "publishedAt",
                "mediaId",
                "contentHtml",
                "contentText",
                "streams",
                "interaction",
            ],
        );
    } else {
        keep(
            &mut o,
            &[
                "id",
                "title",
                "summary",
                "cover",
                "tags",
                "author",
                "stats",
                "publishedAt",
                "mediaId",
            ],
        );
    }
    Ok(Value::Object(o))
}
