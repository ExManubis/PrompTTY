//! The "AI" settings page, shown under the Agents umbrella.
//!
//! Covers Warp's own AI: the global toggle, Active AI suggestions, agent
//! input behavior, credentials (Bedrock, Gemini Enterprise,
//! custom routers) and the miscellaneous agent display settings.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Not;
#[cfg(feature = "local_fs")]
use std::path::PathBuf;
use std::sync::LazyLock;

use ::ai::api_keys::{ApiKeyManager, ApiKeyManagerEvent};
use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use settings::{Setting, ToggleableSetting};
use strum::IntoEnumIterator;
use warp_core::context_flag::ContextFlag;
use warp_core::features::FeatureFlag;
use warp_errors::report_if_error;
use warpui::elements::{
    Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Expanded, Flex,
    FormattedTextElement, HighlightedHyperlink, HyperlinkUrl, MainAxisSize, MouseStateHandle,
    ParentElement, Radius, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::keymap::ContextPredicate;
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::ui_components::switch::{SwitchStateHandle, TooltipConfig};
use warpui::{
    Action, AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle, WeakViewHandle, id,
};

use super::ai_shared::{
    render_ai_feature_switch, render_ai_setting_description, render_ai_setting_label,
    render_ai_setting_toggle, render_toolbar_layout_editor, styles,
    update_editor_interaction_state,
};
use super::settings_page::{
    CONTENT_FONT_SIZE, Category, CategoryHeader, HEADER_PADDING, LocalOnlyIconState, MatchData,
    PageTitle, PageType, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
    TOGGLE_BUTTON_RIGHT_PADDING, ToggleState, build_toggle_element, render_body_item_label,
    render_dropdown_item,
};
use super::{
    SettingActionPairContexts, SettingActionPairDescriptions, SettingsAction, SettingsSection,
    ToggleSettingActionPair, editor_text_colors, flags,
};
use crate::UserWorkspaces;
#[cfg(not(target_family = "wasm"))]
use crate::ai::aws_credentials::refresh_aws_credentials;
use crate::ai::blocklist::agent_view::agent_input_footer::editor::{
    AgentToolbarEditorMode, AgentToolbarInlineEditor,
};
#[cfg(not(target_family = "wasm"))]
use crate::ai::geap_credentials::force_refresh_geap_credentials;
use crate::appearance::Appearance;
use crate::editor::{
    EditorOptions, EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys,
    SingleLineEditorOptions, TextColors, TextOptions,
};
use crate::settings::{
    AIAutoDetectionEnabled, AICommandDenylist, AISettings, AISettingsChangedEvent,
    AgentModeQuerySuggestionsEnabled, AutoApproveBypassesCommandDenylist, AwsBedrockAutoLogin,
    AwsBedrockCredentialsEnabled, DEFAULT_OPENROUTER_CODE_REVIEW_MODEL,
    EnableAiCommandSearchHashTrigger, GeminiEnterpriseCredentialsEnabled,
    GitOperationsAutogenEnabled, IncludeAgentCommandsInHistory, InputSettings,
    IntelligentAutosuggestionsEnabled, LongRunningCommandSubmissionMode, NLDInTerminalEnabled,
    NaturalLanguageAutosuggestionsEnabled, OrchestrationMessageDisplayMode, PromptSubmissionMode,
    SharedBlockTitleGenerationEnabled, ShouldRenderUseAgentToolbarForUserCommands,
    ShouldShowOzUpdatesInZeroState, ShowAgentTips, ShowConversationHistory, ShowHintText,
    ThinkingDisplayMode,
};
use crate::ui_components::icons::Icon;
use crate::util::bindings;
use crate::view_components::action_button::{ActionButton, ButtonSize, SecondaryTheme};
use crate::view_components::{Dropdown, DropdownItem};
use crate::workspaces::user_workspaces::UserWorkspacesEvent;
use crate::workspaces::workspace::{AdminEnablementSetting, CustomerType};

const AI_SETTINGS_DROPDOWN_WIDTH: f32 = 250.;
const AI_SETTINGS_DROPDOWN_MAX_HEIGHT: f32 = 250.;

const NEXT_COMMAND_DESCRIPTION: &str = "Let AI suggest the next command to run based on your command history, outputs, and common workflows.";
const PROMPT_SUGGESTIONS_DESCRIPTION: &str = "Let AI suggest natural language prompts, as inline banners in the input, based on recent commands and their outputs.";
const SUGGESTED_CODE_BANNERS_DESCRIPTION: &str = "Let AI suggest code diffs and queries as inline banners in the blocklist, based on recent commands and their outputs.";
const NATURAL_LANGUAGE_AUTOSUGGESTIONS: &str =
    "Let AI suggest natural language autosuggestions, based on recent commands and their outputs.";
const SHARED_BLOCK_TITLE_GENERATION_DESCRIPTION: &str =
    "Let AI generate a title for your shared block based on the command and output.";
const GIT_OPERATIONS_AUTOGEN_DESCRIPTION: &str =
    "Let AI generate commit messages and pull request titles and descriptions.";

pub fn init_actions_from_parent_view<T: Action + Clone>(
    app: &mut AppContext,
    context: &ContextPredicate,
    builder: fn(SettingsAction) -> T,
) {
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "AI",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleGlobalAI,
                )),
                context,
                flags::IS_ANY_AI_ENABLED,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );

    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "Active AI",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleActiveAI,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::IS_ACTIVE_AI_ENABLED,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );

    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                if FeatureFlag::AgentView.is_enabled() {
                    "terminal command autodetection in agent input"
                } else {
                    "natural language detection"
                },
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleAIInputAutoDetection,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::AI_INPUT_AUTODETECTION_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::AgentMode.is_enabled()),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "agent prompt autodetection in terminal input",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleNLDInTerminal,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::NLD_IN_TERMINAL_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::AgentView.is_enabled()),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "'#' trigger for AI command search",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleAiCommandSearchHashTrigger,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::AI_COMMAND_SEARCH_HASH_TRIGGER_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "Next Command",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleIntelligentAutosuggestions,
                )),
                &(context.clone() & id!(flags::IS_ACTIVE_AI_ENABLED)),
                flags::INTELLIGENT_AUTOSUGGESTIONS_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "prompt suggestions",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::TogglePromptSuggestions,
                )),
                &(context.clone() & id!(flags::IS_ACTIVE_AI_ENABLED)),
                flags::PROMPT_SUGGESTIONS_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "code suggestions",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleCodeSuggestions,
                )),
                &(context.clone()
                    & id!(flags::IS_ACTIVE_AI_ENABLED)
                    & id!(flags::PROMPT_SUGGESTIONS_FLAG)),
                flags::CODE_SUGGESTIONS_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::custom(
                SettingActionPairDescriptions::new("Show agent tips", "Hide agent tips"),
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleShowAgentTips,
                )),
                SettingActionPairContexts::new(
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & !id!(flags::SHOW_AGENT_TIPS_FLAG),
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & id!(flags::SHOW_AGENT_TIPS_FLAG),
                ),
                None,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::AgentTips.is_enabled()),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::custom(
                SettingActionPairDescriptions::new(
                    "Show Warp Agent changelog in new agent conversation view",
                    "Hide Warp Agent changelog in new agent conversation view",
                ),
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleShowOzUpdatesInZeroState,
                )),
                SettingActionPairContexts::new(
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & !id!(flags::SHOW_OZ_UPDATES_IN_ZERO_STATE_FLAG),
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & id!(flags::SHOW_OZ_UPDATES_IN_ZERO_STATE_FLAG),
                ),
                None,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::AgentView.is_enabled()),
        ],
        app,
    );
    {
        use warpui::keymap::FixedBinding;

        use crate::settings::ThinkingDisplayMode;

        let ai_context = context.clone() & id!(flags::IS_ANY_AI_ENABLED);
        let mode_bindings: Vec<FixedBinding> = ThinkingDisplayMode::iter()
            .map(|mode| {
                let context_flag = match mode {
                    ThinkingDisplayMode::ShowAndCollapse => {
                        flags::THINKING_DISPLAY_SHOW_AND_COLLAPSE
                    }
                    ThinkingDisplayMode::AlwaysShow => flags::THINKING_DISPLAY_ALWAYS_SHOW,
                    ThinkingDisplayMode::NeverShow => flags::THINKING_DISPLAY_NEVER_SHOW,
                };
                FixedBinding::empty(
                    mode.command_palette_description(),
                    builder(SettingsAction::WarpAgent(
                        WarpAgentPageAction::SetThinkingDisplayMode(mode),
                    )),
                    ai_context.clone() & !id!(context_flag),
                )
                .with_group(bindings::BindingGroup::WarpAi.as_str())
            })
            .collect();
        app.register_fixed_bindings(mode_bindings);
    }
    {
        use warpui::keymap::FixedBinding;

        let ai_context = context.clone() & id!(flags::IS_ANY_AI_ENABLED);
        let mode_bindings: Vec<FixedBinding> = OrchestrationMessageDisplayMode::iter()
            .map(|mode| {
                let context_flag = match mode {
                    OrchestrationMessageDisplayMode::ShowAndCollapse => {
                        flags::ORCHESTRATION_MESSAGE_DISPLAY_SHOW_AND_COLLAPSE
                    }
                    OrchestrationMessageDisplayMode::AlwaysShow => {
                        flags::ORCHESTRATION_MESSAGE_DISPLAY_ALWAYS_SHOW
                    }
                    OrchestrationMessageDisplayMode::AlwaysCollapse => {
                        flags::ORCHESTRATION_MESSAGE_DISPLAY_ALWAYS_COLLAPSE
                    }
                };
                FixedBinding::empty(
                    mode.command_palette_description(),
                    builder(SettingsAction::WarpAgent(
                        WarpAgentPageAction::SetOrchestrationMessageDisplayMode(mode),
                    )),
                    ai_context.clone() & !id!(context_flag),
                )
                .with_group(bindings::BindingGroup::WarpAi.as_str())
            })
            .collect();
        app.register_fixed_bindings(mode_bindings);
    }
    if FeatureFlag::QueueSlashCommand.is_enabled() {
        use warpui::keymap::FixedBinding;

        let ai_context = context.clone() & id!(flags::IS_ANY_AI_ENABLED);
        let mode_bindings: Vec<FixedBinding> = PromptSubmissionMode::iter()
            .map(|mode| {
                let context_flag = match mode {
                    PromptSubmissionMode::Interrupt => flags::PROMPT_SUBMISSION_INTERRUPT,
                    PromptSubmissionMode::Queue => flags::PROMPT_SUBMISSION_QUEUE,
                };
                FixedBinding::empty(
                    mode.command_palette_description(),
                    builder(SettingsAction::WarpAgent(
                        WarpAgentPageAction::SetPromptSubmissionMode(mode),
                    )),
                    ai_context.clone() & !id!(context_flag),
                )
                .with_group(bindings::BindingGroup::WarpAi.as_str())
            })
            .collect();
        app.register_fixed_bindings(mode_bindings);

        // The LRC submission mode only applies (and is only shown) when the default
        // prompt submission mode is Interrupt, so its palette entries are gated on it.
        let lrc_mode_bindings: Vec<FixedBinding> = LongRunningCommandSubmissionMode::iter()
            .map(|mode| {
                let context_flag = match mode {
                    LongRunningCommandSubmissionMode::SendImmediately => {
                        flags::LRC_SUBMISSION_SEND_IMMEDIATELY
                    }
                    LongRunningCommandSubmissionMode::QueueUntilCommandCompletes => {
                        flags::LRC_SUBMISSION_QUEUE_UNTIL_COMMAND_COMPLETES
                    }
                };
                FixedBinding::empty(
                    mode.command_palette_description(),
                    builder(SettingsAction::WarpAgent(
                        WarpAgentPageAction::SetLongRunningCommandSubmissionMode(mode),
                    )),
                    ai_context.clone()
                        & id!(flags::PROMPT_SUBMISSION_INTERRUPT)
                        & !id!(context_flag),
                )
                .with_group(bindings::BindingGroup::WarpAi.as_str())
            })
            .collect();
        app.register_fixed_bindings(lrc_mode_bindings);
    }
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "natural language autosuggestions",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleNaturalLanguageAutosuggestions,
                )),
                &(context.clone() & id!(flags::IS_ACTIVE_AI_ENABLED)),
                flags::NATURAL_LANGUAGE_AUTOSUGGESTIONS_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::PredictAMQueries.is_enabled()),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "shared block title generation",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleSharedTitleGeneration,
                )),
                &(context.clone() & id!(flags::IS_ACTIVE_AI_ENABLED)),
                flags::SHARED_BLOCK_TITLE_GENERATION_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| FeatureFlag::SharedBlockTitleGeneration.is_enabled()),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "commit and pull request generation",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleGitOperationsAutogen,
                )),
                &(context.clone() & id!(flags::IS_ACTIVE_AI_ENABLED)),
                flags::GIT_OPERATIONS_AUTOGEN_FLAG,
            )
            .with_enabled(|| FeatureFlag::GitOperationsInCodeReview.is_enabled())
            .is_supported_on_current_platform(
                AISettings::as_ref(app)
                    .git_operations_autogen_enabled_internal
                    .is_supported_on_current_platform()
                    && UserWorkspaces::as_ref(app).is_git_operations_ai_enabled(),
            ),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::custom(
                SettingActionPairDescriptions::new(
                    "Show \"Use Agent\" footer",
                    "Hide \"Use Agent\" footer",
                ),
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleUseAgentToolbar,
                )),
                SettingActionPairContexts::new(
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & !id!(flags::USE_AGENT_FOOTER_FLAG),
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & id!(flags::USE_AGENT_FOOTER_FLAG),
                ),
                None,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "include agent-executed commands in history",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleIncludeAgentCommandsInHistory,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::INCLUDE_AGENT_COMMANDS_IN_HISTORY_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi),
            ToggleSettingActionPair::custom(
                SettingActionPairDescriptions::new(
                    "Allow auto-approve to bypass command denylist",
                    "Require approval for denylisted commands in auto-approve",
                ),
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleAutoApproveBypassesCommandDenylist,
                )),
                SettingActionPairContexts::new(
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & !id!(flags::AUTO_APPROVE_BYPASSES_COMMAND_DENYLIST_FLAG),
                    context.clone()
                        & id!(flags::IS_ANY_AI_ENABLED)
                        & id!(flags::AUTO_APPROVE_BYPASSES_COMMAND_DENYLIST_FLAG),
                ),
                None,
            )
            .with_group(bindings::BindingGroup::WarpAi),
            ToggleSettingActionPair::new(
                "conversation history in tools panel",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleShowConversationHistory,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::SHOW_CONVERSATION_HISTORY,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "Auto-spawn servers from third-party agents",
                builder(SettingsAction::WarpAgent(
                    WarpAgentPageAction::ToggleFileBasedMcp,
                )),
                &(context.clone() & id!(flags::IS_ANY_AI_ENABLED)),
                flags::FILE_BASED_MCP_FLAG,
            )
            .with_group(bindings::BindingGroup::WarpAi)
            .with_enabled(|| {
                FeatureFlag::McpServer.is_enabled()
                    && FeatureFlag::FileBasedMcp.is_enabled()
                    && ContextFlag::ShowMCPServers.is_enabled()
            }),
        ],
        app,
    );
}

