//! The headless `warp-tui` front-end's app-side entry point.
//!
//! `warp_tui` boots the real headless Warp app via [`crate::run_tui`]. Once
//! shared initialization is done, [`init`] mounts the TUI immediately and
//! starts a local session. A Warp account is optional.
mod mcp;
mod user_info;

pub use mcp::{
    TuiMcpAction, TuiMcpConfigDiagnostic, TuiMcpFileScope, TuiMcpFileSource, TuiMcpInstallRequest,
    TuiMcpManager, TuiMcpManagerEvent, TuiMcpServerId, TuiMcpServerSnapshot, TuiMcpServerSource,
    TuiMcpServerStatus, TuiMcpSnapshot, TuiMcpSyncedTemplateProvenance, TuiMcpTemplateVariable,
    TuiMcpTransport, TuiMcpVariableValue,
};
pub use user_info::{TuiUserInfoManager, TuiUserInfoManagerEvent, TuiUserInfoSnapshot};
use warpui::{AppContext, SingletonEntity};

use crate::TuiMountFn;
use crate::ai::mcp::FileBasedMCPManager;
use crate::auth::auth_manager::{AuthManager, AuthManagerEvent};
use crate::auth::auth_state::AuthState;
use crate::auth::{self, AuthStateProvider};
use crate::tui_onboarding_markers::TuiOnboardingMarkers;

/// Entry point invoked from `run_internal` once the headless app is initialized.
///
/// Registers TUI-facing singletons, mounts the TUI immediately, and starts a
/// local session. Identity is optional; a missing account does not block the
/// terminal or file-based MCP.
pub(crate) fn init(mount: TuiMountFn, ctx: &mut AppContext) {
    ctx.add_singleton_model(TuiMcpManager::new);
    ctx.add_singleton_model(TuiUserInfoManager::new);
    let onboarding_markers = ctx.add_singleton_model(TuiOnboardingMarkers::new);

    ctx.subscribe_to_model(&AuthManager::handle(ctx), |_, event, ctx| {
        if matches!(event, AuthManagerEvent::AuthComplete) {
            TuiOnboardingMarkers::handle(ctx).update(ctx, |markers, ctx| {
                markers.load_current_account(ctx);
            });
            activate_global_mcp_servers(ctx);
        }
    });
    if has_validated_identity(AuthStateProvider::as_ref(ctx).get()) {
        onboarding_markers.update(ctx, |markers, ctx| {
            markers.load_current_account(ctx);
        });
    }
    mount(ctx);
    activate_global_mcp_servers(ctx);
}

pub(super) fn has_validated_identity(auth_state: &AuthState) -> bool {
    auth_state.is_logged_in() && auth_state.user_id().is_some()
}

/// Clears credentials without evicting the local terminal session.
pub fn log_out_tui(ctx: &mut AppContext) {
    auth::log_out(ctx);
    TuiOnboardingMarkers::handle(ctx).update(ctx, |markers, ctx| {
        markers.reset_for_account_transition(ctx);
    });
}

fn activate_global_mcp_servers(ctx: &mut AppContext) {
    FileBasedMCPManager::handle(ctx).update(ctx, |manager, ctx| {
        manager.activate_global_warp_servers(ctx);
    });
}
