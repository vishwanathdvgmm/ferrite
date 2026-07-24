use crate::fmt::format_source;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn send_response(id: Value, result: Option<Value>, error: Option<Value>) {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
        error,
    };
    if let Ok(json) = serde_json::to_string(&response) {
        let payload = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        let mut stdout = io::stdout();
        let _ = stdout.write_all(payload.as_bytes());
        let _ = stdout.flush();
    }
}

pub fn run_lsp_server() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    eprintln!("Ferrite LSP server started.");

    let mut documents: HashMap<String, String> = HashMap::new();

    loop {
        let mut content_length: Option<usize> = None;

        // Read headers
        loop {
            let mut line = String::new();
            match handle.read_line(&mut line) {
                Ok(0) => return, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        break;
                    }
                    if line.to_lowercase().starts_with("content-length:") {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() == 2 {
                            if let Ok(len) = parts[1].trim().parse::<usize>() {
                                content_length = Some(len);
                            }
                        }
                    }
                }
                Err(_) => return,
            }
        }

        if let Some(len) = content_length {
            let mut body = vec![0; len];
            if handle.read_exact(&mut body).is_ok() {
                if let Ok(req) = serde_json::from_slice::<JsonRpcRequest>(&body) {
                    if let Some(method) = req.method {
                        match method.as_str() {
                            "initialize" => {
                                if let Some(id) = req.id {
                                    send_response(
                                        id,
                                        Some(serde_json::json!({
                                            "capabilities": {
                                                "textDocumentSync": 1, // 1 = Full sync
                                                "documentFormattingProvider": true,
                                                "hoverProvider": true,
                                                "completionProvider": {
                                                    "resolveProvider": false,
                                                    "triggerCharacters": ["."]
                                                }
                                            }
                                        })),
                                        None,
                                    );
                                }
                            }
                            "textDocument/didOpen" => {
                                if let Some(params) = req.params {
                                    if let (Some(uri), Some(text)) = (
                                        params["textDocument"]["uri"].as_str(),
                                        params["textDocument"]["text"].as_str(),
                                    ) {
                                        documents.insert(uri.to_string(), text.to_string());
                                    }
                                }
                            }
                            "textDocument/didChange" => {
                                if let Some(params) = req.params {
                                    if let Some(uri) = params["textDocument"]["uri"].as_str() {
                                        if let Some(changes) = params["contentChanges"].as_array() {
                                            if let Some(first_change) = changes.first() {
                                                if let Some(text) = first_change["text"].as_str() {
                                                    documents
                                                        .insert(uri.to_string(), text.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "textDocument/formatting" => {
                                if let (Some(id), Some(params)) = (req.id, req.params) {
                                    if let Some(uri) = params["textDocument"]["uri"].as_str() {
                                        // Try to get document from in-memory sync state first
                                        let source_opt =
                                            documents.get(uri).cloned().or_else(|| {
                                                // Fallback to disk
                                                let path_str = uri
                                                    .replace("file:///", "")
                                                    .replace("%3A", ":")
                                                    .replace("%3a", ":");
                                                let path = std::path::PathBuf::from(path_str);
                                                std::fs::read_to_string(&path).ok()
                                            });

                                        if let Some(source) = source_opt {
                                            if let Ok(formatted) = format_source(&source) {
                                                send_response(
                                                    id,
                                                    Some(serde_json::json!([
                                                        {
                                                            "range": {
                                                                "start": { "line": 0, "character": 0 },
                                                                "end": { "line": 999999, "character": 0 }
                                                            },
                                                            "newText": formatted
                                                        }
                                                    ])),
                                                    None,
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                    // Fallback: return null if formatting fails
                                    send_response(id, Some(Value::Null), None);
                                }
                            }
                            "shutdown" => {
                                if let Some(id) = req.id {
                                    send_response(id, Some(Value::Null), None);
                                }
                            }
                            "exit" => break,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