pub struct WarpAgentPageView {
    page: PageType<Self>,
    self_handle: WeakViewHandle<Self>,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
    autodetection_denylist_editor: ViewHandle<EditorView>,
    agent_toolbar_inline_editor: ViewHandle<AgentToolbarInlineEditor>,

    thinking_display_mode_dropdown: ViewHandle<Dropdown<WarpAgentPageAction>>,
    orchestration_message_display_mode_dropdown: ViewHandle<Dropdown<WarpAgentPageAction>>,
    default_prompt_submission_mode_dropdown: ViewHandle<Dropdown<WarpAgentPageAction>>,
    lrc_submission_mode_dropdown: ViewHandle<Dropdown<WarpAgentPageAction>>,
    #[cfg(feature = "local_fs")]
    conversation_layout_dropdown: ViewHandle<Dropdown<WarpAgentPageAction>>,

    // Custom model router views (gated on FeatureFlag::CustomModelRouters)
    #[cfg(feature = "local_fs")]
    router_views: Vec<ViewHandle<super::custom_router_view::CustomRouterView>>,
    #[cfg(feature = "local_fs")]
    add_router_button: ViewHandle<ActionButton>,
}

impl WarpAgentPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let self_handle = ctx.handle();
        let is_any_ai_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);

        let thinking_display_mode_dropdown =
            OtherAIWidget::create_thinking_display_mode_dropdown(ctx);
        // Set initial selection based on current setting value.
        {
            let current_mode = AISettings::as_ref(ctx).thinking_display_mode;
            thinking_display_mode_dropdown.update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    WarpAgentPageAction::SetThinkingDisplayMode(current_mode),
                    ctx,
                );
            });
        }
        let orchestration_message_display_mode_dropdown =
            OtherAIWidget::create_orchestration_message_display_mode_dropdown(ctx);
        {
            let current_mode = AISettings::as_ref(ctx).orchestration_message_display_mode;
            orchestration_message_display_mode_dropdown.update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    WarpAgentPageAction::SetOrchestrationMessageDisplayMode(current_mode),
                    ctx,
                );
            });
        }

        let default_prompt_submission_mode_dropdown =
            OtherAIWidget::create_default_prompt_submission_mode_dropdown(ctx);
        {
            let current_mode = AISettings::as_ref(ctx).default_prompt_submission_mode;
            default_prompt_submission_mode_dropdown.update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    WarpAgentPageAction::SetPromptSubmissionMode(current_mode),
                    ctx,
                );
            });
        }

        let lrc_submission_mode_dropdown = OtherAIWidget::create_lrc_submission_mode_dropdown(ctx);
        {
            let current_mode = AISettings::as_ref(ctx).long_running_command_submission_mode;
            lrc_submission_mode_dropdown.update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    WarpAgentPageAction::SetLongRunningCommandSubmissionMode(current_mode),
                    ctx,
                );
            });
        }

        let autodetection_denylist_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let options = EditorOptions {
                autogrow: true,
                soft_wrap: true,
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(TextColors {
                        default_color: appearance.theme().active_ui_text_color(),
                        disabled_color: appearance.theme().disabled_ui_text_color(),
                        hint_color: appearance.theme().disabled_ui_text_color(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);

            editor.set_placeholder_text("Commands, comma separated", ctx);

            let current_value = AISettings::as_ref(ctx)
                .autodetection_command_denylist
                .value()
                .clone();
            editor.set_buffer_text(current_value.as_str(), ctx);
            editor
        });
        update_editor_interaction_state(
            autodetection_denylist_editor.clone(),
            is_any_ai_enabled,
            ctx,
        );

        ctx.subscribe_to_view(&autodetection_denylist_editor, move |me, _, event, ctx| {
            me.handle_detection_denylist_editor_event(event, ctx);
        });

        ctx.subscribe_to_model(&UserWorkspaces::handle(ctx), |_, _handle, _event, ctx| {
            // Re-render if teams-related data changed that may affect whether features such as voice input are enabled.
            ctx.notify();
        });

        ctx.subscribe_to_model(&AISettings::handle(ctx), |me, _, event, ctx| {
            match event {
                AISettingsChangedEvent::AICommandDenylist { .. } => {
                    me.autodetection_denylist_editor.update(ctx, |editor, ctx| {
                        let denylist_value = &AISettings::as_ref(ctx)
                            .autodetection_command_denylist
                            .value()
                            .clone();
                        editor.set_buffer_text(denylist_value, ctx);
                    });
                }
                AISettingsChangedEvent::IsAnyAIEnabled { .. } => {
                    let is_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);

                    update_editor_interaction_state(
                        me.autodetection_denylist_editor.clone(),
                        is_enabled,
                        ctx,
                    );
                }
                AISettingsChangedEvent::ThinkingDisplayMode { .. } => {
                    let current_mode = *AISettings::as_ref(ctx).thinking_display_mode.value();
                    me.thinking_display_mode_dropdown
                        .update(ctx, |dropdown, ctx| {
                            dropdown.set_selected_by_action(
                                WarpAgentPageAction::SetThinkingDisplayMode(current_mode),
                                ctx,
                            );
                        });
                }
                AISettingsChangedEvent::OrchestrationMessageDisplayMode { .. } => {
                    let current_mode = AISettings::as_ref(ctx).orchestration_message_display_mode;
                    me.orchestration_message_display_mode_dropdown
                        .update(ctx, |dropdown, ctx| {
                            dropdown.set_selected_by_action(
                                WarpAgentPageAction::SetOrchestrationMessageDisplayMode(
                                    current_mode,
                                ),
                                ctx,
                            );
                        });
                }
                AISettingsChangedEvent::PromptSubmissionMode { .. } => {
                    let current_mode = AISettings::as_ref(ctx).default_prompt_submission_mode;
                    me.default_prompt_submission_mode_dropdown
                        .update(ctx, |dropdown, ctx| {
                            dropdown.set_selected_by_action(
                                WarpAgentPageAction::SetPromptSubmissionMode(current_mode),
                                ctx,
                            );
                        });
                }
                AISettingsChangedEvent::LongRunningCommandSubmissionMode { .. } => {
                    let current_mode = AISettings::as_ref(ctx).long_running_command_submission_mode;
                    me.lrc_submission_mode_dropdown
                        .update(ctx, |dropdown, ctx| {
                            dropdown.set_selected_by_action(
                                WarpAgentPageAction::SetLongRunningCommandSubmissionMode(
                                    current_mode,
                                ),
                                ctx,
                            );
                        });
                }
                _ => (),
            }
            ctx.notify();
        });

        ctx.subscribe_to_model(&InputSettings::handle(ctx), |_, _, _, ctx| {
            ctx.notify();
        });

        #[cfg(feature = "local_fs")]
        let router_views = Self::create_router_views(ctx);
        #[cfg(feature = "local_fs")]
        let add_router_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("+ Add router", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::OpenAddCustomRouter);
                })
        });
        #[cfg(feature = "local_fs")]
        {
            let is_enabled = warp_core::features::FeatureFlag::CustomModelRouters.is_enabled()
                && is_any_ai_enabled;
            add_router_button.update(ctx, |button, ctx| {
                button.set_disabled(!is_enabled, ctx);
            });
        }

        let agent_toolbar_inline_editor = ctx.add_typed_action_view(|ctx| {
            AgentToolbarInlineEditor::new(AgentToolbarEditorMode::AgentView, ctx)
        });

        #[cfg(feature = "local_fs")]
        let conversation_layout_dropdown = ctx.add_typed_action_view(|ctx| {
            use crate::util::file::external_editor::settings::OpenConversationPreference;

            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(AI_SETTINGS_DROPDOWN_WIDTH);
            dropdown.set_menu_width(AI_SETTINGS_DROPDOWN_WIDTH, ctx);

            let items = vec![
                DropdownItem::new(
                    "New Tab",
                    WarpAgentPageAction::SetConversationLayout(OpenConversationPreference::NewTab),
                ),
                DropdownItem::new(
                    "Split Pane",
                    WarpAgentPageAction::SetConversationLayout(
                        OpenConversationPreference::SplitPane,
                    ),
                ),
            ];
            dropdown.set_items(items, ctx);

            let current = *crate::util::file::external_editor::EditorSettings::as_ref(ctx)
                .open_conversation_layout_preference;
            match current {
                OpenConversationPreference::NewTab => dropdown.set_selected_by_name("New Tab", ctx),
                OpenConversationPreference::SplitPane => {
                    dropdown.set_selected_by_name("Split Pane", ctx)
                }
            };
            dropdown
        });

        // Subscribe to WarpConfig to refresh router views when files change.
        #[cfg(feature = "local_fs")]
        ctx.subscribe_to_model(
            &crate::user_config::WarpConfig::handle(ctx),
            |me, _, event, ctx| {
                use crate::user_config::WarpConfigUpdateEvent;
                if matches!(event, WarpConfigUpdateEvent::ModelConfigs) {
                    me.router_views = Self::create_router_views(ctx);
                    ctx.notify();
                }
            },
        );

        Self {
            page: Self::build_page(ctx),
            self_handle,
            autodetection_denylist_editor,
            local_only_icon_tooltip_states: Default::default(),
            agent_toolbar_inline_editor,
            thinking_display_mode_dropdown,
            orchestration_message_display_mode_dropdown,
            default_prompt_submission_mode_dropdown,
            lrc_submission_mode_dropdown,
            #[cfg(feature = "local_fs")]
            conversation_layout_dropdown,
            #[cfg(feature = "local_fs")]
            router_views,
            #[cfg(feature = "local_fs")]
            add_router_button,
        }
    }

    fn build_page(ctx: &mut ViewContext<Self>) -> PageType<Self> {
        let ai_settings = AISettings::as_ref(ctx);

        let mut categories: Vec<Category<Self>> = Vec::new();

        if ai_settings
            .intelligent_autosuggestions_enabled_internal
            .is_supported_on_current_platform()
            || ai_settings
                .prompt_suggestions_enabled_internal
                .is_supported_on_current_platform()
            || (FeatureFlag::PredictAMQueries.is_enabled()
                && ai_settings
                    .natural_language_autosuggestions_enabled_internal
                    .is_supported_on_current_platform())
            || (FeatureFlag::SharedBlockTitleGeneration.is_enabled()
                && ai_settings
                    .shared_block_title_generation_enabled_internal
                    .is_supported_on_current_platform())
            || (FeatureFlag::GitOperationsInCodeReview.is_enabled()
                && ai_settings
                    .git_operations_autogen_enabled_internal
                    .is_supported_on_current_platform())
        {
            let active_ai_widgets: Vec<Box<dyn SettingsWidget<View = Self>>> = vec![
                Box::new(NextCommandWidget::default()),
                Box::new(PromptSuggestionsWidget::default()),
                Box::new(SuggestedCodeBannersWidget::default()),
                Box::new(NaturalLanguageAutosuggestionsWidget::default()),
                Box::new(SharedBlockTitleGenerationWidget::new(ctx)),
                Box::new(GitOperationsAutogenWidget::default()),
            ];
            let active_ai_toggle = SwitchStateHandle::default();
            categories.push(Category::with_header(
                CategoryHeader::new("Active AI").with_trailing_element(
                    move |_view, _appearance, app| render_active_ai_toggle(&active_ai_toggle, app),
                ),
                active_ai_widgets,
            ));
        }

        categories.push(Category::new(
            "Input",
            vec![
                Box::new(NaturalLanguageDetectionWidget::default()),
                Box::new(ShowInputHintTextWidget::default()),
                Box::new(AiCommandSearchHashTriggerWidget::default()),
                Box::new(ShowAgentTipsWidget::default()),
                Box::new(IncludeAgentCommandsInHistoryWidget::default()),
                Box::new(AutoApproveBypassesCommandDenylistWidget::default()),
                Box::new(PromptSubmissionModeWidget),
            ],
        ));

        categories.push(Category::new(
            "Cloud Handoff",
            vec![
                Box::new(CloudHandoffWidget::default()),
                Box::new(AutoHandoffOnSleepWidget::default()),
                Box::new(AmpersandHandoffWidget::default()),
            ],
        ));

        categories.push(Category::with_header(
            CategoryHeader::new("OpenRouter").with_subtitle(
                "Generates commit messages and pull request titles and descriptions",
            ),
            vec![
                Box::new(OpenRouterApiKeyWidget::new(ctx)),
                Box::new(OpenRouterModelWidget::new(ctx)),
            ],
        ));

        categories.push(Category::new(
            "AWS Bedrock",
            vec![Box::new(AwsBedrockWidget::new(ctx))],
        ));

        categories.push(Category::new(
            "Gemini Enterprise",
            vec![Box::new(GeminiEnterpriseWidget::new(ctx))],
        ));

        if FeatureFlag::CustomModelRouters.is_enabled() {
            #[allow(clippy::vec_init_then_push)]
            let custom_router_widgets: Vec<Box<dyn SettingsWidget<View = Self>>> = {
                let mut widgets: Vec<Box<dyn SettingsWidget<View = Self>>> = Vec::new();
                #[cfg(feature = "local_fs")]
                widgets.push(Box::new(AddCustomRouterWidget));
                widgets.push(Box::new(CustomModelRoutersWidget));
                widgets
            };
            categories.push(Category::new("Custom Routers", custom_router_widgets));
        }

        categories.push(Category::new(
            "Agent Attribution",
            vec![Box::new(AgentAttributionWidget::default())],
        ));

        #[cfg_attr(not(feature = "local_fs"), allow(unused_mut))]
        let mut other_widgets: Vec<Box<dyn SettingsWidget<View = Self>>> = vec![
            Box::new(ShowOzUpdatesInZeroStateWidget::default()),
            Box::new(UseAgentFooterWidget::default()),
            Box::new(AgentToolbarLayoutEditorWidget),
            Box::new(ShowConversationHistoryWidget::default()),
            Box::new(ThinkingDisplayModeWidget),
            Box::new(OrchestrationMessageDisplayModeWidget),
        ];
        #[cfg(feature = "local_fs")]
        other_widgets.push(Box::new(ConversationLayoutPreferenceWidget));
        categories.push(Category::new("Other", other_widgets));

        if FeatureFlag::AgentModeComputerUse.is_enabled() {
            categories.push(Category::new(
                "Experimental",
                vec![Box::new(CloudAgentComputerUseWidget::default())],
            ));
        }

        let global_ai_switch_state = SwitchStateHandle::default();
        PageType::new_categorized(
            categories,
            Some(
                PageTitle::new("AI").with_trailing_element(move |_view, appearance, app| {
                    render_global_ai_toggle(&global_ai_switch_state, appearance, app)
                }),
            ),
        )
    }

    fn handle_detection_denylist_editor_event(
        &mut self,
        event: &EditorEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            EditorEvent::Blurred | EditorEvent::Enter => {
                let buffer_text = self
                    .autodetection_denylist_editor
                    .as_ref(ctx)
                    .buffer_text(ctx);
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    if let Err(e) = settings
                        .autodetection_command_denylist
                        .set_value(buffer_text, ctx)
                    {
                        log::warn!("Failed to set AI autodetection blacklist commands: {e:?}");
                    }
                })
            }
            EditorEvent::Escape => ctx.emit(WarpAgentPageEvent::FocusModal),
            _ => {}
        }
    }

    #[cfg(feature = "local_fs")]
    fn create_router_views(
        ctx: &mut ViewContext<Self>,
    ) -> Vec<ViewHandle<super::custom_router_view::CustomRouterView>> {
        use super::custom_router_view::{CustomRouterView, CustomRouterViewEvent};
        use crate::user_config::WarpConfig;
        if !warp_core::features::FeatureFlag::CustomModelRouters.is_enabled() {
            return Vec::new();
        }
        let routers: Vec<crate::ai::custom_model_routers::CustomModelRouter> =
            WarpConfig::as_ref(ctx).custom_model_routers().clone();
        routers
            .into_iter()
            .map(|router| {
                let router_clone = router.clone();
                let view = ctx.add_typed_action_view(|ctx| CustomRouterView::new(router, ctx));
                ctx.subscribe_to_view(&view, move |me, _, event, ctx| match event {
                    CustomRouterViewEvent::OpenFile(path) => {
                        ctx.emit(WarpAgentPageEvent::OpenCustomRouterFile(path.clone()));
                    }
                    CustomRouterViewEvent::Edit => {
                        let r = router_clone.clone();
                        ctx.emit(WarpAgentPageEvent::OpenCustomRouterEditor(Some(r)));
                    }
                    CustomRouterViewEvent::Delete => {
                        if let Some(path) = &router_clone.source_path {
                            #[cfg(feature = "local_fs")]
                            {
                                if let Err(e) =
                                    crate::user_config::WarpConfig::delete_custom_model_router(path)
                                {
                                    log::warn!("Failed to delete custom router: {e:?}");
                                }
                            }
                            me.router_views = Self::create_router_views(ctx);
                            ctx.notify();
                        }
                    }
                });
                view
            })
            .collect()
    }
}

