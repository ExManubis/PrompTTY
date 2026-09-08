//! No-op tracing initialization.
//!
//! Product OpenTelemetry/OTLP export was removed. Call sites still invoke [`init`]
//! during app startup; it is retained as an empty hook so the surrounding lifecycle
//! code does not need a larger rewrite.

use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    install_no_subscriber()?;
    Ok(Initialization::default())
}

fn install_no_subscriber() -> anyhow::Result<()> {
    // Prevent the `tracing` crate from writing log lines for spans/events unless a
    // real subscriber is installed elsewhere.
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(())
}

#[derive(Default)]
pub struct Initialization {
    initialization_warning: Option<anyhow::Error>,
}

impl Initialization {
    pub fn log_initialization_warning(&mut self) {
        if let Some(err) = self.initialization_warning.take() {
            log::warn!("Failed to initialize tracing: {err:#}");
        }
    }

    pub(crate) fn shutdown(&mut self) {}
}

impl Drop for Initialization {
    fn drop(&mut self) {
        self.shutdown();
    }
}
