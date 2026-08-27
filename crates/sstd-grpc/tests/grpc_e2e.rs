//! End-to-end gRPC test (#64 Phase 2): spawn the real tonic server, simulate the
//! Godot main-thread drain (resolve commands read from the mpsc channel), and
//! drive it with a real tonic client over the wire. This exercises the full
//! protobuf encode/decode + channel-bridge + reply plumb-back path without Godot.

use sstd_grpc::editor::editor_service_client::EditorServiceClient;
use sstd_grpc::editor::{CaptureScreenshotRequest, SwitchTabRequest};
use sstd_grpc::{spawn_server, EditorCommand, ScreenshotResult};
use tonic::transport::Channel;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Simulate the Godot main-thread drain: pull each command from the mpsc channel
/// and reply with a plausible result (tab index resolution; a fake PNG).
fn drain_loop(rx: tokio::sync::mpsc::Receiver<EditorCommand>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(cmd) = rx.recv().await {
            match cmd {
                EditorCommand::SwitchTab { tab_name, reply } => {
                    let idx = sstd_grpc::resolve_tab_index(&tab_name).unwrap_or(-1);
                    let _ = reply.send(Ok(sstd_grpc::SwitchTabResult {
                        ok: idx >= 0,
                        tab_index: idx,
                        error: String::new(),
                    }));
                }
                EditorCommand::CaptureScreenshot { reply } => {
                    // Any non-empty payload is "a valid capture"; contents don't matter
                    // for the wire round trip. Real PNG bytes come only from Godot.
                    let _ = reply.send(Ok(ScreenshotResult {
                        png_data: vec![0x89, 0x50, 0x4e, 0x47], // PNG magic
                        width: 640,
                        height: 360,
                    }));
                }
            }
        }
    })
}

async fn client(addr: SocketAddr) -> EditorServiceClient<Channel> {
    // Server thread binds asynchronously; retry connect until it is up.
    let mut last_err = None;
    for _attempt in 0..50 {
        match EditorServiceClient::connect(format!("http://{addr}")).await {
            Ok(c) => return c,
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
    panic!("failed to connect to gRPC server: {last_err:?}")
}

#[tokio::test(flavor = "multi_thread")]
async fn switch_tab_round_trip_over_wire() {
    // Reserve a free port so we can connect a real tonic client.
    let tcp = std::net::TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .expect("tcp bind");
    let port = tcp.local_addr().expect("local_addr").port();
    drop(tcp);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let rx = spawn_server(addr).expect("failed to spawn server");
    let _drain = drain_loop(rx);

    let mut cli = client(addr).await;
    let resp = cli
        .switch_tab(SwitchTabRequest {
            tab_name: "placement".to_string(),
        })
        .await
        .expect("switch_tab RPC failed")
        .into_inner();
    assert!(resp.ok);
    assert_eq!(resp.tab_index, 3);

    let resp = cli
        .switch_tab(SwitchTabRequest {
            tab_name: "nope".to_string(),
        })
        .await
        .expect("switch_tab RPC failed")
        .into_inner();
    assert!(!resp.ok);
}

#[tokio::test(flavor = "multi_thread")]
async fn capture_screenshot_round_trip_over_wire() {
    let tcp = std::net::TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .expect("tcp bind");
    let port = tcp.local_addr().expect("local_addr").port();
    drop(tcp);

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let rx = spawn_server(addr).expect("failed to spawn server");
    let _drain = drain_loop(rx);

    let mut cli = client(addr).await;
    let resp = cli
        .capture_screenshot(CaptureScreenshotRequest {})
        .await
        .expect("capture_screenshot RPC failed")
        .into_inner();
    assert_eq!(resp.png_data, vec![0x89, 0x50, 0x4e, 0x47]);
    assert_eq!(resp.width, 640);
    assert_eq!(resp.height, 360);
}