impl View for WarpAgentPageView {
    fn ui_name() -> &'static str {
        "WarpAgentPage"
    }

    fn render(&self, app: &warpui::AppContext) -> Box<dyn warpui::Element> {
        self.page.render(self, app)
    }
}

#[allow(clippy::large_enum_variant)]
pub enum WarpAgentPageEvent {
    FocusModal,
    #[cfg(feature = "local_fs")]
    OpenCustomRouterEditor(Option<crate::ai::custom_model_routers::CustomModelRouter>),
    #[cfg(feature = "local_fs")]
    OpenCustomRouterFile(PathBuf),
    SignupAnonymousUser,
}

impl Entity for WarpAgentPageView {
    type Event = WarpAgentPageEvent;
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarpAgentPageAction {
    OpenUrl(String),
    ToggleGlobalAI,
    ToggleActiveAI,
    ToggleIntelligentAutosuggestions,
    TogglePromptSuggestions,
    ToggleCodeSuggestions,
    ToggleNaturalLanguageAutosuggestions,
    ToggleSharedTitleGeneration,
    ToggleGitOperationsAutogen,
    ToggleAIInputAutoDetection,
    ToggleNLDInTerminal,
    ToggleUseAgentToolbar,
    HyperlinkClick(HyperlinkUrl),
    ToggleShowInputHintText,
    ToggleAiCommandSearchHashTrigger,
    ToggleShowAgentTips,
    ToggleShowOzUpdatesInZeroState,
    SetThinkingDisplayMode(ThinkingDisplayMode),
    SetOrchestrationMessageDisplayMode(OrchestrationMessageDisplayMode),
    SetPromptSubmissionMode(PromptSubmissionMode),
    SetLongRunningCommandSubmissionMode(LongRunningCommandSubmissionMode),
    SignupAnonymousUser,
    ToggleAwsBedrockAutoLogin,
    ToggleAwsBedrockCredentialsEnabled,
    RefreshAwsBedrockCredentials,
    RefreshGeminiEnterpriseCredentials,
    ToggleGeminiEnterpriseCredentialsEnabled,
    ToggleCloudAgentComputerUse,
    ToggleFileBasedMcp,
    ToggleIncludeAgentCommandsInHistory,
    ToggleAutoApproveBypassesCommandDenylist,
    ToggleAgentAttribution,

    // Custom model routers
    #[cfg(feature = "local_fs")]
    OpenAddCustomRouter,

    #[cfg(feature = "local_fs")]
    SetConversationLayout(crate::util::file::external_editor::settings::OpenConversationPreference),
    ToggleCloudHandoff,
    ToggleAmpersandHandoff,
    ToggleAutoHandoffOnSleep,
    ToggleShowConversationHistory,
}

impl TypedActionView for WarpAgentPageView {
    type Action = WarpAgentPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WarpAgentPageAction::OpenUrl(url) => {
                ctx.open_url(url.as_str());
            }
            WarpAgentPageAction::ToggleGlobalAI => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings.is_any_ai_enabled.toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Global AI setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleActiveAI => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .is_active_ai_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Active AI setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleIntelligentAutosuggestions => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .intelligent_autosuggestions_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Next Command setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::TogglePromptSuggestions => {
                if !UserWorkspaces::as_ref(ctx).is_prompt_suggestions_toggleable() {
                    return;
                }
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .prompt_suggestions_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Prompt Suggestions setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleCodeSuggestions => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .code_suggestions_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Code Suggestions setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleNaturalLanguageAutosuggestions => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .natural_language_autosuggestions_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!(
                            "Failed to set value for Natural Language Autosuggestions setting: {e:?}"
                        );
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleSharedTitleGeneration => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .shared_block_title_generation_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!(
                            "Failed to set value for Shared Block Title Generation setting: {e:?}"
                        );
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleGitOperationsAutogen => {
                if !UserWorkspaces::as_ref(ctx).is_git_operations_ai_enabled() {
                    return;
                }
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .git_operations_autogen_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Git Operations Autogen setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAIInputAutoDetection => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .ai_autodetection_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Input Auto-detection: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleNLDInTerminal => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .nld_in_terminal_enabled_internal
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for NLD in Terminal: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::ToggleUseAgentToolbar => {
                match AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .should_render_use_agent_footer_for_user_commands
                        .toggle_and_save_value(ctx)
                }) {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Use Agent Footer setting: {e:?}");
                    }
                }
                ctx.notify();
            }
            WarpAgentPageAction::HyperlinkClick(hyperlink) => {
                ctx.notify();
                ctx.open_url(&hyperlink.url);
            }
            WarpAgentPageAction::ToggleShowInputHintText => {
                InputSettings::handle(ctx).update(ctx, |input_settings, ctx| {
                    report_if_error!(input_settings.show_hint_text.toggle_and_save_value(ctx));
                });
            }
            WarpAgentPageAction::ToggleAiCommandSearchHashTrigger => {
                InputSettings::handle(ctx).update(ctx, |input_settings, ctx| {
                    report_if_error!(
                        input_settings
                            .enable_ai_command_search_hash_trigger
                            .toggle_and_save_value(ctx)
                    );
                });
            }
            WarpAgentPageAction::ToggleShowAgentTips => {
                InputSettings::handle(ctx).update(ctx, |input_settings, ctx| match input_settings
                    .show_agent_tips
                    .toggle_and_save_value(ctx)
                {
                    Ok(_new_value) => {}
                    Err(e) => {
                        log::warn!("Failed to set value for Show Agent Tips setting: {e:?}");
                    }
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleShowOzUpdatesInZeroState => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .should_show_oz_updates_in_zero_state
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::SetThinkingDisplayMode(mode) => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.thinking_display_mode.set_value(*mode, ctx));
                });
                ctx.notify();
            }
            WarpAgentPageAction::SetOrchestrationMessageDisplayMode(mode) => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .orchestration_message_display_mode
                            .set_value(*mode, ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::SetPromptSubmissionMode(mode) => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .default_prompt_submission_mode
                            .set_value(*mode, ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::SetLongRunningCommandSubmissionMode(mode) => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .long_running_command_submission_mode
                            .set_value(*mode, ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::SignupAnonymousUser => {
                ctx.emit(WarpAgentPageEvent::SignupAnonymousUser);
            }
            WarpAgentPageAction::ToggleAwsBedrockAutoLogin => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.aws_bedrock_auto_login.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAwsBedrockCredentialsEnabled => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .aws_bedrock_credentials_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::RefreshAwsBedrockCredentials => {
                #[cfg(not(target_family = "wasm"))]
                ApiKeyManager::handle(ctx).update(ctx, |manager, ctx| {
                    drop(refresh_aws_credentials(manager, ctx));
                });
                ctx.notify();
            }
            WarpAgentPageAction::RefreshGeminiEnterpriseCredentials => {
                #[cfg(not(target_family = "wasm"))]
                ApiKeyManager::handle(ctx).update(ctx, |manager, ctx| {
                    force_refresh_geap_credentials(manager, ctx);
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleGeminiEnterpriseCredentialsEnabled => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .gemini_enterprise_credentials_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleCloudAgentComputerUse => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .cloud_agent_computer_use_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleFileBasedMcp => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.file_based_mcp_enabled.toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleIncludeAgentCommandsInHistory => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .include_agent_commands_in_history
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAutoApproveBypassesCommandDenylist => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .auto_approve_bypasses_command_denylist
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            #[cfg(feature = "local_fs")]
            WarpAgentPageAction::SetConversationLayout(layout) => {
                crate::util::file::external_editor::EditorSettings::handle(ctx).update(
                    ctx,
                    |settings, ctx| {
                        report_if_error!(
                            settings
                                .open_conversation_layout_preference
                                .set_value(*layout, ctx)
                        );
                    },
                );
                ctx.notify();
            }
            WarpAgentPageAction::ToggleShowConversationHistory => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .show_conversation_history
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            #[cfg(feature = "local_fs")]
            WarpAgentPageAction::OpenAddCustomRouter => {
                ctx.emit(WarpAgentPageEvent::OpenCustomRouterEditor(None));
            }
            WarpAgentPageAction::ToggleCloudHandoff => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .should_force_disable_cloud_handoff
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAmpersandHandoff => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .should_force_disable_ampersand_handoff
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAutoHandoffOnSleep => {
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .auto_handoff_on_sleep_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            WarpAgentPageAction::ToggleAgentAttribution => {
                // The updated value syncs to warp-server automatically via
                // `CloudPreferencesSyncer` as a `JsonPreference` GSO keyed
                // `Global_AgentAttributionEnabled`; no bespoke server call needed.
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(
                        settings
                            .agent_attribution_enabled
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
        }
    }
}

impl SettingsPageMeta for WarpAgentPageView {
    fn section() -> SettingsSection {
        SettingsSection::WarpAgent
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        FeatureFlag::AgentMode.is_enabled()
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<WarpAgentPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<WarpAgentPageView>) -> Self {
        SettingsPageViewHandle::WarpAgent(view_handle)
    }
}

/// The page title's trailing widget: the global master switch for all AI features.
fn render_global_ai_toggle(
    switch_state: &SwitchStateHandle,
    appearance: &Appearance,
    app: &AppContext,
) -> Box<dyn Element> {
    let ui_builder = appearance.ui_builder();
    let is_ai_disabled_due_to_remote_session_org_policy =
        AISettings::as_ref(app).is_ai_disabled_due_to_remote_session_org_policy(app);

    let mut row = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);

    if is_ai_disabled_due_to_remote_session_org_policy {
        row.add_child(
            Container::new(
                ConstrainedBox::new(
                    Container::new(
                        Text::new("Your organization disallows AI when the active pane contains content from a remote session", appearance.ui_font_family(), 12.)
                            .with_color(appearance.theme().ui_warning_color())
                            .finish()
                    )
                    .with_padding_left(8.)
                    .with_padding_right(8.)
                    .finish()
                )
                .with_max_width(400.)
                .finish()
            )
            .with_margin_right(16.)
            .finish()
        );
    }

    row.add_child(
        Container::new(
            ui_builder
                .switch(switch_state.clone())
                .check(AISettings::as_ref(app).is_any_ai_enabled(app))
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::ToggleGlobalAI);
                })
                .finish(),
        )
        .with_padding_right(TOGGLE_BUTTON_RIGHT_PADDING)
        .finish(),
    );

    row.finish()
}

fn is_next_command_toggleable(app: &AppContext) -> bool {
    UserWorkspaces::as_ref(app).is_next_command_enabled()
        && AISettings::as_ref(app)
            .intelligent_autosuggestions_enabled_internal
            .is_supported_on_current_platform()
}

fn is_prompt_suggestions_toggleable(app: &AppContext) -> bool {
    UserWorkspaces::as_ref(app).is_prompt_suggestions_toggleable()
        && AISettings::as_ref(app)
            .prompt_suggestions_enabled_internal
            .is_supported_on_current_platform()
}

fn is_suggested_code_banners_toggleable(app: &AppContext) -> bool {
    (is_prompt_suggestions_toggleable(app)
        || UserWorkspaces::as_ref(app).is_code_suggestions_toggleable())
        && AISettings::as_ref(app)
            .code_suggestions_enabled_internal
            .is_supported_on_current_platform()
}

fn is_natural_language_autosuggestions_toggleable(app: &AppContext) -> bool {
    FeatureFlag::PredictAMQueries.is_enabled()
        && AISettings::as_ref(app)
            .natural_language_autosuggestions_enabled_internal
            .is_supported_on_current_platform()
}

// TODO: Check if the user's enterprise billing policy allows toggling this feature.
fn is_shared_block_title_generation_toggleable(
    view_handle: &WeakViewHandle<WarpAgentPageView>,
    app: &AppContext,
) -> bool {
    FeatureFlag::SharedBlockTitleGeneration.is_enabled()
        && AISettings::as_ref(app)
            .shared_block_title_generation_enabled_internal
            .is_supported_on_current_platform()
        && (!UserWorkspaces::as_ref(app)
            .team_for_view_handle(view_handle, app)
            .is_some_and(|team| team.billing_metadata.customer_type == CustomerType::Enterprise)
            // Override the enterprise check for dogfood builds, as our dogfood team
            // is an enterprise team.
            || cfg!(debug_assertions))
}

fn is_git_operations_autogen_toggleable(app: &AppContext) -> bool {
    FeatureFlag::GitOperationsInCodeReview.is_enabled()
        && AISettings::as_ref(app)
            .git_operations_autogen_enabled_internal
            .is_supported_on_current_platform()
        && UserWorkspaces::as_ref(app).is_git_operations_ai_enabled()
}

/// The "Active AI" category's header trailing widget: the master switch for all of
/// the category's child settings.
fn render_active_ai_toggle(toggle: &SwitchStateHandle, app: &AppContext) -> Box<dyn Element> {
    let ai_settings = AISettings::as_ref(app);
    let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);
    Container::new(render_ai_feature_switch(
        toggle.clone(),
        *ai_settings.is_active_ai_enabled_internal,
        is_any_ai_enabled,
        WarpAgentPageAction::ToggleActiveAI,
        app,
    ))
    .with_padding_right(TOGGLE_BUTTON_RIGHT_PADDING)
    .finish()
}

#[derive(Default)]
struct NextCommandWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for NextCommandWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. next command suggestions"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_next_command_toggleable(app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);

        Flex::column()
            .with_child(
                render_ai_setting_toggle::<IntelligentAutosuggestionsEnabled>(
                    "Next Command",
                    WarpAgentPageAction::ToggleIntelligentAutosuggestions,
                    *ai_settings.intelligent_autosuggestions_enabled_internal,
                    is_toggleable,
                    self.toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
            )
            .with_child(render_ai_setting_description(
                NEXT_COMMAND_DESCRIPTION,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct PromptSuggestionsWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for PromptSuggestionsWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. prompt suggestions"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_prompt_suggestions_toggleable(app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);
        Flex::column()
            .with_child(
                render_ai_setting_toggle::<AgentModeQuerySuggestionsEnabled>(
                    "Prompt Suggestions",
                    WarpAgentPageAction::TogglePromptSuggestions,
                    *ai_settings.prompt_suggestions_enabled_internal,
                    is_toggleable,
                    self.toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
            )
            .with_child(render_ai_setting_description(
                PROMPT_SUGGESTIONS_DESCRIPTION,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct SuggestedCodeBannersWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for SuggestedCodeBannersWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. code diffs suggested banners"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_suggested_code_banners_toggleable(app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);
        Flex::column()
            .with_child(
                render_ai_setting_toggle::<AgentModeQuerySuggestionsEnabled>(
                    "Suggested Code Banners",
                    WarpAgentPageAction::ToggleCodeSuggestions,
                    *ai_settings.code_suggestions_enabled_internal,
                    is_toggleable,
                    self.toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
            )
            .with_child(render_ai_setting_description(
                SUGGESTED_CODE_BANNERS_DESCRIPTION,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct NaturalLanguageAutosuggestionsWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for NaturalLanguageAutosuggestionsWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. natural language autosuggestions passive"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_natural_language_autosuggestions_toggleable(app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);
        Flex::column()
            .with_child(render_ai_setting_toggle::<
                NaturalLanguageAutosuggestionsEnabled,
            >(
                "Natural Language Autosuggestions",
                WarpAgentPageAction::ToggleNaturalLanguageAutosuggestions,
                *ai_settings.natural_language_autosuggestions_enabled_internal,
                is_toggleable,
                self.toggle.clone(),
                &view.local_only_icon_tooltip_states,
                app,
            ))
            .with_child(render_ai_setting_description(
                NATURAL_LANGUAGE_AUTOSUGGESTIONS,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

struct SharedBlockTitleGenerationWidget {
    toggle: SwitchStateHandle,
    view_handle: WeakViewHandle<WarpAgentPageView>,
}

impl SharedBlockTitleGenerationWidget {
    fn new(ctx: &ViewContext<WarpAgentPageView>) -> Self {
        Self {
            toggle: Default::default(),
            view_handle: ctx.handle(),
        }
    }
}

impl SettingsWidget for SharedBlockTitleGenerationWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. shared block title generation"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_shared_block_title_generation_toggleable(&self.view_handle, app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);
        Flex::column()
            .with_child(
                render_ai_setting_toggle::<SharedBlockTitleGenerationEnabled>(
                    "Shared Block Title Generation",
                    WarpAgentPageAction::ToggleSharedTitleGeneration,
                    *ai_settings.shared_block_title_generation_enabled_internal,
                    is_toggleable,
                    self.toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
            )
            .with_child(render_ai_setting_description(
                SHARED_BLOCK_TITLE_GENERATION_DESCRIPTION,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct GitOperationsAutogenWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for GitOperationsAutogenWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "active ai a.i. unit tests commit pull request pr git code review autogen generate"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        is_git_operations_autogen_toggleable(app)
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_active_ai_enabled(app);
        Flex::column()
            .with_child(render_ai_setting_toggle::<GitOperationsAutogenEnabled>(
                "Commit & Pull Request Generation",
                WarpAgentPageAction::ToggleGitOperationsAutogen,
                *ai_settings.git_operations_autogen_enabled_internal,
                is_toggleable,
                self.toggle.clone(),
                &view.local_only_icon_tooltip_states,
                app,
            ))
            .with_child(render_ai_setting_description(
                GIT_OPERATIONS_AUTOGEN_DESCRIPTION,
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct NaturalLanguageDetectionWidget {
    incorrect_autodetection_highlight_index: HighlightedHyperlink,
    autodetection_toggle: SwitchStateHandle,
    nld_in_terminal_toggle: SwitchStateHandle,
}

impl SettingsWidget for NaturalLanguageDetectionWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai natural language detection autodetection prompt terminal command denylist permissions"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        Self::render_natural_language_detection_section(
            self.incorrect_autodetection_highlight_index.clone(),
            self.autodetection_toggle.clone(),
            self.nld_in_terminal_toggle.clone(),
            view,
            ai_settings,
            appearance,
            app,
        )
    }
}

#[derive(Default)]
struct ShowInputHintTextWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for ShowInputHintTextWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai input show hint text"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_ai_setting_toggle::<ShowHintText>(
            "Show input hint text",
            WarpAgentPageAction::ToggleShowInputHintText,
            *InputSettings::as_ref(app).show_hint_text,
            is_any_ai_enabled,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct AiCommandSearchHashTriggerWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for AiCommandSearchHashTriggerWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "# hash pound trigger ai command search shorthand shell comment"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_ai_setting_toggle::<EnableAiCommandSearchHashTrigger>(
            "Enable '#' trigger for AI Command Search",
            WarpAgentPageAction::ToggleAiCommandSearchHashTrigger,
            *InputSettings::as_ref(app).enable_ai_command_search_hash_trigger,
            is_any_ai_enabled,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct ShowAgentTipsWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for ShowAgentTipsWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai show agent tips"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::AgentTips.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_ai_setting_toggle::<ShowAgentTips>(
            "Show agent tips",
            WarpAgentPageAction::ToggleShowAgentTips,
            *InputSettings::as_ref(app).show_agent_tips,
            is_any_ai_enabled,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct IncludeAgentCommandsInHistoryWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for IncludeAgentCommandsInHistoryWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai include agent-executed commands in history shell"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);
        render_ai_setting_toggle::<IncludeAgentCommandsInHistory>(
            "Include agent-executed commands in history",
            WarpAgentPageAction::ToggleIncludeAgentCommandsInHistory,
            *ai_settings.include_agent_commands_in_history,
            is_any_ai_enabled,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct AutoApproveBypassesCommandDenylistWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for AutoApproveBypassesCommandDenylistWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai auto-approve fast forward bypass denylist permissions"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);
        Flex::column()
            .with_child(render_ai_setting_toggle::<AutoApproveBypassesCommandDenylist>(
                "Allow auto-approve to bypass command denylist",
                WarpAgentPageAction::ToggleAutoApproveBypassesCommandDenylist,
                *ai_settings.auto_approve_bypasses_command_denylist,
                is_any_ai_enabled,
                self.toggle.clone(),
                &view.local_only_icon_tooltip_states,
                app,
            ))
            .with_child(render_ai_setting_description(
                "When enabled, fast forward and auto-approve run denylisted commands without asking for confirmation.",
                is_any_ai_enabled,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct PromptSubmissionModeWidget;

impl SettingsWidget for PromptSubmissionModeWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz agent ai default prompt submission mode queue interrupt auto-queue long-running long running lrc"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::QueueSlashCommand.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);

        let mut column = Flex::column().with_child(render_dropdown_item(
            appearance,
            "Default prompt submission mode",
            Some(
                "What happens when you submit a new prompt while the agent is still \
                 responding. You can override this per conversation using the auto-queue \
                 toggle.",
            ),
            None,
            LocalOnlyIconState::for_setting(
                PromptSubmissionMode::storage_key(),
                PromptSubmissionMode::sync_to_cloud(),
                &mut view.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            (!is_any_ai_enabled).then(|| appearance.theme().disabled_ui_text_color()),
            &view.default_prompt_submission_mode_dropdown,
        ));

        // Only meaningful in Interrupt mode: with Queue selected, prompts already
        // queue until the end of the full response, so the LRC mode is hidden.
        if ai_settings.default_prompt_submission_mode == PromptSubmissionMode::Interrupt {
            column.add_child(
                Container::new(render_dropdown_item(
                    appearance,
                    "Default long-running command submission mode",
                    Some(
                        "What happens when you submit a prompt while an agent is driving an \
                         agent-requested long-running command. Queued prompts are sent to the \
                         agent when the command finishes.",
                    ),
                    None,
                    LocalOnlyIconState::for_setting(
                        LongRunningCommandSubmissionMode::storage_key(),
                        LongRunningCommandSubmissionMode::sync_to_cloud(),
                        &mut view.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    (!is_any_ai_enabled).then(|| appearance.theme().disabled_ui_text_color()),
                    &view.lrc_submission_mode_dropdown,
                ))
                .with_margin_top(styles::DESCRIPTION_MARGIN_BOTTOM)
                .finish(),
            );
        }

        column.finish()
    }
}

impl NaturalLanguageDetectionWidget {
    fn render_natural_language_detection_section(
        incorrect_autodetection_highlight_index: HighlightedHyperlink,
        autodetection_toggle: SwitchStateHandle,
        nld_in_terminal_toggle: SwitchStateHandle,
        view: &WarpAgentPageView,
        ai_settings: &AISettings,
        appearance: &Appearance,
        app: &warpui::AppContext,
    ) -> Box<dyn warpui::Element> {
        let is_toggleable = ai_settings.is_any_ai_enabled(app);
        let is_nld_enabled = *ai_settings.ai_autodetection_enabled_internal.value();

        let autodetection_denylist_input_field = appearance
            .ui_builder()
            .text_input(view.autodetection_denylist_editor.clone())
            .with_style(UiComponentStyles {
                width: Some(280.),
                padding: Some(Coords {
                    top: 4.,
                    bottom: 4.,
                    left: 6.,
                    right: 6.,
                }),
                background: Some(appearance.theme().surface_2().into()),
                ..Default::default()
            })
            .build()
            .finish();

        let mut section = Flex::column();

        if FeatureFlag::AgentView.is_enabled() {
            static AUTODETECTION_DESCRIPTION_FRAGMENTS: LazyLock<Vec<FormattedTextFragment>> =
                LazyLock::new(|| {
                    vec![
                        FormattedTextFragment::plain_text("Encountered an incorrect detection? "),
                        FormattedTextFragment::hyperlink(
                            "Let us know",
                            "https://warpdotdev.typeform.com/to/offrTIpq",
                        ),
                    ]
                });

            section.add_children([
                render_ai_setting_toggle::<NLDInTerminalEnabled>(
                    "Autodetect agent prompts in terminal input",
                    WarpAgentPageAction::ToggleNLDInTerminal,
                    ai_settings.is_nld_in_terminal_enabled(app),
                    is_toggleable,
                    nld_in_terminal_toggle,
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
                render_ai_setting_toggle::<AIAutoDetectionEnabled>(
                    "Autodetect terminal commands in agent input",
                    WarpAgentPageAction::ToggleAIInputAutoDetection,
                    is_nld_enabled,
                    is_toggleable,
                    autodetection_toggle,
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
                Container::new(
                    FormattedTextElement::new(
                        FormattedText::new([FormattedTextLine::Line(
                            (*AUTODETECTION_DESCRIPTION_FRAGMENTS).clone(),
                        )]),
                        CONTENT_FONT_SIZE,
                        appearance.ui_font_family(),
                        appearance.ui_font_family(),
                        styles::description_font_color(is_toggleable, app).into(),
                        incorrect_autodetection_highlight_index,
                    )
                    .with_hyperlink_font_color(appearance.theme().accent().into_solid())
                    .register_default_click_handlers(|url, ctx, _| {
                        ctx.dispatch_typed_action(WarpAgentPageAction::HyperlinkClick(url));
                    })
                    .finish(),
                )
                .with_margin_top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                .with_margin_bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                .with_margin_right(styles::TOGGLE_WIDTH_MARGIN)
                .finish(),
            ])
        } else {
            static NATURAL_LANGUAGE_DETECTION_DESCRIPTION_FRAGMENTS: LazyLock<
                Vec<FormattedTextFragment>,
            > = LazyLock::new(|| {
                vec![
                    FormattedTextFragment::plain_text(
                        "Enabling natural language detection will detect when natural language is written in the terminal input, and then automatically switch to Agent Mode for AI queries.",
                    ),
                    FormattedTextFragment::plain_text(
                        " Encountered an incorrect input detection? ",
                    ),
                    FormattedTextFragment::hyperlink(
                        "Let us know",
                        "https://warpdotdev.typeform.com/to/offrTIpq",
                    ),
                ]
            });

            section.add_children([
                render_ai_setting_toggle::<AIAutoDetectionEnabled>(
                    "Natural language detection",
                    WarpAgentPageAction::ToggleAIInputAutoDetection,
                    is_nld_enabled,
                    is_toggleable,
                    autodetection_toggle,
                    &view.local_only_icon_tooltip_states,
                    app,
                ),
                Container::new(
                    FormattedTextElement::new(
                        FormattedText::new([FormattedTextLine::Line(
                            (*NATURAL_LANGUAGE_DETECTION_DESCRIPTION_FRAGMENTS).clone(),
                        )]),
                        CONTENT_FONT_SIZE,
                        appearance.ui_font_family(),
                        appearance.ui_font_family(),
                        styles::description_font_color(is_toggleable, app).into(),
                        incorrect_autodetection_highlight_index,
                    )
                    .with_hyperlink_font_color(appearance.theme().accent().into_solid())
                    .register_default_click_handlers(|url, ctx, _| {
                        ctx.dispatch_typed_action(WarpAgentPageAction::HyperlinkClick(url));
                    })
                    .finish(),
                )
                .with_margin_top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                .with_margin_bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                .with_margin_right(styles::TOGGLE_WIDTH_MARGIN)
                .finish(),
            ]);
        }

        section
            .with_child(render_ai_setting_label::<AICommandDenylist>(
                "Natural language denylist".to_owned(),
                is_toggleable,
                &view.local_only_icon_tooltip_states,
                app,
            ))
            .with_child(render_ai_setting_description(
                "Commands listed here will never trigger natural language detection.",
                is_toggleable,
                app,
            ))
            .with_child(
                Container::new(autodetection_denylist_input_field)
                    .with_margin_bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                    .finish(),
            )
            .finish()
    }
}

struct OtherAIWidget;

impl OtherAIWidget {
    fn create_thinking_display_mode_dropdown(
        ctx: &mut ViewContext<WarpAgentPageView>,
    ) -> ViewHandle<Dropdown<WarpAgentPageAction>> {
        let items: Vec<DropdownItem<WarpAgentPageAction>> = ThinkingDisplayMode::iter()
            .map(|mode| {
                DropdownItem::new(
                    mode.display_name(),
                    WarpAgentPageAction::SetThinkingDisplayMode(mode),
                )
            })
            .collect();

        ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(AI_SETTINGS_DROPDOWN_WIDTH);
            dropdown.set_menu_width(AI_SETTINGS_DROPDOWN_WIDTH, ctx);
            dropdown.set_menu_max_height(AI_SETTINGS_DROPDOWN_MAX_HEIGHT, ctx);
            dropdown.add_items(items, ctx);
            dropdown
        })
    }

    fn create_default_prompt_submission_mode_dropdown(
        ctx: &mut ViewContext<WarpAgentPageView>,
    ) -> ViewHandle<Dropdown<WarpAgentPageAction>> {
        let items: Vec<DropdownItem<WarpAgentPageAction>> = PromptSubmissionMode::iter()
            .map(|mode| {
                DropdownItem::new(
                    mode.display_name(),
                    WarpAgentPageAction::SetPromptSubmissionMode(mode),
                )
            })
            .collect();

        ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(AI_SETTINGS_DROPDOWN_WIDTH);
            dropdown.set_menu_width(AI_SETTINGS_DROPDOWN_WIDTH, ctx);
            dropdown.set_menu_max_height(AI_SETTINGS_DROPDOWN_MAX_HEIGHT, ctx);
            dropdown.add_items(items, ctx);
            dropdown
        })
    }

    fn create_lrc_submission_mode_dropdown(
        ctx: &mut ViewContext<WarpAgentPageView>,
    ) -> ViewHandle<Dropdown<WarpAgentPageAction>> {
        let items: Vec<DropdownItem<WarpAgentPageAction>> =
            LongRunningCommandSubmissionMode::iter()
                .map(|mode| {
                    DropdownItem::new(
                        mode.display_name(),
                        WarpAgentPageAction::SetLongRunningCommandSubmissionMode(mode),
                    )
                })
                .collect();

        ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(AI_SETTINGS_DROPDOWN_WIDTH);
            dropdown.set_menu_width(AI_SETTINGS_DROPDOWN_WIDTH, ctx);
            dropdown.set_menu_max_height(AI_SETTINGS_DROPDOWN_MAX_HEIGHT, ctx);
            dropdown.add_items(items, ctx);
            dropdown
        })
    }

    fn create_orchestration_message_display_mode_dropdown(
        ctx: &mut ViewContext<WarpAgentPageView>,
    ) -> ViewHandle<Dropdown<WarpAgentPageAction>> {
        let items: Vec<DropdownItem<WarpAgentPageAction>> = OrchestrationMessageDisplayMode::iter()
            .map(|mode| {
                DropdownItem::new(
                    mode.display_name(),
                    WarpAgentPageAction::SetOrchestrationMessageDisplayMode(mode),
                )
            })
            .collect();

        ctx.add_typed_action_view(|ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(AI_SETTINGS_DROPDOWN_WIDTH);
            dropdown.set_menu_width(AI_SETTINGS_DROPDOWN_WIDTH, ctx);
            dropdown.set_menu_max_height(AI_SETTINGS_DROPDOWN_MAX_HEIGHT, ctx);
            dropdown.add_items(items, ctx);
            dropdown
        })
    }
}

#[derive(Default)]
struct ShowOzUpdatesInZeroStateWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for ShowOzUpdatesInZeroStateWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other oz updates zero state empty changelog new conversation agent what's new"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::AgentView.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_any_ai_enabled(app);
        render_ai_setting_toggle::<ShouldShowOzUpdatesInZeroState>(
            "Show Warp Agent changelog in new conversation view",
            WarpAgentPageAction::ToggleShowOzUpdatesInZeroState,
            *ai_settings.should_show_oz_updates_in_zero_state,
            is_toggleable,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct UseAgentFooterWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for UseAgentFooterWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other use agent footer full terminal use long running commands"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::AgentView.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_any_ai_enabled(app);

        Flex::column()
            .with_child(render_ai_setting_toggle::<
                ShouldRenderUseAgentToolbarForUserCommands,
            >(
                "Show \"Use Agent\" footer",
                WarpAgentPageAction::ToggleUseAgentToolbar,
                *ai_settings.should_render_use_agent_footer_for_user_commands,
                is_toggleable,
                self.toggle.clone(),
                &view.local_only_icon_tooltip_states,
                app,
            ))
            .with_child(render_ai_setting_description(
                "Shows hint to use the \"Full Terminal Use\"-enabled agent in long running commands.",
                is_toggleable,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct AgentToolbarLayoutEditorWidget;

impl SettingsWidget for AgentToolbarLayoutEditorWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other agent toolbar layout chip chips rearrange re-arrange"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        FeatureFlag::AgentView.is_enabled()
            && FeatureFlag::AgentToolbarEditor.is_enabled()
            && AISettings::as_ref(app).is_any_ai_enabled(app)
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        render_toolbar_layout_editor(&view.agent_toolbar_inline_editor, appearance)
    }
}

#[derive(Default)]
struct ShowConversationHistoryWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for ShowConversationHistoryWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other conversation history tools panel collapse expand hide"
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_toggleable = ai_settings.is_any_ai_enabled(app);
        render_ai_setting_toggle::<ShowConversationHistory>(
            "Show conversation history in tools panel",
            WarpAgentPageAction::ToggleShowConversationHistory,
            *ai_settings.show_conversation_history,
            is_toggleable,
            self.toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        )
    }
}

#[derive(Default)]
struct ThinkingDisplayModeWidget;

impl SettingsWidget for ThinkingDisplayModeWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other agent thinking display reasoning collapse never show expanded"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_dropdown_item(
            appearance,
            "Agent thinking display",
            Some("Controls how reasoning/thinking traces are displayed."),
            None,
            LocalOnlyIconState::for_setting(
                ThinkingDisplayMode::storage_key(),
                ThinkingDisplayMode::sync_to_cloud(),
                &mut view.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            (!is_any_ai_enabled).then(|| appearance.theme().disabled_ui_text_color()),
            &view.thinking_display_mode_dropdown,
        )
    }
}

#[derive(Default)]
struct OrchestrationMessageDisplayModeWidget;

impl SettingsWidget for OrchestrationMessageDisplayModeWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other orchestration messages child agents collapse expand hide display"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_dropdown_item(
            appearance,
            "Orchestration message display",
            Some("Controls whether orchestration messages stay expanded."),
            None,
            LocalOnlyIconState::for_setting(
                OrchestrationMessageDisplayMode::storage_key(),
                OrchestrationMessageDisplayMode::sync_to_cloud(),
                &mut view.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            (!is_any_ai_enabled).then(|| appearance.theme().disabled_ui_text_color()),
            &view.orchestration_message_display_mode_dropdown,
        )
    }
}

// TODO: OpenConversationLayoutPreference should not depend on local_fs, but it lives under the
// external editor settings which does require local_fs. It was a mistake to put it there, but now
// we keep it there for backward compatibility.
#[cfg(feature = "local_fs")]
#[derive(Default)]
struct ConversationLayoutPreferenceWidget;

#[cfg(feature = "local_fs")]
impl SettingsWidget for ConversationLayoutPreferenceWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "other preferred layout opening existing agent conversations new tab split pane"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        use crate::util::file::external_editor::settings::OpenConversationLayoutPreference;

        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        render_dropdown_item(
            appearance,
            "Preferred layout when opening existing agent conversations",
            None,
            None,
            LocalOnlyIconState::for_setting(
                OpenConversationLayoutPreference::storage_key(),
                OpenConversationLayoutPreference::sync_to_cloud(),
                &mut view.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            (!is_any_ai_enabled).then(|| appearance.theme().disabled_ui_text_color()),
            &view.conversation_layout_dropdown,
        )
    }
}

/// The presentation state of the agent attribution toggle, derived from the
/// org-level [`AdminEnablementSetting`], the user's stored preference, and
/// whether AI is globally enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentAttributionToggleState {
    /// Whether the toggle is rendered in the checked state.
    pub(crate) is_enabled: bool,
    /// Whether the org has forced the value (locking the toggle with a tooltip).
    pub(crate) is_forced_by_org: bool,
    /// Whether the toggle should be rendered as non-interactive overall
    /// (forced by the org, or AI globally disabled).
    pub(crate) is_disabled: bool,
}

/// Derive the toggle state from its three inputs.
pub(crate) fn derive_agent_attribution_toggle_state(
    org_setting: &AdminEnablementSetting,
    user_pref: bool,
    is_any_ai_enabled: bool,
) -> AgentAttributionToggleState {
    let is_forced_by_org = match org_setting {
        AdminEnablementSetting::Enable | AdminEnablementSetting::Disable => true,
        AdminEnablementSetting::RespectUserSetting => false,
    };
    let is_enabled = match org_setting {
        AdminEnablementSetting::Enable => true,
        AdminEnablementSetting::Disable => false,
        AdminEnablementSetting::RespectUserSetting => user_pref,
    };
    AgentAttributionToggleState {
        is_enabled,
        is_forced_by_org,
        is_disabled: is_forced_by_org || !is_any_ai_enabled,
    }
}

#[derive(Default)]
struct AgentAttributionWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for AgentAttributionWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "agent attribution commit pull request co-author author credit oz warp"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);

        let workspaces = UserWorkspaces::as_ref(app);
        let scope = workspaces.team_context(&view.self_handle, app);
        let org_setting = workspaces.get_agent_attribution_setting(&scope);
        let state = derive_agent_attribution_toggle_state(
            &org_setting,
            *ai_settings.agent_attribution_enabled,
            is_any_ai_enabled,
        );

        let ui_builder = appearance.ui_builder();
        let toggle = if state.is_forced_by_org {
            ui_builder
                .switch(self.toggle.clone())
                .check(state.is_enabled)
                .with_tooltip(TooltipConfig {
                    text: "This option is enforced by your organization's settings and cannot be customized.".to_string(),
                    styles: ui_builder.default_tool_tip_styles(),
                })
                .disable()
                .build()
                .finish()
        } else if !is_any_ai_enabled {
            ui_builder
                .switch(self.toggle.clone())
                .check(state.is_enabled)
                .with_disabled(true)
                .build()
                .finish()
        } else {
            ui_builder
                .switch(self.toggle.clone())
                .check(state.is_enabled)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::ToggleAgentAttribution);
                })
                .finish()
        };

        let toggle_row = build_toggle_element(
            render_body_item_label::<WarpAgentPageAction>(
                "Enable agent attribution".to_string(),
                Some(styles::header_font_color(!state.is_disabled, app)),
                None,
                LocalOnlyIconState::Hidden,
                ToggleState::Enabled,
                appearance,
            ),
            toggle,
            appearance,
            None,
        );

        Flex::column()
            .with_child(toggle_row)
            .with_child(render_ai_setting_description(
                "Warp Agent can add attribution to commit messages and pull requests it creates",
                !state.is_disabled,
                app,
            ))
            .finish()
    }
}

