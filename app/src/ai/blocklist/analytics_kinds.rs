//! Keep-types formerly co-located with product-analytics telemetry.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationApprovalStatus {
    Approved,
    Disapproved,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PillBarActionKind {
    /// User clicked the pill body. See `switch_outcome` for what
    /// happened next.
    Switch,
    OpenInNewPane,
    OpenInNewTab,
    /// User picked "Focus pane" from a pill's 3-dot menu. Distinct
    /// from a pill-body click that resolves to the same outcome
    /// (those are `Switch` with `switch_outcome = focused_existing_pane`).
    FocusOpenedConversation,
    Stop,
    Kill,
    TogglePinOn,
    TogglePinOff,
    ViewInOz,
    OpenMenu,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PillBarPillKind {
    Orchestrator,
    Child,
    /// A leading breadcrumb pill navigating back up the drill-down tree
    /// (to the tree root or the anchor's parent level).
    Breadcrumb,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PillSwitchOutcome {
    /// Pill click navigated within the current pane.
    SwitchedInPlace,
    /// Target conversation was already owned by another visible
    /// terminal view; focus moved there instead of switching in place.
    FocusedExistingPane,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunAgentsCardDecision {
    Accept,
    AcceptWithoutOrchestration,
    Reject,
}
