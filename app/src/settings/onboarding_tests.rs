use onboarding::SelectedSettings;
use settings::Setting as _;
use warpui::{App, SingletonEntity};

use super::apply_onboarding_settings;
use crate::settings::AppEditorSettings;
use crate::terminal::session_settings::SessionSettings;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn prompttty_prompt_disables_honor_ps1_and_sets_vim() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.update(|ctx| {
            apply_onboarding_settings(
                &SelectedSettings { use_prompttty_prompt: true, vim_mode: true },
                ctx,
            );
        });
        SessionSettings::handle(&app).read(&app, |s, _| assert!(!*s.honor_ps1.value()));
        AppEditorSettings::handle(&app).read(&app, |s, _| assert!(*s.vim_mode.value()));
    });
}

#[test]
fn keeping_shell_prompt_honors_ps1_and_vim_off() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.update(|ctx| {
            apply_onboarding_settings(
                &SelectedSettings { use_prompttty_prompt: false, vim_mode: false },
                ctx,
            );
        });
        SessionSettings::handle(&app).read(&app, |s, _| assert!(*s.honor_ps1.value()));
        AppEditorSettings::handle(&app).read(&app, |s, _| assert!(!*s.vim_mode.value()));
    });
}