#[cfg(test)]
#[path = "warp_agent_page_tests.rs"]
mod tests;

#[derive(Default)]
struct CloudAgentComputerUseWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for CloudAgentComputerUseWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "oz cloud agent computer use orchestration multi-agent"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        use crate::ai::execution_profiles::{
            CloudAgentComputerUseState, resolve_cloud_agent_computer_use_state,
        };

        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);

        // Determine toggle state based on workspace autonomy setting and user preference
        let CloudAgentComputerUseState {
            enabled: is_checked,
            is_forced_by_org,
        } = {
            let scope = UserWorkspaces::as_ref(app).team_context(&view.self_handle, app);
            resolve_cloud_agent_computer_use_state(&scope, app)
        };

        // Toggle is disabled if forced by org settings OR if AI is globally disabled
        let is_disabled = is_forced_by_org || !is_any_ai_enabled;

        let ui_builder = appearance.ui_builder();
        let toggle = if is_forced_by_org {
            // Disabled by organization setting - show tooltip on hover
            ui_builder
                .switch(self.toggle.clone())
                .check(is_checked)
                .with_tooltip(TooltipConfig {
                    text: "This option is enforced by your organization's settings and cannot be customized.".to_string(),
                    styles: ui_builder.default_tool_tip_styles(),
                })
                .disable()
                .build()
                .finish()
        } else if !is_any_ai_enabled {
            // Disabled because AI is off globally - no tooltip needed
            ui_builder
                .switch(self.toggle.clone())
                .check(is_checked)
                .with_disabled(true)
                .build()
                .finish()
        } else {
            // Enabled - allow toggling
            ui_builder
                .switch(self.toggle.clone())
                .check(is_checked)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::ToggleCloudAgentComputerUse);
                })
                .finish()
        };

        let toggle_row = build_toggle_element(
            render_body_item_label::<WarpAgentPageAction>(
                "Computer use in Cloud Agents".to_string(),
                Some(styles::header_font_color(!is_disabled, app)),
                None,
                LocalOnlyIconState::Hidden,
                ToggleState::Enabled,
                appearance,
            ),
            toggle,
            appearance,
            None,
        );

        Flex::column()
            .with_child(toggle_row)
            .with_child(render_ai_setting_description(
                "Enable computer use in cloud agent conversations started from the Warp app.",
                !is_disabled,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct CloudHandoffWidget {
    handoff_toggle: SwitchStateHandle,
}

impl SettingsWidget for CloudHandoffWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "cloud handoff move to cloud local"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::OzHandoff.is_enabled() && FeatureFlag::HandoffLocalCloud.is_enabled()
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        use crate::settings::PrivacySettings;

        let ai_settings = AISettings::as_ref(app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);

        let privacy = PrivacySettings::as_ref(app);
        let cloud_convos_off = !privacy.is_cloud_conversation_storage_enabled
            || matches!(
                UserWorkspaces::as_ref(app).get_cloud_conversation_storage_enablement_setting(),
                AdminEnablementSetting::Disable
            );
        let is_force_disabled = !is_any_ai_enabled || cloud_convos_off;

        let tooltip_text = if cloud_convos_off {
            "Cloud handoff requires cloud conversations to be enabled."
        } else {
            ""
        };

        let ui_builder = appearance.ui_builder();

        let handoff_toggle = if is_force_disabled {
            let mut builder = ui_builder.switch(self.handoff_toggle.clone()).check(false);
            if !tooltip_text.is_empty() {
                builder = builder.with_tooltip(TooltipConfig {
                    text: tooltip_text.to_string(),
                    styles: ui_builder.default_tool_tip_styles(),
                });
            }
            builder.disable().build().finish()
        } else {
            ui_builder
                .switch(self.handoff_toggle.clone())
                .check(!*ai_settings.should_force_disable_cloud_handoff)
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::ToggleCloudHandoff);
                })
                .finish()
        };

        let handoff_row = build_toggle_element(
            render_body_item_label::<WarpAgentPageAction>(
                "Cloud handoff".to_string(),
                Some(styles::header_font_color(!is_force_disabled, app)),
                None,
                LocalOnlyIconState::Hidden,
                ToggleState::Enabled,
                appearance,
            ),
            handoff_toggle,
            appearance,
            None,
        );

        Flex::column()
            .with_child(handoff_row)
            .with_child(render_ai_setting_description(
                "Hand off local agent conversations to a cloud agent.",
                !is_force_disabled,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct AutoHandoffOnSleepWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for AutoHandoffOnSleepWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "cloud handoff auto sleep before macos"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        FeatureFlag::OzHandoff.is_enabled()
            && FeatureFlag::HandoffLocalCloud.is_enabled()
            && AISettings::as_ref(app).is_cloud_handoff_enabled(app)
            && AISettings::as_ref(app)
                .auto_handoff_on_sleep_enabled
                .is_supported_on_current_platform()
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let ui_builder = appearance.ui_builder();

        let auto_handoff_on_sleep_toggle = ui_builder
            .switch(self.toggle.clone())
            .check(*ai_settings.auto_handoff_on_sleep_enabled)
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WarpAgentPageAction::ToggleAutoHandoffOnSleep);
            })
            .finish();
        let auto_handoff_on_sleep_row = build_toggle_element(
            render_body_item_label::<WarpAgentPageAction>(
                "Auto-handoff before sleep".to_string(),
                Some(styles::header_font_color(true, app)),
                None,
                LocalOnlyIconState::Hidden,
                ToggleState::Enabled,
                appearance,
            ),
            auto_handoff_on_sleep_toggle,
            appearance,
            None,
        );

        Flex::column()
            .with_child(auto_handoff_on_sleep_row)
            .with_child(render_ai_setting_description(
                "When macOS is about to sleep, automatically moves the most recently focused running local Warp Agent conversation to Cloud Mode so it can keep working.",
                true,
                app,
            ))
            .finish()
    }
}

