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

/// Display name of the default onboarding theme (Phenomenon). Used as the
/// initial `selected_theme_name` and the fallback for the UI-setup preview.
pub(crate) const DEFAULT_THEME_NAME: &str = "Phenomenon";

#[derive(Clone, Debug)]
pub(crate) struct OnboardingStateModel {
    step: OnboardingStep,
    use_prompttty_prompt: bool,
    vim_mode: bool,
    /// Display name of the theme chosen on the theme slide, so the UI-setup
    /// slide can show the matching preview screenshot.
    selected_theme_name: String,
}

impl OnboardingStateModel {
    pub(crate) fn new() -> Self {
        Self {
            step: OnboardingStep::Intro,
            use_prompttty_prompt: true,
            vim_mode: false,
            selected_theme_name: DEFAULT_THEME_NAME.to_string(),
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

    pub(crate) fn selected_theme_name(&self) -> &str {
        &self.selected_theme_name
    }

    pub(crate) fn set_selected_theme_name(&mut self, name: String, ctx: &mut ModelContext<Self>) {
        if self.selected_theme_name == name {
            return;
        }
        self.selected_theme_name = name;
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
#[path = "model_tests.rs"]
mod tests;
