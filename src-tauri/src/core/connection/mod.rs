mod protocol;

use crate::types::chat::Frame;
use protocol::{read_frame, write_frame};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};

pub const LISTEN_PORT: u16 = 42070;

pub struct PeerConnection {
    pub peer_id: String,
    pub addr: String,
    pub writer: Mutex<OwnedWriteHalf>,
}

#[derive(Debug, Clone)]
pub struct IncomingFrame {
    pub peer_id: String,
    pub frame: Frame,
}

pub struct ConnectionManager {
    connections: Arc<Mutex<HashMap<String, PeerConnection>>>,
    frame_tx: broadcast::Sender<IncomingFrame>,
    running: Arc<AtomicBool>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (frame_tx, _) = broadcast::channel(256);
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            frame_tx,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IncomingFrame> {
        self.frame_tx.subscribe()
    }

    pub async fn start_listener(self: &Arc<Self>) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", LISTEN_PORT);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind TCP listener: {}", e))?;
        println!("[conn] TCP listener started on {}", addr);

        let this = self.clone();
        tokio::spawn(async move {
            loop {
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let peer_addr_str = peer_addr.to_string();
                        println!("[conn] Incoming TCP from {}", peer_addr_str);
                        let this_clone = this.clone();
                        tokio::spawn(async move {
                            this_clone
                                .accept_connection(stream, peer_addr_str)
                                .await;
                        });
                    }
                    Err(e) => eprintln!("[conn] Accept error: {}", e),
                }
            }
        });

        Ok(())
    }

    async fn accept_connection(
        self: &Arc<Self>,
        mut stream: TcpStream,
        addr: String,
    ) {
        // Read hello frame first
        let peer_id = match read_frame(&mut stream).await {
            Ok(Some(Frame::Hello { id })) => id,
            Ok(Some(_)) => {
                eprintln!("[conn] Expected hello from {}, got other frame", addr);
                return;
            }
            Ok(None) => {
                println!("[conn] Connection closed during hello from {}", addr);
                return;
            }
            Err(e) => {
                eprintln!("[conn] Error reading hello from {}: {}", addr, e);
                return;
            }
        };

        println!("[conn] Peer {} identified as {}", addr, peer_id);

        // Send our hello
        let my_id = get_my_id().await;
        if let Err(e) = write_frame(&mut stream, &Frame::Hello { id: my_id }).await {
            eprintln!("[conn] Failed to send hello to {}: {}", peer_id, e);
            return;
        }

        // Split stream for concurrent read/write
        let (reader, writer) = stream.into_split();
        let conn = PeerConnection {
            peer_id: peer_id.clone(),
            addr: addr.clone(),
            writer: Mutex::new(writer),
        };

        {
            let mut conns = self.connections.lock().await;
            conns.insert(peer_id.clone(), conn);
        }

        // Spawn read loop
        self.spawn_read_loop(peer_id.clone(), reader);
    }

    pub async fn connect_to_peer(
        self: &Arc<Self>,
        peer_id: &str,
        peer_addr: &str,
    ) -> Result<(), String> {
        let addr = format!("{}:{}", peer_addr, LISTEN_PORT);
        println!("[conn] Connecting to {} at {}", peer_id, addr);
        let mut stream =
            TcpStream::connect(&addr)
                .await
                .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

        // Send our hello
        let my_id = get_my_id().await;
        write_frame(&mut stream, &Frame::Hello { id: my_id }).await?;

        // Read peer hello
        let peer_id = match read_frame(&mut stream).await {
            Ok(Some(Frame::Hello { id })) => id,
            Ok(Some(other)) => {
                return Err(format!("Expected hello, got: {:?}", other));
            }
            Ok(None) => return Err("Connection closed during hello handshake".to_string()),
            Err(e) => return Err(e),
        };

        println!("[conn] Connected to peer {}", peer_id);

        let (reader, writer) = stream.into_split();
        let conn = PeerConnection {
            peer_id: peer_id.clone(),
            addr: addr.clone(),
            writer: Mutex::new(writer),
        };

        {
            let mut conns = self.connections.lock().await;
            conns.insert(peer_id.clone(), conn);
        }

        self.spawn_read_loop(peer_id.clone(), reader);

        Ok(())
    }

    fn spawn_read_loop(self: &Arc<Self>, peer_id: String, reader: OwnedReadHalf) {
        let this = self.clone();
        let frame_tx = self.frame_tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut buf_reader = tokio::io::BufReader::new(reader);
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match read_frame(&mut buf_reader).await {
                    Ok(Some(frame)) => {
                        match &frame {
                            Frame::Ping => {
                                // Respond with pong on the same connection
                                let conns = this.connections.lock().await;
                                if let Some(conn) = conns.get(&peer_id) {
                                    let mut writer = conn.writer.lock().await;
                                    let _ = write_frame(&mut *writer, &Frame::Pong).await;
                                }
                            }
                            Frame::Pong => {
                                // Heartbeat response — nothing to do
                            }
                            _ => {
                                let _ = frame_tx.send(IncomingFrame {
                                    peer_id: peer_id.clone(),
                                    frame,
                                });
                            }
                        }
                    }
                    Ok(None) => {
                        println!("[conn] {} disconnected", peer_id);
                        break;
                    }
                    Err(e) => {
                        eprintln!("[conn] Read error from {}: {}", peer_id, e);
                        break;
                    }
                }
            }

            // Cleanup connection on disconnect
            let mut conns = this.connections.lock().await;
            conns.remove(&peer_id);
            // Notify offline
            let _ = frame_tx.send(IncomingFrame {
                peer_id: format!("__disconnected:{}", peer_id),
                frame: Frame::System {
                    content: format!("peer {} disconnected", peer_id),
                },
            });
        });
    }

    pub async fn send(&self, peer_id: &str, frame: &Frame) -> Result<(), String> {
        let conns = self.connections.lock().await;
        match conns.get(peer_id) {
            Some(conn) => {
                let mut writer = conn.writer.lock().await;
                write_frame(&mut *writer, frame).await
            }
            None => Err(format!("No connection to peer {}", peer_id)),
        }
    }

    pub async fn broadcast(&self, frame: &Frame) {
        let conns = self.connections.lock().await;
        for (pid, conn) in conns.iter() {
            let mut writer = conn.writer.lock().await;
            if let Err(e) = write_frame(&mut *writer, frame).await {
                eprintln!("[conn] Failed to broadcast to {}: {}", pid, e);
            }
        }
    }

    pub async fn connected_peers(&self) -> Vec<String> {
        let conns = self.connections.lock().await;
        conns.keys().cloned().collect()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

// Temporary — will be replaced by discovery module's identity
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;
static MY_ID: LazyLock<StdMutex<String>> = LazyLock::new(|| StdMutex::new(String::new()));

pub(crate) async fn set_my_id(id: String) {
    let mut my_id = MY_ID.lock().unwrap();
    *my_id = id;
}

async fn get_my_id() -> String {
    MY_ID.lock().unwrap().clone()
}
