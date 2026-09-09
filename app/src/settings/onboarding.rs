use onboarding::SelectedSettings;
use settings::Setting as _;
use warp_errors::report_if_error;
use warpui::{AppContext, SingletonEntity as _};

use crate::settings::AppEditorSettings;
use crate::terminal::session_settings::SessionSettings;

/// Applies the UX-only onboarding choices. Onboarding writes no AI/account
/// settings — only the prompt style and Vim mode. A PrompTTY account does not
/// exist; nothing here touches auth or the network.
pub(crate) fn apply_onboarding_settings(selected: &SelectedSettings, app: &mut AppContext) {
    // "Use PrompTTY's prompt" means do NOT honor the shell's PS1.
    let honor_ps1 = !selected.use_prompttty_prompt;
    SessionSettings::handle(app).update(app, |settings, ctx| {
        report_if_error!(settings.honor_ps1.set_value(honor_ps1, ctx));
    });

    AppEditorSettings::handle(app).update(app, |settings, ctx| {
        report_if_error!(settings.vim_mode.set_value(selected.vim_mode, ctx));
    });
}

#[cfg(test)]
#[path = "onboarding_tests.rs"]
mod tests;