#[derive(Default)]
struct AmpersandHandoffWidget {
    toggle: SwitchStateHandle,
}

impl SettingsWidget for AmpersandHandoffWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "cloud handoff ampersand & trigger compose"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        FeatureFlag::OzHandoff.is_enabled()
            && FeatureFlag::HandoffLocalCloud.is_enabled()
            && AISettings::as_ref(app).is_cloud_handoff_enabled(app)
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let ui_builder = appearance.ui_builder();

        let ampersand_toggle = ui_builder
            .switch(self.toggle.clone())
            .check(!*ai_settings.should_force_disable_ampersand_handoff)
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WarpAgentPageAction::ToggleAmpersandHandoff);
            })
            .finish();

        let ampersand_row = build_toggle_element(
            render_body_item_label::<WarpAgentPageAction>(
                "Use & to trigger handoff".to_string(),
                Some(styles::header_font_color(true, app)),
                None,
                LocalOnlyIconState::Hidden,
                ToggleState::Enabled,
                appearance,
            ),
            ampersand_toggle,
            appearance,
            None,
        );

        Flex::column()
            .with_child(ampersand_row)
            .with_child(render_ai_setting_description(
                "Type & as the first character to enter cloud handoff compose mode.",
                true,
                app,
            ))
            .finish()
    }
}

