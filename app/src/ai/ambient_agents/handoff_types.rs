use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub enum HandoffSurface {
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    Gui,
    #[cfg_attr(not(feature = "tui"), allow(dead_code))]
    Tui,
}

/// The entry point through which a local-to-cloud handoff was initiated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffEntryPoint {
    /// User typed `&` in the input to enter handoff compose mode.
    #[default]
    Ampersand,
    /// User used the `/handoff` slash command.
    SlashCommand,
    /// User clicked the "Hand off to cloud" chip in the footer toolbar.
    FooterChip,
    /// The client automatically initiated handoff for an eligible local agent.
    Automatic,
}

/// Describes which synthetic-input path drives an empty-prompt handoff.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffInjectionPath {
    /// The handoff carried a non-empty user prompt; no client-side injection.
    #[default]
    None,
    /// Empty prompt + in-progress source. The client substituted `"Continue"`
    /// on the wire so the cloud agent picks up where the local agent left off.
    Continue,
    /// Empty prompt + idle source. The client substituted
    /// `"Apply the workspace changes from my previous session."` on the wire
    /// alongside the snapshot token; the cloud agent's first user-role turn
    /// carries an intent for the rehydrated workspace state.
    SnapshotRehydration,
}
