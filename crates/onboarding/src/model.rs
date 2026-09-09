use warpui_core::{Entity, ModelContext};

/// The three UX-only onboarding steps, in flow order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OnboardingStep {
    Intro,
    ThemePicker,
    UiSetup,
}

/// Settings chosen during onboarding and applied on completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedSettings {
    /// true  = use PrompTTY's prompt (honor_ps1 = false)
    /// false = keep the shell's existing prompt / PS1 (honor_ps1 = true)
    pub use_prompttty_prompt: bool,
    pub vim_mode: bool,
}

impl Default for SelectedSettings {
    fn default() -> Self {
        Self {
            use_prompttty_prompt: true,
            vim_mode: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum OnboardingStateEvent {
    SelectedSlideChanged,
    Completed,
}

#[derive(Clone, Debug)]
pub(crate) struct OnboardingStateModel {
    step: OnboardingStep,
    use_prompttty_prompt: bool,
    vim_mode: bool,
}

impl OnboardingStateModel {
    pub(crate) fn new() -> Self {
        Self {
            step: OnboardingStep::Intro,
            use_prompttty_prompt: true,
            vim_mode: false,
        }
    }

    pub(crate) fn step(&self) -> OnboardingStep {
        self.step
    }

    pub(crate) fn use_prompttty_prompt(&self) -> bool {
        self.use_prompttty_prompt
    }

    pub(crate) fn vim_mode(&self) -> bool {
        self.vim_mode
    }

    pub(crate) fn set_use_prompttty_prompt(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.use_prompttty_prompt == value {
            return;
        }
        self.use_prompttty_prompt = value;
        ctx.notify();
    }

    pub(crate) fn set_vim_mode(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.vim_mode == value {
            return;
        }
        self.vim_mode = value;
        ctx.notify();
    }

    pub(crate) fn settings(&self) -> SelectedSettings {
        SelectedSettings {
            use_prompttty_prompt: self.use_prompttty_prompt,
            vim_mode: self.vim_mode,
        }
    }

    pub(crate) fn set_step(&mut self, step: OnboardingStep, ctx: &mut ModelContext<Self>) {
        if self.step == step {
            return;
        }
        self.step = step;
        ctx.emit(OnboardingStateEvent::SelectedSlideChanged);
        ctx.notify();
    }

    pub(crate) fn next(&mut self, ctx: &mut ModelContext<Self>) {
        match self.step {
            OnboardingStep::Intro => self.set_step(OnboardingStep::ThemePicker, ctx),
            OnboardingStep::ThemePicker => self.set_step(OnboardingStep::UiSetup, ctx),
            OnboardingStep::UiSetup => {}
        }
    }

    pub(crate) fn back(&mut self, ctx: &mut ModelContext<Self>) {
        match self.step {
            OnboardingStep::Intro => {}
            OnboardingStep::ThemePicker => self.set_step(OnboardingStep::Intro, ctx),
            OnboardingStep::UiSetup => self.set_step(OnboardingStep::ThemePicker, ctx),
        }
    }

    pub(crate) fn complete(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.emit(OnboardingStateEvent::Completed);
        ctx.notify();
    }

    /// `(step_index, step_count)` for the bottom-nav progress dots.
    pub(crate) fn progress(&self) -> (usize, usize) {
        let index = match self.step {
            OnboardingStep::Intro => 0,
            OnboardingStep::ThemePicker => 1,
            OnboardingStep::UiSetup => 2,
        };
        (index, 3)
    }
}

impl Entity for OnboardingStateModel {
    type Event = OnboardingStateEvent;
}

#[cfg(test)]
mod tests {
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
}
