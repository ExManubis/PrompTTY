//! No-op tracing initialization.
//!
//! Product OpenTelemetry/OTLP export was removed. Call sites still invoke [`init`] /
//! [`start_auth_refresh`] during app startup; they are retained as empty hooks so the
//! surrounding lifecycle code does not need a larger rewrite.

use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    install_no_subscriber()?;
    Ok(Initialization::default())
}

/// Previously started cloud-agent OTLP credential refresh. Retained as a no-op.
pub fn start_auth_refresh(
    _client: std::sync::Arc<dyn warp_managed_secrets::client::ManagedSecretsClient>,
    _ctx: &mut warpui::AppContext,
) {
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