const OPENROUTER_KEY_PLACEHOLDER: &str = "sk-or-...";

fn render_openrouter_input_row(
    appearance: &Appearance,
    label: &'static str,
    editor: ViewHandle<EditorView>,
    app: &AppContext,
) -> Box<dyn Element> {
    let editor_style = UiComponentStyles {
        padding: Some(Coords {
            top: 10.,
            bottom: 10.,
            left: 16.,
            right: 16.,
        }),
        background: Some(appearance.theme().surface_2().into()),
        ..Default::default()
    };
    let label = Text::new_inline(label, appearance.ui_font_family(), CONTENT_FONT_SIZE)
        .with_color(styles::header_font_color(true, app).into())
        .finish();
    let input = appearance
        .ui_builder()
        .text_input(editor)
        .with_style(editor_style)
        .build()
        .finish();
    Flex::column()
        .with_spacing(8.)
        .with_child(label)
        .with_child(input)
        .finish()
}

struct OpenRouterApiKeyWidget {
    editor: ViewHandle<EditorView>,
}

impl OpenRouterApiKeyWidget {
    fn new(ctx: &mut ViewContext<WarpAgentPageView>) -> Self {
        let key = ApiKeyManager::as_ref(ctx).keys().open_router.clone();
        let editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = SingleLineEditorOptions {
                is_password: true,
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(editor_text_colors(appearance)),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(OPENROUTER_KEY_PLACEHOLDER, ctx);
            if let Some(key) = &key {
                editor.set_buffer_text(key, ctx);
            }
            editor
        });
        ctx.subscribe_to_view(&editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let key = buffer_text.is_empty().not().then_some(buffer_text);
                ApiKeyManager::handle(ctx).update(ctx, |manager, ctx| {
                    report_if_error!(manager.set_open_router_key(key, ctx));
                });
            }
        });
        Self { editor }
    }
}

