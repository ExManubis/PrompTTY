use serde::{Deserialize, Serialize};

/// The CLI agent being used.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CLIAgentType {
    Claude,
    Gemini,
    Codex,
    Amp,
    Droid,
    OpenCode,
    Copilot,
    Pi,
    OhMyPi,
    Auggie,
    Cursor,
    Goose,
    Hermes,
    Vibe,
    Antigravity,
    /// Warp's own headless TUI, targeted by the code review panel as a CLI-agent-equivalent destination.
    WarpTui,
    Unknown,
}

/// Identifies the agent variant that triggered a notification.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAgentVariant {
    /// Warp's built-in agent (Oz).
    Oz,
    /// A CLI agent (e.g., Claude Code, Gemini CLI, etc.).
    CLIAgent(CLIAgentType),
}
