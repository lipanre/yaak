use crate::connect::{message_size_limit, ws_connect};
use crate::error::Error::GenericError;
use crate::error::Result;
use crate::stomp::frame_command;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use http::HeaderMap;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use yaak_tls::ClientCertificateConfig;

type WebsocketSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

struct WebsocketConnection {
    max_message_size: Option<usize>,
    sink: WebsocketSink,
}

#[derive(Clone)]
pub struct WebsocketManager {
    connections: Arc<Mutex<HashMap<String, WebsocketConnection>>>,
    read_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl WebsocketManager {
    pub fn new() -> Self {
        WebsocketManager { connections: Default::default(), read_tasks: Default::default() }
    }

    pub async fn connect(
        &mut self,
        id: &str,
        url: &str,
        headers: HeaderMap<HeaderValue>,
        receive_tx: mpsc::Sender<Message>,
        validate_certificates: bool,
        client_cert: Option<ClientCertificateConfig>,
        request_message_size: i32,
    ) -> Result<Response> {
        let tx = receive_tx.clone();
        let max_message_size = message_size_limit(request_message_size);

        let (stream, response) =
            ws_connect(url, headers, validate_certificates, client_cert, request_message_size)
                .await?;
        let (write, mut read) = stream.split();

        self.connections
            .lock()
            .await
            .insert(id.to_string(), WebsocketConnection { max_message_size, sink: write });

        let handle = {
            let connection_id = id.to_string();
            let connections = self.connections.clone();
            let read_tasks = self.read_tasks.clone();
            tokio::task::spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Err(e) => {
                            warn!("Broken websocket connection: {}", e);
                            break;
                        }
                        Ok(message) => tx.send(message).await.unwrap(),
                    }
                }
                debug!("Connection {} closed", connection_id);
                connections.lock().await.remove(&connection_id);
                read_tasks.lock().await.remove(&connection_id);
            })
        };

        self.read_tasks.lock().await.insert(id.to_string(), handle);

        Ok(response)
    }

    pub async fn send(&mut self, id: &str, msg: Message) -> Result<()> {
        let mut connections = self.connections.lock().await;
        let connection = match connections.get_mut(id) {
            None => return Ok(()),
            Some(c) => c,
        };
        if let Some(limit) = connection.max_message_size {
            let message_size = msg.len();
            if message_size > limit {
                return Err(GenericError(format!(
                    "WebSocket message too large: found {message_size} bytes, the limit is {limit} bytes"
                )));
            }
        }
        connection.sink.send(msg).await?;
        Ok(())
    }

    pub async fn connect_stomp(
        &mut self,
        id: &str,
        url: &str,
        headers: HeaderMap<HeaderValue>,
        receive_tx: mpsc::Sender<Message>,
        validate_certificates: bool,
        client_cert: Option<ClientCertificateConfig>,
        request_message_size: i32,
        connect_frame: String,
        subscribe_frames: Vec<String>,
    ) -> Result<(Response, Vec<Message>)> {
        let tx = receive_tx.clone();
        let max_message_size = message_size_limit(request_message_size);

        let (stream, response) =
            ws_connect(url, headers, validate_certificates, client_cert, request_message_size)
                .await?;
        let (mut write, mut read) = stream.split();
        write.send(Message::Text(connect_frame.into())).await?;

        let mut initial_messages = Vec::new();
        let connected = async {
            loop {
                match read.next().await {
                    Some(Ok(message)) => {
                        initial_messages.push(message.clone());
                        if let Message::Text(text) = &message {
                            match frame_command(text) {
                                "CONNECTED" => break Ok(()),
                                "ERROR" => {
                                    break Err(GenericError(format!(
                                        "STOMP connection failed: {text}"
                                    )));
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Err(e)) => break Err(GenericError(e.to_string())),
                    None => {
                        break Err(GenericError(
                            "WebSocket closed before STOMP CONNECTED frame".to_string(),
                        ));
                    }
                }
            }
        };

        match tokio::time::timeout(Duration::from_secs(10), connected).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(GenericError(
                    "Timed out waiting for STOMP CONNECTED frame".to_string(),
                ));
            }
        }

        for frame in subscribe_frames {
            write.send(Message::Text(frame.into())).await?;
        }

        self.connections
            .lock()
            .await
            .insert(id.to_string(), WebsocketConnection { max_message_size, sink: write });

        let handle = {
            let connection_id = id.to_string();
            let connections = self.connections.clone();
            let read_tasks = self.read_tasks.clone();
            tokio::task::spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Err(e) => {
                            warn!("Broken websocket connection: {}", e);
                            break;
                        }
                        Ok(message) => tx.send(message).await.unwrap(),
                    }
                }
                debug!("Connection {} closed", connection_id);
                connections.lock().await.remove(&connection_id);
                read_tasks.lock().await.remove(&connection_id);
            })
        };

        self.read_tasks.lock().await.insert(id.to_string(), handle);

        Ok((response, initial_messages))
    }

    pub async fn close(&mut self, id: &str) -> Result<()> {
        info!("Closing websocket");
        if let Some(mut connection) = self.connections.lock().await.remove(id) {
            // Wait a maximum of 1 second for the connection to close
            if let Err(e) = connection.sink.close().await {
                warn!("Failed to close websocket connection {e:?}");
            };
        }

        // Wait at short time for the server to close the connection, then stop
        // reading.
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(handle) = self.read_tasks.lock().await.remove(id) {
            handle.abort();
        }

        Ok(())
    }
}