impl SettingsWidget for OpenRouterApiKeyWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "openrouter api key bring your own byo code review commit message"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_openrouter_input_row(appearance, "OpenRouter API key", self.editor.clone(), app)
    }
}

struct OpenRouterModelWidget {
    editor: ViewHandle<EditorView>,
}

impl OpenRouterModelWidget {
    fn new(ctx: &mut ViewContext<WarpAgentPageView>) -> Self {
        let model = AISettings::as_ref(ctx).openrouter_code_review_model();
        let editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::handle(ctx).as_ref(ctx);
            let options = SingleLineEditorOptions {
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(editor_text_colors(appearance)),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(DEFAULT_OPENROUTER_CODE_REVIEW_MODEL, ctx);
            editor.set_buffer_text(&model, ctx);
            editor
        });
        ctx.subscribe_to_view(&editor, move |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let model = editor.as_ref(ctx).buffer_text(ctx);
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings.openrouter_code_review_model.set_value(model, ctx));
                });
                ctx.notify();
            }
        });
        Self { editor }
    }
}

impl SettingsWidget for OpenRouterModelWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "openrouter model slug claude gpt gemini code review commit message pull request title description"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        render_openrouter_input_row(appearance, "OpenRouter model", self.editor.clone(), app)
    }
}

struct AwsBedrockWidget {
    self_handle: WeakViewHandle<WarpAgentPageView>,
    aws_auth_refresh_command_editor: ViewHandle<EditorView>,
    aws_auth_refresh_profile_editor: ViewHandle<EditorView>,
    credentials_enabled_toggle: SwitchStateHandle,
    auto_login_toggle: SwitchStateHandle,
    refresh_credentials_button: ViewHandle<ActionButton>,
}

