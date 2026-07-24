use std::io::{self, BufRead, Read, Write};

pub fn run_lsp_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut handle = stdin.lock();

    eprintln!("Ferrite LSP server started.");

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
                        break; // End of headers
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
                Err(e) => {
                    eprintln!("LSP Read Error: {}", e);
                    return;
                }
            }
        }

        if let Some(len) = content_length {
            let mut body = vec![0; len];
            if handle.read_exact(&mut body).is_ok() {
                let body_str = String::from_utf8_lossy(&body);
                // Very basic JSON-RPC parsing for "initialize"
                if body_str.contains("\"method\":\"initialize\"")
                    || body_str.contains("\"method\": \"initialize\"")
                {
                    // Extract ID
                    let mut id_val = "null".to_string();
                    if let Some(id_idx) = body_str.find("\"id\":") {
                        let rest = &body_str[id_idx + 5..];
                        let end_idx = rest.find(',').unwrap_or(rest.len());
                        let potential_id = rest[..end_idx].trim();
                        // simplistic cleanup
                        id_val = potential_id.replace("}", "").trim().to_string();
                    }

                    // Respond with basic capabilities
                    let response = format!(
                        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"capabilities\":{{\"textDocumentSync\":1,\"hoverProvider\":true,\"completionProvider\":{{\"resolveProvider\":false,\"triggerCharacters\":[\".\"]}}}}}}}}",
                        id_val
                    );

                    let payload = format!("Content-Length: {}\r\n\r\n{}", response.len(), response);
                    if stdout.write_all(payload.as_bytes()).is_err() {
                        break;
                    }
                    stdout.flush().unwrap();
                } else if body_str.contains("\"method\":\"shutdown\"")
                    || body_str.contains("\"method\": \"shutdown\"")
                {
                    // Extract ID
                    let mut id_val = "null".to_string();
                    if let Some(id_idx) = body_str.find("\"id\":") {
                        let rest = &body_str[id_idx + 5..];
                        let end_idx = rest.find(',').unwrap_or(rest.len());
                        id_val = rest[..end_idx].trim().replace("}", "").to_string();
                    }
                    let response =
                        format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}", id_val);
                    let payload = format!("Content-Length: {}\r\n\r\n{}", response.len(), response);
                    let _ = stdout.write_all(payload.as_bytes());
                    let _ = stdout.flush();
                } else if body_str.contains("\"method\":\"exit\"")
                    || body_str.contains("\"method\": \"exit\"")
                {
                    break;
                }
            }
        }
    }
}
