//! Minimal MCP stdio server exposing Vectify's raster tracing pipeline.

mod protocol;
mod tool;

use anyhow::Result;
use protocol::{Request, Response};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Serve newline-delimited JSON-RPC requests until the client closes stdin.
pub fn serve_stdio() -> Result<()> {
    serve(&mut std::io::stdin().lock(), &mut std::io::stdout())
}

fn serve(input: &mut impl BufRead, output: &mut impl Write) -> Result<()> {
    loop {
        let Some(frame) = read_frame(input)? else {
            return Ok(());
        };
        let response = match frame {
            Frame::Line(line) if !line.trim().is_empty() => dispatch(line.trim()),
            Frame::Line(_) => None,
            Frame::TooLong => Some(Response::error(None, -32700, "frame exceeds 8 MiB limit")),
        };
        if let Some(response) = response {
            writeln!(output, "{}", serde_json::to_string(&response)?)?;
            output.flush()?;
        }
    }
}

enum Frame {
    Line(String),
    TooLong,
}

fn read_frame(input: &mut impl BufRead) -> Result<Option<Frame>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            return Ok((!bytes.is_empty() || oversized).then(|| finish_frame(bytes, oversized)));
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if !oversized {
            if bytes.len() + consumed > MAX_FRAME_BYTES {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..consumed]);
            }
        }
        let complete = available[consumed - 1] == b'\n';
        input.consume(consumed);
        if complete {
            return Ok(Some(finish_frame(bytes, oversized)));
        }
    }
}

fn finish_frame(bytes: Vec<u8>, oversized: bool) -> Frame {
    if oversized {
        Frame::TooLong
    } else {
        Frame::Line(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn dispatch(raw: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(raw) {
        Ok(request) => request,
        Err(error) => return Some(Response::error(None, -32700, error.to_string())),
    };
    if request.id.is_none() || request.id == Some(Value::Null) {
        return None;
    }
    if request.jsonrpc != "2.0" {
        return Some(Response::error(
            request.id,
            -32600,
            "invalid jsonrpc version",
        ));
    }
    match request.method.as_str() {
        "initialize" => Some(Response::success(
            request.id,
            json!({
                "protocolVersion": protocol::VERSION,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "vectify-mcp", "version": env!("CARGO_PKG_VERSION")}
            }),
        )),
        "ping" => Some(Response::success(request.id, json!({}))),
        "tools/list" => Some(Response::success(
            request.id,
            json!({"tools": [tool::definition()]}),
        )),
        "tools/call" => Some(call_tool(request.id, request.params)),
        method => Some(Response::error(
            request.id,
            -32601,
            format!("method not found: {method}"),
        )),
    }
}

fn call_tool(id: Option<Value>, params: Option<Value>) -> Response {
    let Some(name) = params
        .as_ref()
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
    else {
        return Response::error(id, -32602, "missing tool name");
    };
    if name != "vectify_trace" {
        return Response::error(id, -32601, format!("tool not found: {name}"));
    }
    let args = params
        .and_then(|value| value.get("arguments").cloned())
        .unwrap_or(Value::Null);
    let result = tool::trace(&args);
    Response::success(id, json!(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn serves_initialize_and_lists_trace_tool() {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n";
        let mut output = Vec::new();
        serve(&mut Cursor::new(input), &mut output).expect("serve frames");
        let lines: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["result"]["serverInfo"]["name"], "vectify-mcp");
        assert_eq!(lines[1]["result"]["tools"][0]["name"], "vectify_trace");
    }

    #[test]
    fn ignores_notifications() {
        assert!(dispatch(r#"{"jsonrpc":"2.0","method":"ping"}"#).is_none());
    }

    #[test]
    fn oversized_frame_is_rejected_and_reader_resynchronizes() {
        let mut bytes = vec![b'x'; MAX_FRAME_BYTES + 1];
        bytes.extend_from_slice(b"\n{}\n");
        let mut input = std::io::Cursor::new(bytes);
        assert!(matches!(
            read_frame(&mut input).unwrap(),
            Some(Frame::TooLong)
        ));
        assert!(matches!(
            read_frame(&mut input).unwrap(),
            Some(Frame::Line(_))
        ));
    }
}
