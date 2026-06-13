use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption { pub id: String, pub name: String, pub color: String }

#[derive(Debug, Clone, PartialEq)]
pub enum PropKind {
    Title, RichText, Number, Checkbox, Url, Email, Phone, Date,
    Select(Vec<SelectOption>), MultiSelect(Vec<SelectOption>), Status(Vec<SelectOption>),
    People, Relation, Files, Formula, Rollup, CreatedTime, LastEditedTime, Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotionProp { pub id: String, pub name: String, pub kind: PropKind }

#[derive(Debug, Clone)]
pub struct NotionDatabase { pub id: String, pub title: String, pub props: Vec<NotionProp> }

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Text(String), Number(f64), Checkbox(bool), Date(String),
    Select(Option<SelectOption>), MultiSelect(Vec<SelectOption>),
    People(Vec<String>), Url(String), Raw(String), Empty,
}

#[derive(Debug, Clone)]
pub struct NotionTask {
    pub id: String,
    pub title: String,
    pub url: String,
    pub props: HashMap<String, PropValue>,
}

fn prop_kind(type_str: &str, def: &serde_json::Value) -> PropKind {
    let opts = |key: &str| def.get(key)
        .and_then(|o| o.get("options"))
        .and_then(|o| o.as_array())
        .map(|a| a.iter().filter_map(select_opt).collect())
        .unwrap_or_default();
    match type_str {
        "title" => PropKind::Title,
        "rich_text" => PropKind::RichText,
        "number" => PropKind::Number,
        "checkbox" => PropKind::Checkbox,
        "url" => PropKind::Url,
        "email" => PropKind::Email,
        "phone_number" => PropKind::Phone,
        "date" => PropKind::Date,
        "select" => PropKind::Select(opts("select")),
        "multi_select" => PropKind::MultiSelect(opts("multi_select")),
        "status" => PropKind::Status(opts("status")),
        "people" => PropKind::People,
        "relation" => PropKind::Relation,
        "files" => PropKind::Files,
        "formula" => PropKind::Formula,
        "rollup" => PropKind::Rollup,
        "created_time" => PropKind::CreatedTime,
        "last_edited_time" => PropKind::LastEditedTime,
        other => PropKind::Unknown(other.to_string()),
    }
}

fn parse_props(v: &serde_json::Value) -> Vec<NotionProp> {
    v.get("properties").and_then(|p| p.as_object()).map(|obj| {
        obj.iter().map(|(name, def)| {
            let type_str = def.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
            NotionProp {
                id: def.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                name: name.clone(),
                kind: prop_kind(type_str, def),
            }
        }).collect()
    }).unwrap_or_default()
}

fn db_title(v: &serde_json::Value) -> String {
    let t = plain_text(v.get("title").unwrap_or(&serde_json::Value::Null));
    if t.is_empty() { "Untitled".to_string() } else { t }
}

/// Parse the `results` array of POST /v1/search (databases only).
pub fn parse_databases(v: &serde_json::Value) -> Vec<NotionDatabase> {
    v.get("results").and_then(|r| r.as_array()).map(|arr| {
        arr.iter()
            .filter(|d| d.get("object").and_then(|o| o.as_str()) == Some("database"))
            .map(|d| NotionDatabase {
                id: d.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                title: db_title(d),
                props: parse_props(d),
            }).collect()
    }).unwrap_or_default()
}

/// Parse GET /v1/databases/{id} into a schema.
pub fn parse_database(v: &serde_json::Value) -> Option<NotionDatabase> {
    let id = v.get("id")?.as_str()?.to_string();
    Some(NotionDatabase { id, title: db_title(v), props: parse_props(v) })
}

