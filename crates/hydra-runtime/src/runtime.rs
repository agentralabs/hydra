use std::sync::Arc;

use crate::boot::{BootError, BootSequence};
use crate::config::HydraRuntimeConfig;
use crate::event_bus::EventBus;
use crate::shutdown::ShutdownSequence;

/// The Hydra runtime — coordinates all subsystems
pub struct HydraRuntime {
    config: HydraRuntimeConfig,
    event_bus: Arc<EventBus>,
    shutdown: ShutdownSequence,
    booted: bool,
}

impl HydraRuntime {
    pub fn new(config: HydraRuntimeConfig) -> Self {
        Self {
            config,
            event_bus: Arc::new(EventBus::new(1024)),
            shutdown: ShutdownSequence::new(),
            booted: false,
        }
    }

    /// Boot the runtime
    pub async fn boot(&mut self) -> Result<(), BootError> {
        let mut boot = BootSequence::new(self.config.clone());
        boot.execute(&self.event_bus).await?;
        self.booted = true;
        Ok(())
    }

    /// Shutdown the runtime
    pub async fn shutdown(&self, reason: &str) -> crate::shutdown::ShutdownResult {
        self.shutdown.execute(&self.event_bus, reason).await
    }

    /// Check if runtime is booted
    pub fn is_booted(&self) -> bool {
        self.booted
    }

    /// Check if shutdown is in progress
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.is_shutting_down()
    }

    /// Get event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get config
    pub fn config(&self) -> &HydraRuntimeConfig {
        &self.config
    }
}
