//! Code generation for API requests — curl, JavaScript, Python.

use serde_json::Value;

/// The ApiRequest subset needed for code generation.
pub struct CodegenRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_type: Option<String>,
}

pub fn generate_curl(req: &CodegenRequest) -> String {
    let mut parts = vec![format!("curl -X {}", req.method)];

    parts.push(format!("'{}'", req.url));

    for (k, v) in &req.headers {
        parts.push(format!("-H '{}: {}'", k, v));
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            parts.push(format!("-d '{}'", body.replace('\'', "'\\''")));
        }
    }

    parts.join(" \\\n  ")
}

pub fn generate_javascript(req: &CodegenRequest) -> String {
    let mut indent = "  ";
    let mut lines = vec![format!("fetch('{}', {{", req.url)];

    lines.push(format!("{}method: '{}',", indent, req.method));

    if !req.headers.is_empty() {
        lines.push(format!("{}headers: {{", indent));
        for (k, v) in &req.headers {
            lines.push(format!("{}  '{}': '{}',", indent, k, v));
        }
        lines.push(format!("{}}},", indent));
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            if req.body_type.as_deref() == Some("json") {
                lines.push(format!("{}body: JSON.stringify({}),", indent, body));
            } else {
                lines.push(format!("{}body: '{}',", indent, body));
            }
        }
    }

    lines.push("})".to_string());
    lines.push(".then(res => res.json())".to_string());
    lines.push(".then(console.log)".to_string());
    lines.push(".catch(console.error);".to_string());

    lines.join("\n")
}

pub fn generate_python(req: &CodegenRequest) -> String {
    let mut lines = vec!["import requests".to_string(), String::new()];

    let method_lower = req.method.to_lowercase();
    let method_call = if method_lower == "get" {
        "requests.get"
    } else {
        &format!("requests.{}", method_lower)
    };

    lines.push(format!("response = {}.\\", method_call));
    lines.push(format!("    '{}',\\", req.url));

    if !req.headers.is_empty() {
        let header_items: Vec<String> = req
            .headers
            .iter()
            .map(|(k, v)| format!("        '{}': '{}'", k, v))
            .collect();
        lines.push("    headers={".to_string());
        lines.extend(header_items);
        lines.push("    },".to_string());
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            if req.body_type.as_deref() == Some("json") {
                lines.push(format!("    json={},", format_json_arg(body)));
            } else {
                lines.push(format!("    data='{}',", body));
            }
        }
    }

    lines.push(String::new());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.text)".to_string());

    lines.join("\n")
}

fn format_json_arg(body: &str) -> String {
    // Try to parse as JSON and re-serialize as a Python-like dict
    if let Ok(val) = serde_json::from_str::<Value>(body) {
        json_to_python(&val)
    } else {
        format!("'{}'", body)
    }
}

fn json_to_python(val: &Value) -> String {
    match val {
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("    '{}': {}", k, json_to_python(v)))
                .collect();
            format!("{{\n{}\n}}", items.join(",\n"))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_python).collect();
            format!("[{}]", items.join(", "))
        }
        Value::String(s) => format!("'{}'", s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "None".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req() -> CodegenRequest {
        CodegenRequest {
            method: "POST".into(),
            url: "https://api.example.com/data".into(),
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("Authorization".into(), "Bearer tok_123".into()),
            ],
            body: Some(r#"{"name":"test"}"#.into()),
            body_type: Some("json".into()),
        }
    }

    #[test]
    fn test_generate_curl() {
        let code = generate_curl(&make_req());
        assert!(code.contains("curl -X POST"));
        assert!(code.contains("api.example.com/data"));
        assert!(code.contains("Authorization: Bearer tok_123"));
        assert!(code.contains("-d"));
    }

    #[test]
    fn test_generate_javascript() {
        let code = generate_javascript(&make_req());
        assert!(code.contains("fetch("));
        assert!(code.contains("method: 'POST'"));
        assert!(code.contains("JSON.stringify"));
    }

    #[test]
    fn test_generate_python() {
        let code = generate_python(&make_req());
        assert!(code.contains("import requests"));
        assert!(code.contains("requests.post"));
        assert!(code.contains("json="));
    }

    #[test]
    fn test_generate_curl_get() {
        let req = CodegenRequest {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: None,
            body_type: None,
        };
        let code = generate_curl(&req);
        assert!(code.contains("curl -X GET"));
        assert!(!code.contains("-d"));
    }

    #[test]
    fn test_empty_headers() {
        let req = CodegenRequest {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: None,
            body_type: None,
        };
        let js = generate_javascript(&req);
        assert!(js.contains("fetch("));
        assert!(!js.contains("headers:"));
    }
}