/// Parse one page object (an element of query `results`) into a task.
/// `props` is the parent DB schema, used to read property kinds.
pub fn parse_task(v: &serde_json::Value, props: &[NotionProp]) -> Option<NotionTask> {
    let id = v.get("id")?.as_str()?.to_string();
    let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
    let pobj = v.get("properties")?.as_object()?;
    let mut out: HashMap<String, PropValue> = HashMap::new();
    let mut title = String::new();
    for prop in props {
        // find the property entry in the page by matching the schema property id
        let entry = pobj.iter().find(|(_, def)| {
            def.get("id").and_then(|i| i.as_str()) == Some(prop.id.as_str())
        }).map(|(_, def)| def);
        let val = match (&prop.kind, entry) {
            (PropKind::Title, Some(def)) => {
                let t = plain_text(def.get("title").unwrap_or(&serde_json::Value::Null));
                title = t.clone();
                PropValue::Text(t)
            }
            (PropKind::RichText, Some(def)) =>
                PropValue::Text(plain_text(def.get("rich_text").unwrap_or(&serde_json::Value::Null))),
            (PropKind::Number, Some(def)) =>
                def.get("number").and_then(|n| n.as_f64()).map(PropValue::Number).unwrap_or(PropValue::Empty),
            (PropKind::Checkbox, Some(def)) =>
                PropValue::Checkbox(def.get("checkbox").and_then(|b| b.as_bool()).unwrap_or(false)),
            (PropKind::Url, Some(def)) =>
                PropValue::Url(def.get("url").and_then(|s| s.as_str()).unwrap_or("").to_string()),
            (PropKind::Date, Some(def)) =>
                def.get("date").and_then(|d| d.get("start")).and_then(|s| s.as_str())
                    .map(|s| PropValue::Date(s.to_string())).unwrap_or(PropValue::Empty),
            (PropKind::Select(_), Some(def)) =>
                PropValue::Select(def.get("select").and_then(select_opt)),
            (PropKind::Status(_), Some(def)) =>
                PropValue::Select(def.get("status").and_then(select_opt)),
            (PropKind::MultiSelect(_), Some(def)) =>
                PropValue::MultiSelect(def.get("multi_select").and_then(|a| a.as_array())
                    .map(|a| a.iter().filter_map(select_opt).collect()).unwrap_or_default()),
            (PropKind::People, Some(def)) =>
                PropValue::People(def.get("people").and_then(|a| a.as_array())
                    .map(|a| a.iter().filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
                    .unwrap_or_default()),
            (_, Some(def)) => PropValue::Raw(def.to_string()),
            (_, None) => PropValue::Empty,
        };
        out.insert(prop.id.clone(), val);
    }
    Some(NotionTask { id, title, url, props: out })
}

fn plain_text(arr: &serde_json::Value) -> String {
    arr.as_array().map(|a| a.iter()
        .filter_map(|t| t.get("plain_text").and_then(|s| s.as_str()))
        .collect::<String>()).unwrap_or_default()
}

fn select_opt(v: &serde_json::Value) -> Option<SelectOption> {
    let o = v.as_object()?;
    Some(SelectOption {
        id: o.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        name: o.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        color: o.get("color").and_then(|x| x.as_str()).unwrap_or("default").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Vec<NotionProp> {
        vec![
            NotionProp { id: "title".into(), name: "Name".into(), kind: PropKind::Title },
            NotionProp { id: "stat".into(), name: "Status".into(),
                kind: PropKind::Status(vec![SelectOption{id:"s1".into(),name:"Todo".into(),color:"gray".into()}]) },
        ]
    }

    #[test]
    fn parses_databases_list() {
        let v = json!({"results":[{
            "object":"database","id":"db1",
            "title":[{"plain_text":"My Board"}],
            "properties":{"Name":{"id":"title","type":"title","title":{}}}
        }]});
        let dbs = parse_databases(&v);
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].id, "db1");
        assert_eq!(dbs[0].title, "My Board");
    }

    #[test]
    fn parses_database_schema_kinds() {
        let v = json!({
            "id":"db1","title":[{"plain_text":"B"}],
            "properties":{
                "Name":{"id":"title","type":"title","title":{}},
                "Status":{"id":"stat","type":"status","status":{"options":[
                    {"id":"s1","name":"Todo","color":"gray"}]}}
            }
        });
        let db = parse_database(&v).unwrap();
        let by_name: HashMap<_,_> = db.props.iter().map(|p|(p.name.clone(),p.kind.clone())).collect();
        assert_eq!(by_name["Name"], PropKind::Title);
        assert!(matches!(by_name["Status"], PropKind::Status(_)));
    }

    #[test]
    fn parses_task_title_and_status() {
        let v = json!({
            "id":"pg1","url":"https://notion.so/pg1",
            "properties":{
                "Name":{"id":"title","type":"title","title":[{"plain_text":"Fix bug"}]},
                "Status":{"id":"stat","type":"status","status":{"id":"s1","name":"Todo","color":"gray"}}
            }
        });
        let t = parse_task(&v, &schema()).unwrap();
        assert_eq!(t.title, "Fix bug");
        assert_eq!(t.id, "pg1");
        match &t.props["stat"] {
            PropValue::Select(Some(o)) => assert_eq!(o.name, "Todo"),
            other => panic!("expected Select(Some), got {:?}", other),
        }
    }

    #[test]
    fn unknown_prop_type_falls_back_to_raw() {
        let v = json!({
            "id":"pg1","url":"",
            "properties":{
                "Name":{"id":"title","type":"title","title":[{"plain_text":"X"}]},
                "Weird":{"id":"w","type":"rollup","rollup":{"number":5}}
            }
        });
        let mut sch = schema();
        sch.push(NotionProp{id:"w".into(),name:"Weird".into(),kind:PropKind::Rollup});
        let t = parse_task(&v, &sch).unwrap();
        assert!(matches!(t.props.get("w"), Some(PropValue::Raw(_)) | Some(PropValue::Empty)));
    }
}