impl AwsBedrockWidget {
    fn new(ctx: &mut ViewContext<<Self as SettingsWidget>::View>) -> Self {
        let self_handle = ctx.handle();
        let ai_settings = AISettings::as_ref(ctx);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(ctx);

        let aws_auth_refresh_command = ai_settings.aws_bedrock_auth_refresh_command.value().clone();
        let aws_auth_refresh_profile = ai_settings.aws_bedrock_profile.value().clone();
        let user_workspaces = UserWorkspaces::as_ref(ctx);
        let scope = user_workspaces.team_context_for_view(ctx);
        let is_usage_enabled =
            is_any_ai_enabled && user_workspaces.is_aws_bedrock_credentials_enabled(&scope, ctx);

        let aws_auth_refresh_command_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::as_ref(ctx);
            let options = SingleLineEditorOptions {
                is_password: false,
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(TextColors {
                        default_color: appearance.theme().active_ui_text_color(),
                        disabled_color: appearance.theme().disabled_ui_text_color(),
                        hint_color: appearance.theme().disabled_ui_text_color(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("aws login", ctx);
            editor.set_buffer_text(&aws_auth_refresh_command, ctx);
            editor
        });
        update_editor_interaction_state(
            aws_auth_refresh_command_editor.clone(),
            is_usage_enabled,
            ctx,
        );
        ctx.subscribe_to_view(&aws_auth_refresh_command_editor, |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let should_reset = buffer_text.trim().is_empty();
                let value = if should_reset {
                    "aws login".to_string()
                } else {
                    buffer_text
                };
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings
                        .aws_bedrock_auth_refresh_command
                        .set_value(value, ctx);
                });
                if should_reset {
                    editor.update(ctx, |editor, ctx| {
                        editor.set_buffer_text("aws login", ctx);
                    });
                }
            }
        });

        let aws_auth_refresh_profile_editor = ctx.add_typed_action_view(move |ctx| {
            let appearance = Appearance::as_ref(ctx);
            let options = SingleLineEditorOptions {
                is_password: false,
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(TextColors {
                        default_color: appearance.theme().active_ui_text_color(),
                        disabled_color: appearance.theme().disabled_ui_text_color(),
                        hint_color: appearance.theme().disabled_ui_text_color(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text("default", ctx);
            editor.set_buffer_text(&aws_auth_refresh_profile, ctx);
            editor
        });
        update_editor_interaction_state(
            aws_auth_refresh_profile_editor.clone(),
            is_usage_enabled,
            ctx,
        );
        ctx.subscribe_to_view(&aws_auth_refresh_profile_editor, |_, editor, event, ctx| {
            if matches!(event, EditorEvent::Blurred | EditorEvent::Enter) {
                let buffer_text = editor.as_ref(ctx).buffer_text(ctx);
                let should_reset = buffer_text.trim().is_empty();
                let value = if should_reset {
                    "default".to_string()
                } else {
                    buffer_text
                };
                AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    let _ = settings.aws_bedrock_profile.set_value(value, ctx);
                });
                if should_reset {
                    editor.update(ctx, |editor, ctx| {
                        editor.set_buffer_text("default", ctx);
                    });
                }
            }
        });

        let refresh_credentials_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Refresh", SecondaryTheme)
                .with_icon(Icon::RefreshCw04)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(WarpAgentPageAction::RefreshAwsBedrockCredentials);
                })
        });
        refresh_credentials_button.update(ctx, |button, ctx| {
            button.set_disabled(!is_usage_enabled, ctx);
        });

        // Keep enablement in sync with the Global AI toggle.
        let aws_auth_refresh_command_editor_clone = aws_auth_refresh_command_editor.clone();
        let aws_auth_refresh_profile_editor_clone = aws_auth_refresh_profile_editor.clone();
        let refresh_credentials_button_clone = refresh_credentials_button.clone();
        ctx.subscribe_to_model(&AISettings::handle(ctx), move |_, _, event, ctx| {
            if matches!(
                event,
                AISettingsChangedEvent::IsAnyAIEnabled { .. }
                    | AISettingsChangedEvent::AwsBedrockCredentialsEnabled { .. }
            ) {
                let is_any_ai_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);
                let user_workspaces = UserWorkspaces::as_ref(ctx);
                let scope = user_workspaces.team_context_for_view(ctx);
                let is_usage_enabled = is_any_ai_enabled
                    && user_workspaces.is_aws_bedrock_credentials_enabled(&scope, ctx);

                update_editor_interaction_state(
                    aws_auth_refresh_command_editor_clone.clone(),
                    is_usage_enabled,
                    ctx,
                );
                update_editor_interaction_state(
                    aws_auth_refresh_profile_editor_clone.clone(),
                    is_usage_enabled,
                    ctx,
                );
                refresh_credentials_button_clone.update(ctx, |button, ctx| {
                    button.set_disabled(!is_usage_enabled, ctx);
                });

                ctx.notify();
            }
        });

        let aws_auth_refresh_command_editor_clone = aws_auth_refresh_command_editor.clone();
        let aws_auth_refresh_profile_editor_clone = aws_auth_refresh_profile_editor.clone();
        let refresh_credentials_button_clone = refresh_credentials_button.clone();
        ctx.subscribe_to_model(
            &UserWorkspaces::handle(ctx),
            move |_, workspace, event, ctx| {
                if let UserWorkspacesEvent::TeamsChanged = event {
                    let is_any_ai_enabled = AISettings::as_ref(ctx).is_any_ai_enabled(ctx);
                    let user_workspaces = workspace.as_ref(ctx);
                    let scope = user_workspaces.team_context_for_view(ctx);
                    let is_usage_enabled = is_any_ai_enabled
                        && user_workspaces.is_aws_bedrock_credentials_enabled(&scope, ctx);

                    update_editor_interaction_state(
                        aws_auth_refresh_command_editor_clone.clone(),
                        is_usage_enabled,
                        ctx,
                    );
                    update_editor_interaction_state(
                        aws_auth_refresh_profile_editor_clone.clone(),
                        is_usage_enabled,
                        ctx,
                    );
                    refresh_credentials_button_clone.update(ctx, |button, ctx| {
                        button.set_disabled(!is_usage_enabled, ctx);
                    });

                    ctx.notify();
                }
            },
        );

        Self {
            self_handle,
            aws_auth_refresh_command_editor,
            aws_auth_refresh_profile_editor,
            credentials_enabled_toggle: SwitchStateHandle::default(),
            auto_login_toggle: SwitchStateHandle::default(),
            refresh_credentials_button,
        }
    }

    fn render_aws_bedrock_section(
        &self,
        appearance: &Appearance,
        app: &AppContext,
        is_bedrock_available: bool,
    ) -> Box<dyn Element> {
        let ai_settings = AISettings::as_ref(app);
        let user_workspaces = UserWorkspaces::as_ref(app);
        let scope = user_workspaces.team_context(&self.self_handle, app);
        let is_any_ai_enabled = ai_settings.is_any_ai_enabled(app);
        let is_section_enabled = is_any_ai_enabled && is_bedrock_available;
        let is_admin_enforced = matches!(
            user_workspaces.aws_bedrock_host_enablement_setting(&scope),
            crate::workspaces::workspace::HostEnablementSetting::Enforce
        );
        let is_toggleable = is_section_enabled && !is_admin_enforced;
        let are_credentials_enabled =
            user_workspaces.is_aws_bedrock_credentials_enabled(&scope, app);
        let is_usage_enabled = is_section_enabled && are_credentials_enabled;
        let toggle_description = if is_admin_enforced {
            "Warp loads and sends local AWS CLI credentials for Bedrock-supported models. This setting is managed by your organization.".to_string()
        } else {
            "Warp loads and sends local AWS CLI credentials for Bedrock-supported models."
                .to_string()
        };

        let mut column = Flex::column().with_spacing(16.).with_child(
            Flex::column()
                .with_child(render_ai_setting_toggle::<AwsBedrockCredentialsEnabled>(
                    "Use AWS Bedrock credentials",
                    WarpAgentPageAction::ToggleAwsBedrockCredentialsEnabled,
                    are_credentials_enabled,
                    is_toggleable,
                    self.credentials_enabled_toggle.clone(),
                    &RefCell::new(HashMap::new()),
                    app,
                ))
                .with_child(render_ai_setting_description(
                    toggle_description,
                    is_section_enabled,
                    app,
                ))
                .finish(),
        );

        /// Helper function to render the UI for an input field.
        fn render_input(
            appearance: &Appearance,
            label: &'static str,
            editor: ViewHandle<EditorView>,
            is_enabled: bool,
            app: &AppContext,
        ) -> Box<dyn Element> {
            let padding = Some(Coords {
                top: 10.,
                bottom: 10.,
                left: 16.,
                right: 16.,
            });
            let editor_style = UiComponentStyles {
                padding,
                background: Some(appearance.theme().surface_2().into()),
                ..Default::default()
            };

            let label = Text::new_inline(label, appearance.ui_font_family(), CONTENT_FONT_SIZE)
                .with_color(styles::header_font_color(is_enabled, app).into())
                .finish();

            let input = appearance
                .ui_builder()
                .text_input(editor)
                .with_style(editor_style)
                .build()
                .finish();

            Flex::column()
                .with_spacing(8.)
                .with_child(label)
                .with_child(input)
                .finish()
        }

        fn render_credential_status_card(
            refresh_button: &ViewHandle<ActionButton>,
            appearance: &Appearance,
            are_credentials_enabled: bool,
            app: &AppContext,
        ) -> Box<dyn Element> {
            let (title_color, detail_color) = (
                styles::header_font_color(are_credentials_enabled, app),
                styles::description_font_color(are_credentials_enabled, app),
            );
            let (title_text, detail_text, icon) = ApiKeyManager::as_ref(app)
                .aws_credentials_state()
                .user_facing_components();

            let icon = Container::new(
                ConstrainedBox::new(icon.to_warpui_icon(title_color).finish())
                    .with_width(16.)
                    .with_height(16.)
                    .finish(),
            )
            .with_horizontal_padding(4.)
            .finish();

            let text_column = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_spacing(4.)
                .with_child(
                    Text::new_inline(title_text, appearance.ui_font_family(), CONTENT_FONT_SIZE)
                        .with_style(Properties::default().weight(Weight::Semibold))
                        .with_color(title_color.into())
                        .finish(),
                )
                .with_child(
                    Text::new(detail_text, appearance.ui_font_family(), CONTENT_FONT_SIZE)
                        .with_color(detail_color.into())
                        .soft_wrap(true)
                        .finish(),
                );

            Container::new(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(12.)
                    .with_child(
                        Expanded::new(
                            1.,
                            Flex::row()
                                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                                .with_spacing(12.)
                                .with_child(icon)
                                .with_child(Expanded::new(1., text_column.finish()).finish())
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_child(ChildView::new(refresh_button).finish())
                    .finish(),
            )
            .with_uniform_padding(12.)
            .with_background(appearance.theme().surface_2())
            .with_border(Border::all(1.).with_border_fill(appearance.theme().outline()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .finish()
        }

        column.add_child(
            Container::new(render_credential_status_card(
                &self.refresh_credentials_button,
                appearance,
                are_credentials_enabled,
                app,
            ))
            .with_margin_top(-styles::DESCRIPTION_MARGIN_BOTTOM)
            .finish(),
        );
        column.add_child(render_input(
            appearance,
            "Login Command",
            self.aws_auth_refresh_command_editor.clone(),
            is_usage_enabled,
            app,
        ));
        column.add_child(render_input(
            appearance,
            "AWS Profile",
            self.aws_auth_refresh_profile_editor.clone(),
            is_usage_enabled,
            app,
        ));

        let auto_login_enabled = *AISettings::as_ref(app).aws_bedrock_auto_login.value();

        let toggle = render_ai_setting_toggle::<AwsBedrockAutoLogin>(
            "Automatically run login command",
            WarpAgentPageAction::ToggleAwsBedrockAutoLogin,
            auto_login_enabled,
            is_usage_enabled,
            self.auto_login_toggle.clone(),
            &RefCell::new(HashMap::new()),
            app,
        );
        let description = render_ai_setting_description(
            "When enabled, the login command will run automatically when AWS Bedrock credentials expire.",
            is_usage_enabled,
            app,
        );
        column.add_child(
            Flex::column()
                .with_child(toggle)
                .with_child(description)
                .finish(),
        );

        column.finish()
    }
}

impl SettingsWidget for AwsBedrockWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "aws bedrock amazon credentials login command profile auto refresh"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        // Only show if admin has enabled AWS Bedrock for the window's team
        let scope = UserWorkspaces::as_ref(app).team_context(&self.self_handle, app);
        UserWorkspaces::as_ref(app).is_aws_bedrock_available(&scope)
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let scope = UserWorkspaces::as_ref(app).team_context(&self.self_handle, app);
        let is_bedrock_available = UserWorkspaces::as_ref(app).is_aws_bedrock_available(&scope);

        Container::new(self.render_aws_bedrock_section(appearance, app, is_bedrock_available))
            .with_margin_bottom(HEADER_PADDING)
            .finish()
    }
}

struct GeminiEnterpriseWidget {
    self_handle: WeakViewHandle<WarpAgentPageView>,
    credentials_enabled_toggle: SwitchStateHandle,
    refresh_credentials_button: ViewHandle<ActionButton>,
}

impl GeminiEnterpriseWidget {
    fn is_refresh_enabled<T: Entity>(ctx: &ViewContext<T>) -> bool {
        let user_workspaces = UserWorkspaces::as_ref(ctx);
        let scope = user_workspaces.team_context_for_view(ctx);
        AISettings::as_ref(ctx).is_any_ai_enabled(ctx)
            && UserWorkspaces::as_ref(ctx).is_gemini_enterprise_credentials_enabled(&scope, ctx)
            && !ApiKeyManager::as_ref(ctx)
                .geap_credentials_state()
                .requires_admin_action()
    }

    fn new(ctx: &mut ViewContext<<Self as SettingsWidget>::View>) -> Self {
        let self_handle = ctx.handle();
        let is_refresh_enabled = Self::is_refresh_enabled(ctx);
        let refresh_credentials_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("Refresh", SecondaryTheme)
                .with_icon(Icon::RefreshCw04)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(
                        WarpAgentPageAction::RefreshGeminiEnterpriseCredentials,
                    );
                })
        });
        refresh_credentials_button.update(ctx, |button, ctx| {
            button.set_disabled(!is_refresh_enabled, ctx);
        });

        let refresh_credentials_button_clone = refresh_credentials_button.clone();
        ctx.subscribe_to_model(&UserWorkspaces::handle(ctx), move |_, _, event, ctx| {
            if matches!(
                event,
                UserWorkspacesEvent::TeamsChanged
                    | UserWorkspacesEvent::UpdateWorkspaceSettingsSuccess
            ) {
                let is_refresh_enabled = Self::is_refresh_enabled(ctx);
                refresh_credentials_button_clone.update(ctx, |button, ctx| {
                    button.set_disabled(!is_refresh_enabled, ctx);
                });
                ctx.notify();
            }
        });

        let refresh_credentials_button_clone = refresh_credentials_button.clone();
        ctx.subscribe_to_model(&AISettings::handle(ctx), move |_, _, event, ctx| {
            if matches!(
                event,
                AISettingsChangedEvent::GeminiEnterpriseCredentialsEnabled { .. }
                    | AISettingsChangedEvent::IsAnyAIEnabled { .. }
            ) {
                let is_refresh_enabled = Self::is_refresh_enabled(ctx);
                refresh_credentials_button_clone.update(ctx, |button, ctx| {
                    button.set_disabled(!is_refresh_enabled, ctx);
                });
                ctx.notify();
            }
        });

        let refresh_credentials_button_clone = refresh_credentials_button.clone();
        ctx.subscribe_to_model(&ApiKeyManager::handle(ctx), move |_, _, event, ctx| {
            if matches!(event, ApiKeyManagerEvent::KeysUpdated) {
                let is_refresh_enabled = Self::is_refresh_enabled(ctx);
                refresh_credentials_button_clone.update(ctx, |button, ctx| {
                    button.set_disabled(!is_refresh_enabled, ctx);
                });
            }
        });

        Self {
            self_handle,
            credentials_enabled_toggle: SwitchStateHandle::default(),
            refresh_credentials_button,
        }
    }

    fn render_gemini_enterprise_section(
        &self,
        appearance: &Appearance,
        app: &AppContext,
        is_gemini_enterprise_available: bool,
    ) -> Box<dyn Element> {
        let user_workspaces = UserWorkspaces::as_ref(app);
        let scope = user_workspaces.team_context(&self.self_handle, app);
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);
        let is_section_enabled = is_any_ai_enabled && is_gemini_enterprise_available;
        let is_admin_enforced = matches!(
            user_workspaces.gemini_enterprise_host_enablement_setting(&scope),
            crate::workspaces::workspace::HostEnablementSetting::Enforce
        );
        let is_toggleable = is_section_enabled
            && user_workspaces.is_gemini_enterprise_credentials_toggleable(&scope);
        let are_credentials_enabled =
            user_workspaces.is_gemini_enterprise_credentials_enabled(&scope, app);
        let toggle_description = if is_admin_enforced {
            "Warp routes eligible requests through your workspace's Gemini Enterprise Google Cloud \
             project. This setting is managed by your organization."
                .to_string()
        } else {
            "Warp routes eligible requests through your workspace's Gemini Enterprise Google Cloud \
             project."
                .to_string()
        };

        let mut column = Flex::column().with_spacing(16.).with_child(
            Flex::column()
                .with_child(
                    render_ai_setting_toggle::<GeminiEnterpriseCredentialsEnabled>(
                        "Use Gemini Enterprise credentials",
                        WarpAgentPageAction::ToggleGeminiEnterpriseCredentialsEnabled,
                        are_credentials_enabled,
                        is_toggleable,
                        self.credentials_enabled_toggle.clone(),
                        &RefCell::new(HashMap::new()),
                        app,
                    ),
                )
                .with_child(render_ai_setting_description(
                    toggle_description,
                    is_section_enabled,
                    app,
                ))
                .finish(),
        );

        column.add_child(
            Container::new(self.render_credential_status_card(
                appearance,
                are_credentials_enabled,
                app,
            ))
            .with_margin_top(-styles::DESCRIPTION_MARGIN_BOTTOM)
            .finish(),
        );

        column.finish()
    }

    fn render_credential_status_card(
        &self,
        appearance: &Appearance,
        are_credentials_enabled: bool,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let manager = ApiKeyManager::as_ref(app);
        let (title_text, detail_text, icon) =
            manager.geap_credentials_state().user_facing_components();

        let (title_color, detail_color) = (
            styles::header_font_color(are_credentials_enabled, app),
            styles::description_font_color(are_credentials_enabled, app),
        );

        let icon = Container::new(
            ConstrainedBox::new(icon.to_warpui_icon(title_color).finish())
                .with_width(16.)
                .with_height(16.)
                .finish(),
        )
        .with_horizontal_padding(4.)
        .finish();

        let text_column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_spacing(4.)
            .with_child(
                Text::new_inline(title_text, appearance.ui_font_family(), CONTENT_FONT_SIZE)
                    .with_style(Properties::default().weight(Weight::Semibold))
                    .with_color(title_color.into())
                    .finish(),
            )
            .with_child(
                Text::new(detail_text, appearance.ui_font_family(), CONTENT_FONT_SIZE)
                    .with_color(detail_color.into())
                    .soft_wrap(true)
                    .finish(),
            );

        let row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(12.)
            .with_child(
                Expanded::new(
                    1.,
                    Flex::row()
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_spacing(12.)
                        .with_child(icon)
                        .with_child(Expanded::new(1., text_column.finish()).finish())
                        .finish(),
                )
                .finish(),
            )
            .with_child(ChildView::new(&self.refresh_credentials_button).finish());

        Container::new(row.finish())
            .with_uniform_padding(12.)
            .with_background(appearance.theme().surface_2())
            .with_border(Border::all(1.).with_border_fill(appearance.theme().outline()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .finish()
    }
}

impl SettingsWidget for GeminiEnterpriseWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "gemini enterprise geap google vertex credentials refresh"
    }

    fn should_render(&self, app: &AppContext) -> bool {
        let scope = UserWorkspaces::as_ref(app).team_context(&self.self_handle, app);
        FeatureFlag::GeminiEnterprise.is_enabled()
            && UserWorkspaces::as_ref(app).is_gemini_enterprise_available_from_workspace(&scope)
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let scope = UserWorkspaces::as_ref(app).team_context(&self.self_handle, app);
        let is_gemini_enterprise_available =
            UserWorkspaces::as_ref(app).is_gemini_enterprise_available_from_workspace(&scope);

        Container::new(self.render_gemini_enterprise_section(
            appearance,
            app,
            is_gemini_enterprise_available,
        ))
        .with_margin_bottom(HEADER_PADDING)
        .finish()
    }
}

/// Stable `&'static str` id for the custom model routers settings widget,
/// exposed for the `warp://settings?widget=custom_router` deeplink (see
/// `settings_widget_deeplink_target`).
pub(crate) fn custom_model_routers_widget_id() -> &'static str {
    CustomModelRoutersWidget::static_widget_id()
}

#[cfg(feature = "local_fs")]
#[derive(Default)]
struct AddCustomRouterWidget;

#[cfg(feature = "local_fs")]
impl SettingsWidget for AddCustomRouterWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "add new custom model router create"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::CustomModelRouters.is_enabled()
    }

    fn render(
        &self,
        view: &Self::View,
        _appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        Container::new(view.add_router_button.as_ref(app).render(app))
            .with_padding_bottom(HEADER_PADDING)
            .finish()
    }
}

#[derive(Default)]
struct CustomModelRoutersWidget;

impl SettingsWidget for CustomModelRoutersWidget {
    type View = WarpAgentPageView;

    fn search_terms(&self) -> &str {
        "custom model router complexity prompt auto model routing"
    }

    fn should_render(&self, _app: &AppContext) -> bool {
        FeatureFlag::CustomModelRouters.is_enabled()
    }

    #[cfg_attr(not(feature = "local_fs"), allow(unused_variables))]
    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let is_any_ai_enabled = AISettings::as_ref(app).is_any_ai_enabled(app);

        let mut column = Flex::column();

        column.add_child(render_ai_setting_description(
            "Automatically route tasks to specific models based on task complexity or custom rules. Custom routers will appear in your model selector menu.",
            is_any_ai_enabled,
            app,
        ));

        // Error cards and router summary cards (local_fs only)
        #[cfg(feature = "local_fs")]
        {
            use super::custom_router_view::render_router_error_card;
            use crate::user_config::WarpConfig;
            // Error cards (files that failed to parse) — shown first
            let errors = WarpConfig::as_ref(app).custom_model_router_errors();
            for error in errors.iter() {
                column.add_child(
                    Container::new(render_router_error_card(
                        &error.file_name,
                        &error.error_message,
                        appearance,
                    ))
                    .with_margin_top(8.)
                    .finish(),
                );
            }
            // Router summary cards
            for view_handle in &view.router_views {
                column.add_child(
                    Container::new(warpui::elements::ChildView::new(view_handle).finish())
                        .with_margin_top(8.)
                        .finish(),
                );
            }
        }

        // Add trailing space beneath this section (matching sibling sections
        // like AWS Bedrock) so the following section's title isn't crowded
        // against the router cards.
        Container::new(column.finish())
            .with_margin_bottom(HEADER_PADDING)
            .finish()
    }
}
