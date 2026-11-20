use crate::{PluginError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// All values are tagged, so ordinary plugin objects cannot collide with binary markers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "lowercase")]
pub enum Transfer {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Transfer>),
    Object(BTreeMap<String, Transfer>),
    Bytes(Vec<u8>),
}
impl Transfer {
    pub fn from_json(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(v) => Self::Bool(v),
            Value::Number(v) => Self::Number(v.as_f64().unwrap_or(0.0)),
            Value::String(v) => Self::String(v),
            Value::Array(v) => Self::Array(v.into_iter().map(Self::from_json).collect()),
            Value::Object(v) => Self::Object(
                v.into_iter()
                    .map(|(k, v)| (k, Self::from_json(v)))
                    .collect(),
            ),
        }
    }
    pub fn json(&self) -> Value {
        match self {
            Self::Null | Self::Bytes(_) => Value::Null,
            Self::Bool(v) => Value::Bool(*v),
            Self::Number(v) if v.fract() == 0.0 && v.abs() <= 9_007_199_254_740_991.0 => {
                serde_json::json!(*v as i64)
            }
            Self::Number(v) => serde_json::json!(v),
            Self::String(v) => Value::String(v.clone()),
            Self::Array(v) => Value::Array(v.iter().map(Self::json).collect()),
            Self::Object(v) => {
                Value::Object(v.iter().map(|(k, v)| (k.clone(), v.json())).collect())
            }
        }
    }
    pub fn field(&self, key: &str) -> Option<&Self> {
        if let Self::Object(v) = self {
            v.get(key)
        } else {
            None
        }
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        match self {
            Self::Bytes(v) => Ok(v.clone()),
            Self::String(v) => Ok(v.as_bytes().to_vec()),
            _ => Err(PluginError::invalid("需要字符串或 Uint8Array")),
        }
    }
    pub fn validate(&self, max: usize) -> Result<()> {
        fn visit(v: &Transfer, depth: usize, binary: &mut usize) -> Result<()> {
            if depth > 32 {
                return Err(PluginError::invalid("数据嵌套超过 32 层"));
            }
            match v {
                Transfer::Number(v) if !v.is_finite() => {
                    return Err(PluginError::invalid("数字必须有限"))
                }
                Transfer::Bytes(v) => *binary += v.len(),
                Transfer::Array(v) => {
                    for x in v {
                        visit(x, depth + 1, binary)?;
                    }
                }
                Transfer::Object(v) => {
                    for x in v.values() {
                        visit(x, depth + 1, binary)?;
                    }
                }
                _ => {}
            }
            Ok(())
        }
        let mut binary = 0;
        visit(self, 0, &mut binary)?;
        let json_size = serde_json::to_vec(&self.json())
            .map_err(|_| PluginError::invalid("无法传递数据"))?
            .len();
        if json_size + binary > max {
            return Err(PluginError::invalid("可传递数据超过大小上限"));
        }
        Ok(())
    }
}
