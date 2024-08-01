use async_trait::async_trait;
use futures_channel::mpsc::TrySendError;
use moosicbox_channel_utils::futures_channel::MoosicBoxUnboundedSender;
use moosicbox_ws::{WebsocketSendError, WebsocketSender};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use crate::sender::{TunnelResponseMessage, TunnelResponsePacket};

pub struct TunnelWebsocketSender<T>
where
    T: WebsocketSender + Send + Sync,
{
    pub id: usize,
    pub propagate_id: usize,
    pub request_id: usize,
    pub packet_id: u32,
    pub root_sender: T,
    pub tunnel_sender: MoosicBoxUnboundedSender<TunnelResponseMessage>,
}

impl<T> TunnelWebsocketSender<T>
where
    T: WebsocketSender + Send + Sync,
{
    fn send_tunnel(
        &self,
        data: &str,
        broadcast: bool,
        except_id: Option<usize>,
        only_id: Option<usize>,
    ) -> Result<(), TrySendError<TunnelResponseMessage>> {
        let body: Value = serde_json::from_str(data).unwrap();
        let request_id = self.request_id;
        let packet_id = self.packet_id;
        let value = json!({"request_id": request_id, "body": body});

        self.tunnel_sender
            .unbounded_send(TunnelResponseMessage::Packet(TunnelResponsePacket {
                request_id,
                packet_id,
                broadcast,
                except_id,
                only_id,
                message: Message::Text(value.to_string()),
            }))
    }
}

#[async_trait]
impl<T> WebsocketSender for TunnelWebsocketSender<T>
where
    T: WebsocketSender + Send + Sync,
{
    async fn send(&self, connection_id: &str, data: &str) -> Result<(), WebsocketSendError> {
        let id = connection_id.parse::<usize>().unwrap();

        if id == self.id {
            if self
                .send_tunnel(data, false, None, Some(self.propagate_id))
                .is_err()
            {
                log::error!("Failed to send tunnel message");
            }
        } else {
            self.root_sender.send(connection_id, data).await?;
        }

        Ok(())
    }

    async fn send_all(&self, data: &str) -> Result<(), WebsocketSendError> {
        if self.send_tunnel(data, true, None, None).is_err() {
            log::error!("Failed to send tunnel message");
        }

        self.root_sender.send_all(data).await?;

        Ok(())
    }

    async fn send_all_except(
        &self,
        connection_id: &str,
        data: &str,
    ) -> Result<(), WebsocketSendError> {
        let id = connection_id.parse::<usize>().unwrap();

        if id != self.propagate_id
            && self
                .send_tunnel(data, true, Some(self.propagate_id), None)
                .is_err()
        {
            log::error!("Failed to send tunnel message");
        }

        self.root_sender
            .send_all_except(connection_id, data)
            .await?;

        Ok(())
    }

    async fn ping(&self) -> Result<(), WebsocketSendError> {
        self.root_sender.ping().await
    }
}
