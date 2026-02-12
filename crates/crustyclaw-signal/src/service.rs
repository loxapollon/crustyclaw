//! Async Signal service — bridges Signal messages to the core daemon message bus.

use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn};

use crustyclaw_core::message::{Direction, Envelope};

use crate::message::SignalMessage;
use crate::rate_limit::{RateLimitConfig, RateLimiter};
use crate::SignalError;

/// Commands that can be sent to the Signal service.
#[derive(Debug)]
pub enum ServiceCommand {
    /// Send a message via Signal.
    Send(SignalMessage),
    /// Shut down the service.
    Shutdown,
}

/// The async Signal service that runs as a tokio task.
///
/// Bridges inbound Signal messages to the daemon's message bus (as [`Envelope`]s)
/// and processes outbound messages from the bus back to Signal.
pub struct SignalService {
    /// Channel for receiving commands (send message, shutdown).
    command_rx: mpsc::Receiver<ServiceCommand>,

    /// Sender for the daemon's message bus.
    bus_tx: broadcast::Sender<Envelope>,

    /// Rate limiter for inbound messages.
    rate_limiter: RateLimiter,
}

/// Handle for interacting with a running SignalService.
pub struct SignalServiceHandle {
    command_tx: mpsc::Sender<ServiceCommand>,
}

impl SignalServiceHandle {
    /// Send a message via Signal.
    pub async fn send_message(&self, msg: SignalMessage) -> Result<(), SignalError> {
        self.command_tx
            .send(ServiceCommand::Send(msg))
            .await
            .map_err(|_| SignalError::SendFailed("service channel closed".to_string()))
    }

    /// Request the service to shut down.
    pub async fn shutdown(&self) -> Result<(), SignalError> {
        self.command_tx
            .send(ServiceCommand::Shutdown)
            .await
            .map_err(|_| SignalError::SendFailed("service channel closed".to_string()))
    }
}

impl SignalService {
    /// Create a new Signal service and return it with a handle for sending commands.
    pub fn new(
        bus_tx: broadcast::Sender<Envelope>,
        rate_limit_config: RateLimitConfig,
    ) -> (Self, SignalServiceHandle) {
        let (command_tx, command_rx) = mpsc::channel(256);

        let service = Self {
            command_rx,
            bus_tx,
            rate_limiter: RateLimiter::new(rate_limit_config),
        };

        let handle = SignalServiceHandle { command_tx };

        (service, handle)
    }

    /// Run the service event loop until shutdown.
    pub async fn run(mut self) {
        info!("Signal service started");

        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                ServiceCommand::Send(msg) => {
                    self.handle_outbound(msg);
                }
                ServiceCommand::Shutdown => {
                    info!("Signal service shutting down");
                    break;
                }
            }
        }

        info!("Signal service stopped");
    }

    /// Process an inbound Signal message (from Signal → daemon bus).
    ///
    /// This would be called when the Signal protocol layer receives a message.
    /// Currently exposed for testing; in production, the Signal protocol
    /// integration would call this internally.
    pub fn process_inbound(&mut self, msg: &SignalMessage) -> Result<(), SignalError> {
        // Rate limit check
        if !self.rate_limiter.check(&msg.sender) {
            warn!(sender = %msg.sender, "Rate limited");
            return Err(SignalError::RateLimited(format!(
                "sender {} exceeded rate limit",
                msg.sender
            )));
        }

        // Convert to Envelope and publish to bus
        let envelope = Envelope::new("signal", &msg.body);
        let _ = self.bus_tx.send(envelope);

        info!(sender = %msg.sender, "Inbound Signal message routed to bus");
        Ok(())
    }

    fn handle_outbound(&self, msg: SignalMessage) {
        let recipient = msg.recipient.as_deref().unwrap_or("unknown");
        info!(
            recipient = %recipient,
            body_len = msg.body.len(),
            "Outbound Signal message queued"
        );
        // TODO: When Signal protocol is integrated, actually deliver the message.
        // For now, publish to the bus as an outbound envelope for TUI visibility.
        let mut envelope = Envelope::new("signal", &msg.body);
        envelope.direction = Direction::Outbound;
        let _ = self.bus_tx.send(envelope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let (bus_tx, _bus_rx) = broadcast::channel(16);
        let (service, _handle) = SignalService::new(bus_tx, RateLimitConfig::default());
        // Service exists and can be dropped
        drop(service);
    }

    #[tokio::test]
    async fn test_service_shutdown() {
        let (bus_tx, _bus_rx) = broadcast::channel(16);
        let (service, handle) = SignalService::new(bus_tx, RateLimitConfig::default());

        let service_task = tokio::spawn(service.run());
        handle.shutdown().await.unwrap();
        service_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_inbound_message_routing() {
        let (bus_tx, mut bus_rx) = broadcast::channel(16);
        let config = RateLimitConfig {
            max_tokens: 10,
            refill_interval: std::time::Duration::from_secs(60),
        };
        let (mut service, _handle) = SignalService::new(bus_tx, config);

        let msg = SignalMessage::text("+1234567890", "Hello daemon");
        service.process_inbound(&msg).unwrap();

        let envelope = bus_rx.recv().await.unwrap();
        assert_eq!(envelope.body, "Hello daemon");
        assert_eq!(envelope.channel, "signal");
    }

    #[tokio::test]
    async fn test_inbound_rate_limiting() {
        let (bus_tx, _bus_rx) = broadcast::channel(16);
        let config = RateLimitConfig {
            max_tokens: 2,
            refill_interval: std::time::Duration::from_secs(60),
        };
        let (mut service, _handle) = SignalService::new(bus_tx, config);

        let msg = SignalMessage::text("+1", "msg");
        assert!(service.process_inbound(&msg).is_ok());
        assert!(service.process_inbound(&msg).is_ok());
        assert!(service.process_inbound(&msg).is_err()); // rate limited
    }

    #[tokio::test]
    async fn test_outbound_message() {
        let (bus_tx, mut bus_rx) = broadcast::channel(16);
        let (service, handle) = SignalService::new(bus_tx, RateLimitConfig::default());

        let service_task = tokio::spawn(service.run());

        let msg = SignalMessage::outbound("+0987654321", "Reply from agent");
        handle.send_message(msg).await.unwrap();

        let envelope = bus_rx.recv().await.unwrap();
        assert_eq!(envelope.body, "Reply from agent");
        assert_eq!(envelope.direction, Direction::Outbound);

        handle.shutdown().await.unwrap();
        service_task.await.unwrap();
    }
}
