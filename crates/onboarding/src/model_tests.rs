use warpui_core::App;

use super::*;

#[test]
fn defaults_match_prompttty_prompt_on_vim_off() {
    App::test((), |mut app| async move {
        let handle = app.add_model(|_| OnboardingStateModel::new());
        handle.read(&app, |m, _| {
            assert_eq!(m.step(), OnboardingStep::Intro);
            assert!(m.use_prompttty_prompt());
            assert!(!m.vim_mode());
            assert_eq!(
                m.settings(),
                SelectedSettings {
                    use_prompttty_prompt: true,
                    vim_mode: false,
                }
            );
        });
    })
}

#[test]
fn next_advances_intro_theme_uisetup_then_stops() {
    App::test((), |mut app| async move {
        let handle = app.add_model(|_| OnboardingStateModel::new());

        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::Intro));
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| {
            assert_eq!(m.step(), OnboardingStep::ThemePicker)
        });
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::UiSetup));
        // UiSetup is the last step: `next` is a no-op there.
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::UiSetup));
    })
}

#[test]
fn back_reverses_then_stops_at_intro() {
    App::test((), |mut app| async move {
        let handle = app.add_model(|_| OnboardingStateModel::new());

        // Advance to the last step first.
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::UiSetup));

        handle.update(&mut app, |m, ctx| m.back(ctx));
        handle.read(&app, |m, _| {
            assert_eq!(m.step(), OnboardingStep::ThemePicker)
        });
        handle.update(&mut app, |m, ctx| m.back(ctx));
        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::Intro));
        // Intro is the first step: `back` is a no-op there.
        handle.update(&mut app, |m, ctx| m.back(ctx));
        handle.read(&app, |m, _| assert_eq!(m.step(), OnboardingStep::Intro));
    })
}

#[test]
fn progress_is_three_steps() {
    App::test((), |mut app| async move {
        let handle = app.add_model(|_| OnboardingStateModel::new());

        handle.read(&app, |m, _| assert_eq!(m.progress(), (0, 3)));
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| assert_eq!(m.progress(), (1, 3)));
        handle.update(&mut app, |m, ctx| m.next(ctx));
        handle.read(&app, |m, _| assert_eq!(m.progress(), (2, 3)));
    })
}

#[test]
fn setters_flip_values_and_flow_into_settings() {
    App::test((), |mut app| async move {
        let handle = app.add_model(|_| OnboardingStateModel::new());

        handle.update(&mut app, |m, ctx| {
            m.set_use_prompttty_prompt(false, ctx);
            m.set_vim_mode(true, ctx);
        });
        handle.read(&app, |m, _| {
            assert!(!m.use_prompttty_prompt());
            assert!(m.vim_mode());
            assert_eq!(
                m.settings(),
                SelectedSettings {
                    use_prompttty_prompt: false,
                    vim_mode: true,
                }
            );
        });
    })
}
