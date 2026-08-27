//! Live smoke client (#64 Phase 2): connects to a running Godot editor's gRPC
//! server (default 127.0.0.1:50051) and exercises SwitchTab + CaptureScreenshot.
//!
//! Run the editor with a real display first, then:
//!   cargo run -p sstd-grpc --example grpc_smoke
//!
//! Usage: grpc_smoke [port] [tab_name]

use sstd_grpc::editor::editor_service_client::EditorServiceClient;
use sstd_grpc::editor::{CaptureScreenshotRequest, SwitchTabRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).map(|s| s.as_str()).unwrap_or("50051");
    let tab = args.get(2).map(|s| s.as_str()).unwrap_or("simulator");
    let addr = format!("http://127.0.0.1:{port}");

    let mut retries = 0;
    let mut cli = loop {
        match EditorServiceClient::connect(addr.clone()).await {
            Ok(c) => break c,
            Err(_) if retries < 20 => {
                // Server thread binds asynchronously after editor boot; retry briefly.
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(format!("connect failed: {e}").into()),
        }
    };

    let sw = cli
        .switch_tab(SwitchTabRequest {
            tab_name: tab.to_string(),
        })
        .await?
        .into_inner();
    println!("SWITCH_TAB({tab}) -> ok={} tab_index={} error={}", sw.ok, sw.tab_index, sw.error);

    let shot = cli
        .capture_screenshot(CaptureScreenshotRequest {})
        .await?
        .into_inner();
    let empty = shot.png_data.is_empty();
    println!(
        "CAPTURE -> {} bytes, {}x{}, {}", shot.png_data.len(), shot.width, shot.height,
        if empty { "EMPTY" } else { "ok" }
    );

    Ok(())
}
