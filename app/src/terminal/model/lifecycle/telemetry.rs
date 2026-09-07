//! Feature-specific telemetry events for lifecycle recovery.

use serde_json::{Value, json};
use strum_macros::{EnumDiscriminants, EnumIter};
use warp_core::telemetry::{EnablementState, TelemetryEvent, TelemetryEventDesc};

use super::recovery::LifecycleRecoveryRecord;

/// Defines feature-specific telemetry events emitted by the lifecycle coordinator.
#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub(in crate::terminal) enum LifecycleTelemetryEvent {
    /// Reports a conservative or recovery lifecycle transition.
    Recovery(LifecycleRecoveryRecord),
}

impl TelemetryEvent for LifecycleTelemetryEvent {
    fn name(&self) -> &'static str {
        LifecycleTelemetryEventDiscriminants::from(self).name()
    }

    fn payload(&self) -> Option<Value> {
        match self {
            LifecycleTelemetryEvent::Recovery(record) => Some(json!({
                "previous_phase": format!("{:?}", record.previous_phase),
                "next_phase": format!("{:?}", record.next_phase),
                "input_kind": format!("{:?}", record.input_kind),
                "action": format!("{:?}", record.action),
                "active_block_id": record.active_block_id,
                "supplied_next_block_id": record.supplied_next_block_id,
                "active_session_id": record.active_session_id,
                "hook_session_id": record.hook_session_id,
                "block_state": format!("{:?}", record.block_state),
                "started": record.started,
                "finished": record.finished,
                "received_precmd": record.received_precmd,
                "is_in_band": record.is_in_band,
                "is_bootstrapped": record.is_bootstrapped,
                "is_bootstrap_done": record.is_bootstrap_done,
                "is_alt_screen_active": record.is_alt_screen_active,
                "completion_mismatch": record.completion_mismatch,
                "suppressed_repeats": record.suppressed_repeats,
            })),
        }
    }

    fn description(&self) -> &'static str {
        LifecycleTelemetryEventDiscriminants::from(self).description()
    }

    fn enablement_state(&self) -> EnablementState {
        LifecycleTelemetryEventDiscriminants::from(self).enablement_state()
    }

    fn contains_ugc(&self) -> bool {
        false
    }

    fn event_descs() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>> {
        warp_core::telemetry::enum_events::<Self>()
    }
}

impl TelemetryEventDesc for LifecycleTelemetryEventDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            LifecycleTelemetryEventDiscriminants::Recovery => "Terminal Lifecycle Recovery",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            LifecycleTelemetryEventDiscriminants::Recovery => {
                "A terminal lifecycle transition required conservative handling or recovery"
            }
        }
    }

    fn enablement_state(&self) -> EnablementState {
        EnablementState::Always
    }
}

warp_core::register_telemetry_event!(LifecycleTelemetryEvent);
