//! gRPC EditorService server + channel bridge for external editor control (#64).
//!
//! Godot cannot host gRPC (HTTP/1.1-only stack), so the tonic server runs here
//! inside the Rust GDExtension bridge on a background tokio thread and hands each
//! command to the Godot main thread over an mpsc channel. The main thread,
//! fulfilling inside `SstdBridge::poll_grpc_commands`, replies via a oneshot.
//!
//! Thread-safety invariant: this crate only ever touches its own channels and
//! server state. Every Godot API call (tab switch, viewport capture) is executed
//! on the Godot main thread by the bridge, never here.

pub mod editor {
    tonic::include_proto!("sstd.editor");
}

use std::net::SocketAddr;

use tokio::sync::{mpsc, oneshot};

use editor::editor_service_server::{EditorService, EditorServiceServer};
use editor::{CaptureScreenshotRequest, CaptureScreenshotResponse, SwitchTabRequest, SwitchTabResponse};
use tonic::{transport::Server, Request, Response, Status};

/// A command queued by the tonic server for execution on the Godot main thread.
pub enum EditorCommand {
    SwitchTab {
        tab_name: String,
        reply: oneshot::Sender<Result<SwitchTabResult, String>>,
    },
    CaptureScreenshot {
        reply: oneshot::Sender<Result<ScreenshotResult, String>>,
    },
}

/// Result of a tab switch, resolved on the Godot main thread.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchTabResult {
    pub ok: bool,
    pub tab_index: i32,
    pub error: String,
}

/// Result of a viewport screenshot, resolved on the Godot main thread.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotResult {
    pub png_data: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// Canonical tab names, mirrored from `main.gd.TAB_NAMES`. Order is the
/// tab index (0-4).
pub const TAB_NAMES: [&str; 5] = ["tile", "entity", "map", "placement", "simulator"];

/// Resolve a tab name to its index, or `None` if unknown.
pub fn resolve_tab_index(tab_name: &str) -> Option<i32> {
    TAB_NAMES
        .iter()
        .position(|&n| n == tab_name)
        .map(|i| i as i32)
}

/// The tonic `EditorService` implementation. Thin: every handler forwards to the
/// Godot main thread via the command channel and awaits the oneshot reply.
pub struct EditorServer {
    cmd_tx: mpsc::Sender<EditorCommand>,
}

impl EditorServer {
    pub fn new(cmd_tx: mpsc::Sender<EditorCommand>) -> Self {
        Self { cmd_tx }
    }

    /// Convert a `SwitchTabResult` into the protobuf wire message.
    fn switch_tab_response(result: &SwitchTabResult) -> SwitchTabResponse {
        SwitchTabResponse {
            ok: result.ok,
            tab_index: result.tab_index,
            error: result.error.clone(),
        }
    }
}

/// mpsc capacity between the tonic server and the Godot main thread drain.
pub const COMMAND_CHANNEL_CAPACITY: usize = 64;

#[tonic::async_trait]
impl EditorService for EditorServer {
    async fn switch_tab(
        &self,
        request: Request<SwitchTabRequest>,
    ) -> Result<Response<SwitchTabResponse>, Status> {
        let tab_name = request.into_inner().tab_name;
        let (reply, rx) = oneshot::channel();
        let sent = self
            .cmd_tx
            .send(EditorCommand::SwitchTab { tab_name, reply })
            .await;
        match sent {
            Ok(()) => match rx.await {
                Ok(Ok(result)) => Ok(Response::new(Self::switch_tab_response(&result))),
                Ok(Err(e)) => Err(Status::internal(format!("tab switch failed: {e}"))),
                Err(_) => Err(Status::unavailable("command channel closed before reply")),
            },
            Err(_) => Err(Status::unavailable("command channel closed")),
        }
    }

    async fn capture_screenshot(
        &self,
        _request: Request<CaptureScreenshotRequest>,
    ) -> Result<Response<CaptureScreenshotResponse>, Status> {
        let (reply, rx) = oneshot::channel();
        let sent = self
            .cmd_tx
            .send(EditorCommand::CaptureScreenshot { reply })
            .await;
        match sent {
            Ok(()) => match rx.await {
                Ok(Ok(result)) => Ok(Response::new(CaptureScreenshotResponse {
                    png_data: result.png_data,
                    width: result.width,
                    height: result.height,
                })),
                Ok(Err(e)) => Err(Status::internal(format!("screenshot failed: {e}"))),
                Err(_) => Err(Status::unavailable("command channel closed before reply")),
            },
            Err(_) => Err(Status::unavailable("command channel closed")),
        }
    }
}

/// Spawn the tonic gRPC server on a background tokio runtime bound to `addr`.
/// Returns a handle to the command channel sender the bridge drains. Callers
/// must hold the returned `mpsc::Receiver` on the Godot main thread.
pub fn spawn_server(
    addr: SocketAddr,
) -> Result<mpsc::Receiver<EditorCommand>, Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
    let server = EditorServer::new(tx);

    std::thread::Builder::new()
        .name("sstd-grpc-server".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            runtime.block_on(async move {
                if let Err(e) = Server::builder()
                    .add_service(EditorServiceServer::new(server))
                    .serve(addr)
                    .await
                {
                    eprintln!("sstd-grpc server error: {e}");
                }
            });
        })?;

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_tab_index_maps_names() {
        assert_eq!(resolve_tab_index("tile"), Some(0));
        assert_eq!(resolve_tab_index("entity"), Some(1));
        assert_eq!(resolve_tab_index("map"), Some(2));
        assert_eq!(resolve_tab_index("placement"), Some(3));
        assert_eq!(resolve_tab_index("simulator"), Some(4));
        assert_eq!(resolve_tab_index("bogus"), None);
        assert_eq!(resolve_tab_index(""), None);
    }

    #[test]
    fn switch_tab_response_conversion() {
        let result = SwitchTabResult {
            ok: true,
            tab_index: 2,
            error: String::new(),
        };
        let resp = EditorServer::switch_tab_response(&result);
        assert!(resp.ok);
        assert_eq!(resp.tab_index, 2);
        assert!(resp.error.is_empty());
    }

    #[test]
    fn switch_tab_error_conversion() {
        let result = SwitchTabResult {
            ok: false,
            tab_index: -1,
            error: "unknown tab".to_string(),
        };
        let resp = EditorServer::switch_tab_response(&result);
        assert!(!resp.ok);
        assert_eq!(resp.error, "unknown tab");
    }

    #[tokio::test]
    async fn command_channel_round_trip() {
        let (tx, mut rx) = mpsc::channel::<EditorCommand>(4);

        // Simulate the tonic handler sending a command.
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(EditorCommand::SwitchTab {
            tab_name: "map".to_string(),
            reply: reply_tx,
        })
        .await
        .unwrap();

        // Simulate the Godot main thread drain.
        if let Some(cmd) = rx.recv().await {
            match cmd {
                EditorCommand::SwitchTab { tab_name, reply } => {
                    let idx = resolve_tab_index(&tab_name).unwrap_or(-1);
                    let _ = reply.send(Ok(SwitchTabResult {
                        ok: idx >= 0,
                        tab_index: idx,
                        error: String::new(),
                    }));
                }
                EditorCommand::CaptureScreenshot { .. } => panic!("unexpected capture"),
            }
        }

        let result = reply_rx.await.unwrap().unwrap();
        assert!(result.ok);
        assert_eq!(result.tab_index, 2);
    }
}
