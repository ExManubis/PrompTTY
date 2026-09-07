pub use warp_terminal::local_tty::PtySpawnMode;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum PaletteSource {
    PrefixChange,
    Keybinding,
    CtrlTab { shift_pressed_initially: bool },
    WarpDrive,
    QuitModal,
    LogOutModal,
    IntegrationTest,
    ConversationManager,
    ContextChip,
    PaneHeader,
    AgentTip,
    TitleBarSearchBar,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FileTreeSource {
    /// Opened from the pane header toolbelt button.
    PaneHeader,
    Keybinding,
    LeftPanelToolbelt,
    ForceOpened,
    /// Opened from the CLI agent view footer (e.g., Claude Code).
    CLIAgentView,
    /// Opened from the File explorer chip in Warp's own agent input toolbelt.
    AgentToolbelt,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloseTarget {
    App,
    Window,
    Tab,
    Pane,
    EditorTab,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypointSelectionType {
    /// User entered Agent Mode by taking action on a blocklist text selection.
    Text,

    /// User entered Agent Mode by taking action on a block selection.
    Block,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypoint {
    /// The stars icon button in the tab bar.
    #[serde(rename = "tab_bar")]
    TabBar,

    /// This corresponds to _both_ triggering from the command palette and via keybinding.
    ///
    /// Unfortunately due to the way the command palette automatically surfaces any editable
    /// keybinding as an action, we don't have enough information to discern if the binding was
    /// triggered by the palette or keyboard.
    #[serde(rename = "new_pane_binding")]
    NewPaneBinding,

    /// The stars button in the hoverable block "toolbelt".
    #[serde(rename = "block_toolbelt")]
    BlockToolbelt,

    /// The "Ask Agent Mode" option from AI command search.
    #[serde(rename = "ai_command_search")]
    AICommandSearch,

    /// Context menu item(s) that attach a blocklist selection as context to an Agent Mode query.
    #[serde(rename = "context_menu")]
    ContextMenu {
        selection_type: AgentModeEntrypointSelectionType,
    },

    /// The Agent Mode chip in the prompt.
    #[serde(rename = "prompt_chip")]
    PromptChip,

    /// The Agent Management popup, where you can see all the most recent tasks for each terminal
    /// pane across all windows/tabs/panes.
    #[serde(rename = "agent_management_popup")]
    AgentManagementPopup,

    /// User manually switched between terminal and AI input modes in UDI interface
    #[serde(rename = "udi_terminal_input_switcher")]
    UDITerminalInputSwitcher,

    /// The agent management view, where you can see both local interactive and ambient agent tasks
    #[serde(rename = "agent_management_view")]
    AgentManagementView,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InteractionSource {
    Button,
    Keybinding,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum DownloadSource {
    Website,
    Homebrew,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SharingDialogSource {
    /// The sharing button in the pane header.
    PaneHeader,
    /// The per-pane command palette entry (includes keybindings).
    CommandPalette,
    /// The Warp Drive index context menu.
    DriveIndex,
    /// The sharing dialog was auto-opened from shared session creation.
    StartedSessionShare,
    /// The user intented into Warp with an email address to invite.
    InviteeRequest,
    /// The user jumped from an inherited ACL to its definition on a parent object.
    InheritedPermission,
    /// The onboarding block shown after users create new personal objects.
    OnboardingBlock,
    /// The conversation list overflow menu.
    ConversationList,
    /// The AI block context menu.
    AIBlockContextMenu,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum FindOption {
    CaseSensitive,
    FindInBlock,
    Regex,
}

#[cfg(feature = "local_fs")]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodePanelsFileOpenEntrypoint {
    CodeReview,
    ProjectExplorer,
    GlobalSearch,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginChipTelemetryKind {
    Install,
    Update,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WarpDriveSource {
    Legacy,
    LeftPanelToolbelt,
    ForceOpened,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SaveAsWorkflowModalSource {
    Block,
    Input,
    WarpAIWorkflowCard,
    WarpAIPanel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LaunchConfigUiLocation {
    CommandPalette,
    AppMenu,
    TabMenu,
    Uri,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnonymousUserSignupEntrypoint {
    HitDriveObjectLimit,
    LoginGatedFeature,
    SignUpButton,
    RenotificationBlock,
    SignUpAIPrompt,
    NextCommandSuggestionsUpgradeBanner,
    Unknown,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum KnowledgePaneEntrypoint {
    /// Triggered by either the command palette or the mac menus
    #[serde(rename = "global")]
    Global,

    #[serde(rename = "settings")]
    Settings,

    #[serde(rename = "warp_drive")]
    WarpDrive,

    #[serde(rename = "ai_blocklist")]
    AIBlocklist,

    #[serde(rename = "slash_command")]
    SlashCommand,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPServerCollectionPaneEntrypoint {
    /// Triggered by either the command palette or the mac menus
    #[serde(rename = "global")]
    Global,

    #[serde(rename = "settings")]
    Settings,

    #[serde(rename = "warp_drive")]
    WarpDrive,

    #[serde(rename = "slash_command")]
    SlashCommand,

    #[serde(rename = "mcp_settings_tab")]
    MCPSettingsTab,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionViewType {
    TerminalView,
    AgentView,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AgentModeRewindEntrypoint {
    /// The rewind button in the AI block header.
    Button,
    /// The context menu item "Rewind to before here".
    ContextMenu,
    /// The /rewind slash command.
    SlashCommand,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionFallbackReason {
    /// Code file had too many lines, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_lines")]
    FileTooManyLines,
    /// Code file had too many bytes, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_bytes")]
    FileTooManyBytes,
    /// Missing file, when looking up filepaths in local file system.
    #[serde(rename = "missing_file")]
    MissingFile,
    /// Failed to retrieve file from local file system.
    #[serde(rename = "failed_to_retrieve_file")]
    FailedToRetrieveFile,
    /// In an SSH/remote session.
    #[serde(rename = "ssh_remote_session")]
    SSHRemoteSession,
    /// No read files permission.
    #[serde(rename = "no_read_files_permission")]
    NoReadFilesPermission,
    /// AI query timeout.
    #[serde(rename = "ai_query_timeout")]
    AIQueryTimeout,
    /// Failed to send AI request.
    #[serde(rename = "failed_to_send_ai_request")]
    FailedToSendAIRequest,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeContextDestination {
    Pty,
    AgentInput,
    RichInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AddTabWithShellSource {
    CommandPalette,
    ShellSelectorMenu,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandXRayTrigger {
    Hover,
    Keystroke,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPTemplateInstallationSource {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "shared")]
    Shared,
    #[serde(rename = "gallery")]
    Gallery,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OpenedWarpAISource {
    GlobalEntryButton,
    HelpWithBlock,
    HelpWithTextSelection,
    FromAICommandSearch,
    WarmWelcome,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueuedPromptSendNowTrigger {
    SendNowButton,
    EnterOnEmptyInput,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteCodebaseAutoIndexTrigger {
    NavigatedToGitRepo,
    CodebaseContextEnablementChanged,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteCodebaseIndexStatusTelemetrySource {
    Snapshot,
    PushUpdate,
    MutationResponse,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToggleBlockFilterSource {
    /// This includes the keybinding and the command palette items.
    Binding,
    ContextMenu,
}

/// Where in the vertical tabs UI a clickable diff-stats or GitHub PR chip
/// was rendered when the user clicked it.
#[derive(Clone, Copy, Debug)]
pub enum VerticalTabsChipEntrypoint {
    /// The chip was rendered on a row representing a single pane
    /// (display granularity: Panes).
    Pane,
    /// The chip was rendered on a row representing a tab group
    /// (display granularity: Tabs).
    Tab,
    /// The chip was rendered inside the detail sidecar that appears on row hover.
    DetailsSidecar,
}
