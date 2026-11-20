//! Validation for host-rendered forms. Diagnostics contain paths, never submitted values.
use crate::{model, PluginError, Result};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub(crate) fn supports(method: &str) -> bool {
    matches!(
        method,
        "auth.getMethods"
            | "auth.submit"
            | "auth.getProfile"
            | "auth.logout"
            | "ui.getActions"
            | "ui.submit"
    )
}
fn bad(path: &str, message: &str) -> PluginError {
    PluginError::internal(format!("{path}: {message}"))
}
fn object(v: Value, path: &str, keys: &[&str]) -> Result<Map<String, Value>> {
    let Value::Object(mut o) = v else {
        return Err(bad(path, "必须为对象"));
    };
    o.retain(|k, _| keys.contains(&k.as_str()));
    Ok(o)
}
fn string(o: &Map<String, Value>, key: &str, path: &str) -> Result<String> {
    o.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| bad(&format!("{path}.{key}"), "必须为文本"))
}
fn identifier(o: &Map<String, Value>, key: &str, path: &str) -> Result<String> {
    let s = string(o, key, path)?;
    if s.is_empty() || s.len() > 256 {
        return Err(bad(&format!("{path}.{key}"), "标识必须为 1–256 字节"));
    }
    Ok(s)
}
fn text(
    o: &mut Map<String, Value>,
    key: &str,
    max: usize,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    if o.contains_key(key) {
        let s = string(o, key, path)?;
        if s.chars().count() > max {
            warnings.push(format!("{path}.{key}: 展示文本已截断"));
        }
        o.insert(key.into(), Value::String(s.chars().take(max).collect()));
    }
    Ok(())
}
fn image(o: &mut Map<String, Value>, key: &str, http: bool, path: &str) -> Result<()> {
    if let Some(v) = o.remove(key) {
        let v =
            model::resource(v, http).map_err(|_| bad(&format!("{path}.{key}"), "资源引用无效"))?;
        // Header/transform validation remains in the media transport, as for covers.
        o.insert(key.into(), v);
    }
    Ok(())
}
fn profile(v: Value, http: bool, path: &str, warnings: &mut Vec<String>) -> Result<Value> {
    let mut o = object(v, path, &["id", "name", "avatar", "description", "stats"])?;
    identifier(&o, "id", path)?;
    if string(&o, "name", path)?.is_empty() {
        return Err(bad(&format!("{path}.name"), "不得为空"));
    }
    text(&mut o, "name", 200, path, warnings)?;
    text(&mut o, "description", 5000, path, warnings)?;
    image(&mut o, "avatar", http, path)?;
    if let Some(v) = o.get("stats") {
        let stats = v
            .as_object()
            .ok_or_else(|| bad(&format!("{path}.stats"), "必须为计数对象"))?;
        if stats.values().any(|v| {
            !v.as_f64()
                .is_some_and(|n| n.is_finite() && n >= 0.0 && n.fract() == 0.0)
        }) {
            return Err(bad(&format!("{path}.stats"), "计数必须为非负整数"));
        }
    }
    Ok(Value::Object(o))
}
fn form(v: Value, http: bool, path: &str, warnings: &mut Vec<String>) -> Result<Value> {
    let mut o = object(v, path, &["title", "description", "fields", "submitLabel"])?;
    for k in ["title", "submitLabel", "description"] {
        text(
            &mut o,
            k,
            if k == "description" { 5000 } else { 200 },
            path,
            warnings,
        )?;
    }
    let fields = o
        .remove("fields")
        .and_then(|v| v.as_array().cloned())
        .ok_or_else(|| bad(&format!("{path}.fields"), "必须为数组"))?;
    let mut keys = HashSet::new();
    let mut out = Vec::new();
    // Fields are one atomic form, not independent content cards: never silently drop a
    // required credential or hidden challenge and then submit a different operation.
    for (i, v) in fields.into_iter().enumerate() {
        let p = format!("{path}.fields[{i}]");
        let mut f = object(
            v,
            &p,
            &[
                "key",
                "type",
                "label",
                "placeholder",
                "required",
                "value",
                "options",
                "image",
            ],
        )?;
        let key = identifier(&f, "key", &p)?;
        if !keys.insert(key) {
            return Err(bad(&format!("{p}.key"), "字段重复"));
        }
        let kind = string(&f, "type", &p)?;
        if !matches!(
            kind.as_str(),
            "text" | "password" | "email" | "number" | "otp" | "select" | "hidden" | "captcha"
        ) {
            return Err(bad(&format!("{p}.type"), "未知字段类型"));
        }
        for k in ["label", "placeholder"] {
            text(&mut f, k, 200, &p, warnings)?;
        }
        if f.contains_key("value") {
            string(&f, "value", &p)?;
        }
        if f.get("required").is_some_and(|v| !v.is_boolean()) {
            return Err(bad(&format!("{p}.required"), "必须为布尔值"));
        }
        image(&mut f, "image", http, &p)?;
        if kind == "select" || f.contains_key("options") {
            let options = f
                .remove("options")
                .and_then(|v| v.as_array().cloned())
                .ok_or_else(|| bad(&format!("{p}.options"), "必须为数组"))?;
            let mut values = HashSet::new();
            let mut clean = Vec::new();
            for (j, v) in options.into_iter().enumerate() {
                let q = format!("{p}.options[{j}]");
                let mut option = object(v, &q, &["value", "label"])?;
                if !values.insert(string(&option, "value", &q)?) {
                    return Err(bad(&format!("{q}.value"), "选项重复"));
                }
                string(&option, "label", &q)?;
                text(&mut option, "label", 200, &q, warnings)?;
                clean.push(Value::Object(option));
            }
            f.insert("options".into(), Value::Array(clean));
        }
        out.push(Value::Object(f));
    }
    o.insert("fields".into(), Value::Array(out));
    Ok(Value::Object(o))
}
fn descriptors(
    v: Value,
    auth: bool,
    http: bool,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<Value> {
    let a = v.as_array().ok_or_else(|| bad(path, "必须为数组"))?;
    let mut out = Vec::new();
    let mut ids = HashSet::new();
    if a.len() > 100 {
        warnings.push(format!("{path}: 超量条目已丢弃"));
    }
    for (i, v) in a.iter().take(100).enumerate() {
        let p = format!("{path}[{i}]");
        let result = (|| {
            let mut d = object(
                v.clone(),
                &p,
                if auth {
                    &["id", "label", "form"]
                } else {
                    &["id", "label", "description", "form"]
                },
            )?;
            let id = identifier(&d, "id", &p)?;
            string(&d, "label", &p)?;
            text(&mut d, "label", 200, &p, warnings)?;
            text(&mut d, "description", 5000, &p, warnings)?;
            if !auth || d.contains_key("form") {
                let f = form(
                    d.remove("form").unwrap_or(Value::Null),
                    http,
                    &format!("{p}.form"),
                    warnings,
                )?;
                d.insert("form".into(), f);
            }
            if !ids.insert(id) {
                return Err(bad(&format!("{p}.id"), "条目标识重复"));
            }
            Ok(Value::Object(d))
        })();
        match result {
            Ok(v) => out.push(v),
            Err(e) => warnings.push(e.message),
        }
    }
    Ok(Value::Array(out))
}
pub(crate) fn validate(
    method: &str,
    v: Value,
    http: bool,
    warnings: &mut Vec<String>,
) -> Result<Value> {
    match method {
        "auth.getMethods" => {
            let mut o = object(v, method, &["methods", "registerUrl"])?;
            let methods = descriptors(
                o.remove("methods").unwrap_or(Value::Null),
                true,
                http,
                "auth.getMethods.methods",
                warnings,
            )?;
            o.insert("methods".into(), methods);
            if o.contains_key("registerUrl") {
                let raw = string(&o, "registerUrl", method)?;
                // Registration is an external browser link; still only HTTP(S).
                model::url(&raw, true, false)
                    .map_err(|_| bad("auth.getMethods.registerUrl", "必须为 HTTP(S) 地址"))?;
            }
            Ok(Value::Object(o))
        }
        "ui.getActions" => descriptors(v, false, http, method, warnings),
        "auth.getProfile" if v.is_null() => Ok(v),
        "auth.getProfile" => profile(v, http, method, warnings),
        "auth.logout" if v.is_null() => Ok(Value::Null),
        "auth.logout" => Err(bad(method, "必须无返回值")),
        "auth.submit" | "ui.submit" => {
            let mut o = object(v, method, &["status", "form", "user", "message"])?;
            match string(&o, "status", method)?.as_str() {
                "next" => {
                    let f = form(
                        o.remove("form").unwrap_or(Value::Null),
                        http,
                        &format!("{method}.form"),
                        warnings,
                    )?;
                    o.retain(|k, _| k == "status");
                    o.insert("form".into(), f);
                }
                "ok" if method == "auth.submit" => {
                    let user = profile(
                        o.remove("user").unwrap_or(Value::Null),
                        http,
                        "auth.submit.user",
                        warnings,
                    )?;
                    o.retain(|k, _| k == "status");
                    o.insert("user".into(), user);
                }
                "ok" => {
                    o.retain(|k, _| k == "status" || k == "message");
                    text(&mut o, "message", 5000, method, warnings)?;
                }
                _ => return Err(bad(&format!("{method}.status"), "必须为 ok 或 next")),
            }
            Ok(Value::Object(o))
        }
        _ => Err(PluginError::new("UNSUPPORTED", "未知表单方法")),
    }
}
pub(crate) fn validate_params(method: &str, params: &Value) -> Result<()> {
    if !supports(method) {
        return Ok(());
    }
    let o = params
        .as_object()
        .ok_or_else(|| PluginError::invalid("表单参数必须为对象"))?;
    if matches!(method, "auth.submit" | "ui.submit") {
        let key = if method == "auth.submit" {
            "methodId"
        } else {
            "actionId"
        };
        identifier(o, key, method).map_err(|_| PluginError::invalid(format!("{key} 无效")))?;
        if !o
            .get("values")
            .and_then(Value::as_object)
            .is_some_and(|v| v.values().all(Value::is_string))
        {
            return Err(PluginError::invalid("values 必须为字符串键值对象"));
        }
    }
    Ok(())
}
