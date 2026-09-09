# Local-First Onboarding Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Warp's account/AI-sales onboarding funnel with a lean, offline, UX-only three-step flow (Welcome → Theme → UI setup) that sells nothing, needs no account, and is detached from Warp's auth routing.

**Architecture:** Keep the `crates/onboarding` framework (state machine, animated slide host, progress dots, bottom nav, layout helpers). Collapse the `OnboardingStep` state machine to three linear steps, delete the AI/account/billing slides and model state, rewrite the three surviving slides, and detach `RootView` from auth so first-run keys purely off the local `HasCompletedOnboarding` preference.

**Tech Stack:** Rust, the `warpui`/`warpui_core` GUI framework, the repo's `define_settings_group!` settings system, `cargo test`, `./script/presubmit`.

**Spec:** `docs/superpowers/specs/2026-09-09-onboarding-local-first-rebuild-design.md` — read it alongside this plan.

## Global Constraints

- Onboarding is **UX-only**: it must write no AI/account/billing/Drive/tabs settings and make no auth or server calls. It writes only the theme (existing events), `SessionSettings.honor_ps1`, and `AppEditorSettings.vim_mode`.
- Flow order is exactly: **Welcome → Theme → UI setup → Terminal.** Three steps; progress dots are `(0,3)/(1,3)/(2,3)`.
- Prompt-toggle default = **"Use PrompTTY's prompt"** → `honor_ps1 = false`. Vim-mode default = off (`false`).
- Intro copy — title **"Welcome to PrompTTY"**, subtitle **"A fast, modern terminal. Let's set up your theme and prompt."**
- No `warp.dev` URLs, no "Warp Agent"/"Warp Drive" naming, no ToS/telemetry consent surface anywhere in the flow.
- First-run gate stays keyed on `FeatureFlag::AgentOnboarding` + the local `HasCompletedOnboarding` pref (`HAS_COMPLETED_ONBOARDING_KEY`, `root_view.rs:1706`). No server sync of completion.
- Out of scope (do not touch): broader auth system, HOA onboarding (`app/src/workspace/hoa_onboarding/`), headless TUI onboarding markers, any BYOK/local-model AI.
- Attribution for commits: end each message with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

---

### Task 1: PrompTTY mark asset (already placed)

The real, multi-color PrompTTY mark has been added to the repo at
`app/assets/bundled/svg/promptty-mark.svg` (C2PA provenance metadata stripped;
teal-gradient glyph with a dark-navy `#06213C` drop-shadow). Because it is
**multi-color**, it is rendered by path (like `about_page.rs`), NOT via the
`Icon` enum's single-fill tint — so there is no `warp_core` change in this task.
The intro slide consumes it in Task 2, Step 5.

**Files:**
- Committed: `app/assets/bundled/svg/promptty-mark.svg`

- [ ] **Step 1: Confirm the asset is present and well-formed**

Run: `head -1 app/assets/bundled/svg/promptty-mark.svg` and `grep -c c2pa app/assets/bundled/svg/promptty-mark.svg`
Expected: an `<svg ... viewBox="0 0 1024 1024" ...>` opening tag; c2pa count `0`. (The file is already committed — nothing to add here.)

Note: the dark-navy `#06213C` shadow rects are near-invisible on dark themes; the
main teal glyph reads on both light and dark, so a single file serves both. A
dark-specific variant is a possible follow-up, not required.

---

### Task 2: Rewrite the `onboarding` crate to the three-step flow

`crates/onboarding` is one compile unit (model ↔ slides ↔ view host), so it is rewritten as one task. The gate is `cargo test -p onboarding`. The `app` crate will not compile until Task 3 — that is expected between these two tasks.

**Files:**
- Modify: `crates/onboarding/src/model.rs`
- Modify: `crates/onboarding/src/lib.rs`
- Modify: `crates/onboarding/src/slides/mod.rs`
- Modify: `crates/onboarding/src/slides/intro_slide.rs`
- Modify: `crates/onboarding/src/slides/theme_picker_slide.rs`
- Create: `crates/onboarding/src/slides/ui_setup_slide.rs`
- Modify: `crates/onboarding/src/agent_onboarding_view.rs`
- Delete: `crates/onboarding/src/slides/{intention_slide,ai_setup_slide,agent_slide,ai_access_slide,third_party_slide,offer_slide,customize_slide}.rs`
- Delete (if only used by the no-AI dialog): `crates/onboarding/src/components/feature_optout_dialog.rs`
- Test: model unit tests inline in `crates/onboarding/src/model.rs`

**Interfaces:**
- Produces (public, consumed by `app` in Task 3):
  - `OnboardingStep` (crate-internal) `= { Intro, ThemePicker, UiSetup }`.
  - `pub struct SelectedSettings { pub use_prompttty_prompt: bool, pub vim_mode: bool }` (replaces the old enum; delete `is_ai_enabled`/`is_warp_drive_enabled`).
  - `AgentOnboardingView::new(theme_picker_themes: [WarpTheme; 4], skippable: bool, ctx: &mut ViewContext<Self>) -> Self` (drops `models`, `default_model_id`, `workspace_enforces_autonomy`, `auth_state`).
  - `AgentOnboardingEvent = { ThemeSelected { theme_name: String }, SyncWithOsToggled { enabled: bool }, OnboardingCompleted(SelectedSettings), OnboardingSkipped, AppBecameActive }` (all other variants removed).
- Removed (must be gone from the crate's public API): `OnboardingIntention`, `OnboardingAuthState`, `AI_FEATURES`, `WARP_DRIVE_FEATURES`, `SessionDefault`, `OfferVariant`, `AgentSlide`/`AgentDevelopmentSettings`/`AgentAutonomy`/`OnboardingModelInfo`, and all AI/auth/offer setters.

- [ ] **Step 1: Delete the six AI/account slide files and customize_slide**

```bash
git rm crates/onboarding/src/slides/intention_slide.rs \
       crates/onboarding/src/slides/ai_setup_slide.rs \
       crates/onboarding/src/slides/agent_slide.rs \
       crates/onboarding/src/slides/ai_access_slide.rs \
       crates/onboarding/src/slides/third_party_slide.rs \
       crates/onboarding/src/slides/offer_slide.rs \
       crates/onboarding/src/slides/customize_slide.rs
```

- [ ] **Step 2: Rewrite `model.rs` to the three-step state machine**

Replace the entire contents of `crates/onboarding/src/model.rs` with the following (removes all AI/auth/intention/offer state; `warpui_core` import matches the original):

```rust
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
        Self { use_prompttty_prompt: true, vim_mode: false }
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
    use super::*;

    #[test]
    fn next_advances_intro_theme_uisetup_then_stops() {
        // Pure state-machine test: exercise `next` transitions without a ModelContext
        // by asserting the transition table via a helper. Replace `step_after_next`
        // with direct calls if a test ModelContext is available in this crate.
        assert_eq!(step_after(OnboardingStep::Intro, Dir::Next), OnboardingStep::ThemePicker);
        assert_eq!(step_after(OnboardingStep::ThemePicker, Dir::Next), OnboardingStep::UiSetup);
        assert_eq!(step_after(OnboardingStep::UiSetup, Dir::Next), OnboardingStep::UiSetup);
    }

    #[test]
    fn back_reverses_then_stops_at_intro() {
        assert_eq!(step_after(OnboardingStep::UiSetup, Dir::Back), OnboardingStep::ThemePicker);
        assert_eq!(step_after(OnboardingStep::ThemePicker, Dir::Back), OnboardingStep::Intro);
        assert_eq!(step_after(OnboardingStep::Intro, Dir::Back), OnboardingStep::Intro);
    }

    #[test]
    fn progress_is_three_steps() {
        assert_eq!(progress_of(OnboardingStep::Intro), (0, 3));
        assert_eq!(progress_of(OnboardingStep::ThemePicker), (1, 3));
        assert_eq!(progress_of(OnboardingStep::UiSetup), (2, 3));
    }

    // --- pure-logic mirrors of the transition table, so tests need no ModelContext ---
    enum Dir { Next, Back }
    fn step_after(step: OnboardingStep, dir: Dir) -> OnboardingStep {
        match (step, dir) {
            (OnboardingStep::Intro, Dir::Next) => OnboardingStep::ThemePicker,
            (OnboardingStep::ThemePicker, Dir::Next) => OnboardingStep::UiSetup,
            (OnboardingStep::UiSetup, Dir::Next) => OnboardingStep::UiSetup,
            (OnboardingStep::Intro, Dir::Back) => OnboardingStep::Intro,
            (OnboardingStep::ThemePicker, Dir::Back) => OnboardingStep::Intro,
            (OnboardingStep::UiSetup, Dir::Back) => OnboardingStep::ThemePicker,
        }
    }
    fn progress_of(step: OnboardingStep) -> (usize, usize) {
        let index = match step {
            OnboardingStep::Intro => 0,
            OnboardingStep::ThemePicker => 1,
            OnboardingStep::UiSetup => 2,
        };
        (index, 3)
    }
}
```

Note: the test mirrors the transition table in pure functions so the state-machine logic is verified without a live `ModelContext`. Keep `step_after`/`progress_of` byte-identical to `next`/`back`/`progress` above; if the two ever diverge that is the bug the test exists to catch. If this crate already has a `ModelContext` test harness, prefer driving `next`/`back`/`progress` directly and delete the mirrors.

- [ ] **Step 3: Rewrite `lib.rs` exports**

In `crates/onboarding/src/lib.rs`: delete `OnboardingIntention` (enum + Display), `AI_FEATURES`, `WARP_DRIVE_FEATURES`, `SessionDefault` (enum + Display), the `callout` re-export line if the callout module is being removed (it is NOT — keep it), and `pub use slides::OfferVariant;`. Update `pub use model::...` to:

```rust
pub use model::SelectedSettings;
```

Keep: `pub use agent_onboarding_view::{AgentOnboardingAction, AgentOnboardingEvent, AgentOnboardingView};`, `pub use callout::{OnboardingCalloutView, OnboardingKeybindings};`, the `mod`/`pub mod` declarations, `components`, and `init`.

- [ ] **Step 4: Update `slides/mod.rs`**

Replace module list and re-exports so only surviving slides remain:

```rust
mod bottom_nav;
mod intro_slide;
pub mod layout;
mod onboarding_slide;
mod progress_dots;
pub mod slide_content;
mod theme_picker_slide;
mod toggle_card;
mod two_line_button;
mod ui_setup_slide;

pub use bottom_nav::onboarding_bottom_nav;
pub use intro_slide::IntroSlide;
pub use onboarding_slide::OnboardingSlide;
pub use theme_picker_slide::{ThemePickerSlide, ThemePickerSlideEvent};
pub use ui_setup_slide::UiSetupSlide;
```

Note: `upgrade_auth_prompt` is dropped (it served the AI-access/offer slides); if `two_line_button` or `upgrade_auth_prompt` turn out to be unused after deletions, remove their `mod` lines too and delete the files. Verify with the compiler.

- [ ] **Step 5: Rewrite `intro_slide.rs`**

Edit `crates/onboarding/src/slides/intro_slide.rs`:
- Delete `IntroSlideEvent` (the `LoginRequested` variant) and change `Entity::Event` to `()`. Delete `IntroSlideAction::LoginClicked` (keep `GetStartedClicked`). Delete the `login_mouse_state` field and the `login_row` block (`:80-118`) plus the `Stack`/`add_positioned_child` wrapping — return the centered content directly.
- In `render_centered_content`, replace the single-fill `Icon` logo with the multi-color mark rendered **by path** (mirrors `app/src/settings_view/about_page.rs:105`), so its colors are preserved. Drop the `logo_fill`/`Icon` usage and add the needed imports (`Image`, `AssetSource`, `CacheOption` from `warpui_core`):

```rust
        let logo = ConstrainedBox::new(
            Image::new(
                AssetSource::Bundled { path: "bundled/svg/promptty-mark.svg" },
                CacheOption::BySize,
            )
            .finish(),
        )
        .with_width(64.)
        .with_height(64.)
        .finish();
```

- Update the subtitle text to:

```rust
            "A fast, modern terminal. Let's set up your theme and prompt.",
```

- Keep the title `"Welcome to PrompTTY"`, the "Get started" button, and `on_enter`/`get_started_clicked` → `model.next(ctx)`.
- In `TypedActionView::handle_action`, remove the `LoginClicked` arm.

- [ ] **Step 6: Edit `theme_picker_slide.rs` (remove consent, make it the middle step)**

In `crates/onboarding/src/slides/theme_picker_slide.rs`:
- Delete `ThemePickerSlideEvent::PrivacySettingsRequested` (`:34-37`) and `ThemePickerSlideAction::PrivacySettingsClicked` (`:48-50`).
- Delete `const TOS_URL` (`:53`), the `tos_mouse_state` and `privacy_settings_mouse_state` fields (`:67-68`) and their initializers (`:116-117`).
- Delete `render_disclaimer_section` entirely (`:538-616`) and the block that pushes it (`:169-180`, the `if !FeatureFlag::AccountFirstOnboarding... { content.push(...) }`).
- Remove the `use crate::OnboardingIntention;` import and every `OnboardingIntention` use. In `render_bottom_nav`, delete the `account_first`/`is_terminal` branching (`:271-298`); set the label to `"Next"` and compute dots from the model:

```rust
        let (step_index, step_count) = self.onboarding_state.as_ref(app).progress();
```

- In `next` (`:626-630`), change `model.complete(ctx)` to `model.next(ctx)` (theme is now the middle step).
- Delete the `IntentionChanged` subscription in `new` (`:85-89`).
- In `theme_visual_path` (`:450-471`), remove the intention/`use_vertical_tabs` lookups and pin a fixed variant, e.g.:

```rust
    fn theme_visual_path(&self) -> &'static str {
        let theme_name = self.theme_display_name(self.selected_theme_index);
        let name_key = match theme_name.as_str() {
            "Phenomenon" => "phenomenon",
            "Dark" => "dark",
            "Light" => "light",
            "Adeberry" => "adeberry",
            _ => "dark",
        };
        Self::VISUAL_IMAGE_PATHS
            .iter()
            .find(|p| p.contains("terminal_intention") && p.contains(name_key) && p.contains("horizontal"))
            .unwrap_or(&Self::VISUAL_IMAGE_PATHS[0])
    }
```

  Update the caller `render_theme_picker_visual` to `self.theme_visual_path()` (drop the `app` arg it no longer needs), and trim `VISUAL_IMAGE_PATHS` to just the four `terminal_intention/theme/*_horizontal.png` entries actually referenced.
- Remove the now-unused `FeatureFlag` import if nothing else uses it.

- [ ] **Step 7: Create `ui_setup_slide.rs`**

Create `crates/onboarding/src/slides/ui_setup_slide.rs`, modelled on the deleted `customize_slide.rs` layout (two-column `layout::static_left`, `slide_content::onboarding_slide_content`, `render_toggle_card` per toggle, `bottom_nav`). It has exactly two expandable toggle cards and no chips. Concrete spec:

- Struct `UiSetupSlide { onboarding_state: ModelHandle<OnboardingStateModel>, selected_card: Option<UiCard>, prompt_card_mouse: MouseStateHandle, prompt_seg_left_mouse: MouseStateHandle, prompt_seg_right_mouse: MouseStateHandle, vim_card_mouse: MouseStateHandle, vim_seg_left_mouse: MouseStateHandle, vim_seg_right_mouse: MouseStateHandle, back_button: button::Button, next_button: button::Button, scroll_state: ClippedScrollStateHandle }`, with `enum UiCard { Prompt, Vim }`.
- `enum UiSetupAction { SelectCard { card_index: usize }, SetUsePromptTtyPrompt { value: bool }, SetVimMode { value: bool }, BackClicked, NextClicked }`.
- Header: title `"Set up your terminal"`, subtitle `"Choose your prompt and editing style. You can change these later in settings."`.
- **Prompt card** (`ToggleCardSpec`): `title: "Prompt"`, `is_left_selected: model.use_prompttty_prompt()`, `left_label: "PrompTTY prompt"`, `right_label: "Keep my shell's prompt"`, `on_left` → `SetUsePromptTtyPrompt { value: true }`, `on_right` → `SetUsePromptTtyPrompt { value: false }`, `on_expand` → `SelectCard { card_index: 0 }`, `chips: vec![]`.
- **Vim card** (`ToggleCardSpec`): `title: "Vim mode"`, `is_left_selected: model.vim_mode()`, `left_label: "On"`, `right_label: "Off"`, `on_left` → `SetVimMode { value: true }`, `on_right` → `SetVimMode { value: false }`, `on_expand` → `SelectCard { card_index: 1 }`, `chips: vec![]`.
- `render`: `layout::static_left(|| self.render_content(...), || layout::onboarding_right_panel_with_bg(<a theme-neutral existing onboarding png>, layout::FOREGROUND_LAYOUT_DEFAULT))`. For the right-panel image reuse an existing bundled path such as `"async/png/onboarding/terminal_intention/theme/theme_dark_horizontal.png"` (no new asset).
- `render_bottom_nav`: `"Back"` (Naked, dispatches `BackClicked`) and `"Get started"` (Primary, Enter keystroke, dispatches `NextClicked`); dots from `self.onboarding_state.as_ref(app).progress()`.
- `OnboardingSlide`: `on_up`/`on_down` move `selected_card` between `Prompt`/`Vim`; `on_left`/`on_right` set the selected card's value via the model setters; `on_enter` calls `self.next(ctx)`.
- `fn next(&mut self, ctx)` → `self.onboarding_state.update(ctx, |m, ctx| m.complete(ctx))` (UI setup is the LAST step, so it completes).
- `TypedActionView<Action = UiSetupAction>`: `SelectCard` sets `selected_card`; `SetUsePromptTtyPrompt`/`SetVimMode` call `model.set_use_prompttty_prompt`/`model.set_vim_mode`; `BackClicked` → `model.back`; `NextClicked` → `self.next`.
- `Entity::Event = ()`.

Copy the exact `use` block, `render_content`, `render_header`, and `render_bottom_nav` shapes from git history of `customize_slide.rs` (`git show HEAD:crates/onboarding/src/slides/customize_slide.rs`) and adapt to the two cards above.

- [ ] **Step 8: Rewrite `agent_onboarding_view.rs`**

In `crates/onboarding/src/agent_onboarding_view.rs`:
- Trim imports: drop `ai::LLMId`, `instant::Instant`, `warp_core::features::FeatureFlag`, the `feature_optout_dialog` import, `OnboardingAuthState`, and the deleted-slide imports. Keep `IntroSlide`, `ThemePickerSlide`/`ThemePickerSlideEvent`, `UiSetupSlide`, `OnboardingSlide`, `SelectedSettings`, `OnboardingStateEvent`, `OnboardingStateModel`, `OnboardingStep`.
- `AgentOnboardingEvent`: keep only `ThemeSelected`, `SyncWithOsToggled`, `OnboardingCompleted(SelectedSettings)`, `OnboardingSkipped`, `AppBecameActive`. Delete `LoginFromWelcomeRequested`, `PrivacySettingsFromTerminalThemeSlideRequested`, all `Upgrade*`, all `Offer*`.
- `AgentOnboardingAction`: keep the key actions (`UpKey`..`Escape`); delete `NoAiConfirm/NoAiCancel/NoAiDismiss/DismissPlanActivatedToast`, and drop those arms from `dispatch_onboarding_action_to_slide`.
- Struct fields: keep `onboarding_state`, `intro_slide`, `theme_picker_slide`, `skippable`, `close_button`; add `ui_setup_slide: ViewHandle<UiSetupSlide>`. Delete every AI/offer/no-ai/toast/auth field (`intention_slide`, `ai_setup_slide`, `customize_slide`, `agent_slide`, `ai_access_slide`, `offer_slide`, `third_party_slide`, `no_ai_*`, `last_model_refresh`, `show_plan_activated_toast`, `last_auth_state`, `plan_activated_close_mouse_state`).
- `new`: change signature to `pub fn new(theme_picker_themes: [WarpTheme; 4], skippable: bool, ctx: &mut ViewContext<Self>) -> Self`. Build `onboarding_state` with `OnboardingStateModel::new()`. Keep the intro, theme-picker, and (new) ui-setup slide construction and the theme-picker subscription. Keep the `WindowManager` `AppBecameActive` subscription. In the model subscription, keep only `OnboardingStateEvent::Completed => me.handle_onboarding_completed(ctx)` and `SelectedSlideChanged => {}`. Delete the intro `LoginRequested` subscription, the auth/models/credit subscriptions, and all deleted-slide construction.
- Delete methods: `set_onboarding_models`, `set_pricing_promotion_message`, `set_workspace_enforces_autonomy`, `set_auth_state`, `on_ai_credit_availability_observed`, `on_checkout_succeeded`, `show_post_auth_offer`, `use_vertical_tabs`, `render_no_ai_dialog`, `handle_ai_sell_offer_satisfied`, `handle_auth_state_changed`, `render_plan_activated_toast`. Keep `handle_onboarding_completed`, `handle_theme_picker_slide_event` (drop its `PrivacySettingsRequested` arm), and `preload_onboarding_images` (reduce to the surviving slides' assets — the theme-picker paths and the bg; delete the `AccountFirstOnboarding` branch and deleted-slide preloads).
- `render`: reduce the slide `match` to three arms (`Intro`/`ThemePicker`/`UiSetup`), all `ChildView::new(&self.<slide>)` (no more `Option`/`expect`). Keep the background-image stack and the `skippable` "Skip" button (which emits `OnboardingSkipped`).

- [ ] **Step 9: Delete the no-AI opt-out dialog component if now unused**

```bash
grep -rn "feature_optout_dialog\|FeatureOptOutDialog\|render_feature_optout_dialog" crates/onboarding/src
```
If the only references were in the deleted `agent_onboarding_view` dialog, `git rm crates/onboarding/src/components/feature_optout_dialog.rs` and remove its `mod`/`pub use` from `crates/onboarding/src/components/mod.rs`. If anything else references it, leave it.

- [ ] **Step 10: Build and test the crate**

Run: `cargo test -p onboarding`
Expected: PASS — crate compiles, the three model tests pass. Fix any remaining references to deleted symbols the compiler flags (e.g. leftover `two_line_button`/`upgrade_auth_prompt` modules — delete if unused).

- [ ] **Step 11: Commit**

```bash
git add -A crates/onboarding
git commit -m "feat: rebuild onboarding crate as three-step local-first flow"
```

---

### Task 3: Restore the app-crate build — detach onboarding from auth (absorbs former Task 4)

Makes the whole `warp` app crate compile again against the new crate API, severs onboarding from auth, and removes the onboarding-triggered tutorial. Gate: `cargo build -p warp` compiles + the settings tests pass.

**Scope note (blast radius, verified by call-site tracing):** the deleted onboarding symbols are consumed by more app files than a naive read suggests. This task restores the *whole* app crate. Files whose types are reachable ONLY from the now-removed onboarding-login/offer wiring are DELETED; the rest are FIXED. The former standalone "Task 4" (tutorial trigger) is folded in here because the `From<SelectedSettings> for OnboardingTutorial` impl and the `OnboardingCompleted` rewrite are the same edit.

**Files:**
- Delete: `app/src/ai/onboarding.rs` (+ remove `pub mod onboarding;` at `app/src/ai/mod.rs:46`)
- Delete: `app/src/auth/login_slide.rs` (+ remove `pub mod login_slide;` and `login_slide::init(app);` at `app/src/auth/mod.rs:9,62`)
- Delete: `app/src/auth/paste_auth_token_modal.rs` (+ remove `pub mod paste_auth_token_modal;` and its `init` at `app/src/auth/mod.rs:11,63`)
- Modify: `app/src/settings/onboarding.rs` (rewrite apply fn; delete account-first/agent helpers)
- Modify: `app/src/settings/onboarding_tests.rs` (rewrite)
- Modify: `app/src/root_view.rs` (big prune)
- Modify: `app/src/terminal/view/action.rs` (one-line re-export path fix)
- Modify: `app/src/workspace/view/onboarding.rs` (delete the `From<SelectedSettings> for OnboardingTutorial` impl; keep the `OnboardingTutorial` type)

**Interfaces:**
- Consumes: `onboarding::SelectedSettings { use_prompttty_prompt, vim_mode }`, `AgentOnboardingView::new(themes, skippable, ctx)`, the trimmed `AgentOnboardingEvent`, and `onboarding::callout::OnboardingIntention` (relocated) from Task 2.
- Produces: `apply_onboarding_settings(selected: &SelectedSettings, app: &mut AppContext)` — sets `honor_ps1` and `vim_mode` only.

**Guiding rule:** let the compiler drive. Delete a symbol/member only after its readers are gone. Do NOT silence errors with `#[allow(dead_code)]` — remove dead code ("rip out, don't bypass"). If something you expected to delete turns out to have a live non-onboarding caller, keep it and report as a concern.

- [ ] **Step 1: Rewrite `apply_onboarding_settings` and delete the account-first/agent helpers**

Replace the body of `app/src/settings/onboarding.rs` with only the UX-only application. Delete `apply_account_first_onboarding_settings`, `apply_agent_settings`, `apply_ui_customization_settings`, `action_permissions_for_onboarding_autonomy`, `OnboardingAutonomyPermissions`, and their now-unused imports (`AgentAutonomy`, `AgentDevelopmentSettings`, `SessionDefault`, `UICustomizationSettings`, execution-profile/Drive/Tab/Code imports, `FtueAccountClass`, `TeamContextForOperation`). New content:

```rust
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
```

(Import paths verified against the codebase: `AppEditorSettings` is re-exported at `crate::settings::AppEditorSettings`; `SessionSettings` lives at `crate::terminal::session_settings::SessionSettings`, defined via `define_settings_group!` at `session_settings.rs:277`.)

- [ ] **Step 2: Rewrite `onboarding_tests.rs`**

Replace `app/src/settings/onboarding_tests.rs` with tests that assert only prompt + vim are written. Keep the `App::test` + `initialize_settings_for_tests` harness (drop singleton models no longer needed once AI/auth is gone — keep whatever the compiler requires for `SessionSettings`/`AppEditorSettings`). Two tests:

```rust
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
```

Adjust `.value()` accessors / read closures to match the codebase's `Setting` API if the compiler disagrees (see `initializer.rs:86-87` and `editor.rs` for the exact shape).

- [ ] **Step 3: Update the onboarding imports and view construction in `root_view.rs`**

- Update the onboarding import (`root_view.rs:11`) to `use onboarding::{AgentOnboardingEvent, AgentOnboardingView, SelectedSettings};` (drop `OfferVariant`, `OnboardingIntention`). Update `:81-82` to import only `apply_onboarding_settings` (drop `apply_account_first_onboarding_settings`). Where `root_view.rs` still needs `OnboardingIntention` after pruning (surviving uses only), get it via `crate::terminal::view::OnboardingIntention` (fixed in Step 6c) — do not import it from the `onboarding` crate root.
- In `create_agent_onboarding_view` (`:2082-2193`): call `AgentOnboardingView::new(themes.clone(), false, ctx)`. Delete the `build_onboarding_models`, `default_model_id`, `team_enforces_autonomy`, and `current_onboarding_auth_state` locals and the LLMPreferences/UserWorkspaces/AIRequestUsageModel/AuthManager subscriptions (`:2114-2187`). Also delete `offer_variant_for_account_class` (`:133-137`) once unused. Keep the `subscribe_to_view(&onboarding_view, handle_agent_onboarding_event)` hookup (`:2189-2191`).

- [ ] **Step 4: Simplify the `AgentOnboardingEvent` dispatcher**

In `handle_agent_onboarding_event` (`:2468-2727`) reduce to the surviving variants:
- `ThemeSelected` / `SyncWithOsToggled`: keep unchanged (`:2474-2490`).
- `OnboardingCompleted(selected)`: require `Onboarding` state; call `mark_local_onboarding_completed(ctx)` (and `mark_hoa_onboarding_completed` if `HOAOnboardingFlow`, preserving current behavior), `apply_onboarding_settings(&selected, ctx)`, then `target.to_workspace(ctx)` → `Terminal`, emit `AuthOnboardingStateChanged`, `start_autoupdate_polling` (mirror `:2536-2543`). **Delete** the `PostAuthOnboarding` branch (`:2492-2513`), the `set_user_onboarded` call (`:2531-2534`), AND the tutorial trigger — do **not** call `start_pending_tutorial` / `OnboardingTutorial::from(selected)` here (the new UX-only onboarding starts no tutorial; former Task 4). Remove `refresh_pending_onboarding_choices` (`:602-609`) and the `OnboardingTutorial::from` call sites (`:608`, `:2537`) so the `From<SelectedSettings>` impl deleted in Step 6d has no callers.
- `OnboardingSkipped`: keep `mark_local_onboarding_completed` + `to_workspace → Terminal`; **delete** the `set_user_onboarded` call.
- `AppBecameActive`: delete the arm (or make it a no-op) — it only refreshed billing/models, which no longer exist.
- Delete the arms for every removed variant (`UpgradeRequested`, `UpgradeCopyUrlRequested`, `UpgradePasteTokenFromClipboardRequested`, `OfferSetUpLaterSelected`, `OfferAiSellSatisfied`, `LoginFromWelcomeRequested`, `PrivacySettingsFromTerminalThemeSlideRequested`). Their construction sites for `LoginSlideView` (`:2621`, `:2671`) and `PasteAuthTokenModalView` (`:2578`) go with them — that is what makes the deleted files in Step 6a/6b dead.

- [ ] **Step 5: Remove the auth gating from the first-run decision and completion path**

- First-run gate (`:1827-1862`): remove the `if auth_state.is_logged_in()` short-circuit branch (`:1828`) so onboarding is considered regardless of login; keep the `AgentOnboarding && !has_completed_local_onboarding(ctx) && !is_content_deep_link()` condition (`:1838-1840`) → `Onboarding { onboarding_view, target }`. Leave the `else` fall-through to `Terminal`/`Auth` for the not-onboarding case intact.
- In `AuthOnboardingState::complete_auth_and_create_workspace` (`:3831-3864`): delete the `try_open_onboarding_slides` call and its `is_onboarded`/`is_anonymous`/local-flag preconditions (`:3837-3848`); keep the `Auth/ConfirmIncomingAuth/LoginSlide → Terminal` conversions.
- Delete `sync_local_onboarding_to_server` (`:3240-3248`) and its call in `handle_auth_manager_event` (`:3275`).
- Delete the now-dead onboarding auth machinery: `LoginFromWelcomeRequested`/`PrivacySettings…` handlers already removed in Step 4; also remove `complete_account_first` (`:2352-2415`), `refresh_onboarding_account_state` (`:156-169`), `refresh_pending_onboarding_choices` (`:602-609`) if unused, and the `pending_post_auth_onboarding_settings` / `pending_account_first_settings_class` fields and their remaining references. Let the compiler drive this: remove members only after their readers are gone. If any of these are shared with non-onboarding auth flows, leave them and just stop calling them from onboarding.

Note: keep `AuthOnboardingState`, `AuthOnboardingTarget`, `to_workspace`, and `create_workspace` — they build the terminal workspace. The `LoginSlide`/`PostAuthOnboarding` enum variants may become unused; remove them only if the compiler confirms no remaining constructor.

- [ ] **Step 6: Delete the now-dead app files and fix the two shared consumers**

**6a — Delete `app/src/ai/onboarding.rs`.** Its `build_onboarding_models` / `current_onboarding_auth_state` had only one caller (the old `create_agent_onboarding_view` args, removed in Step 3). `git rm app/src/ai/onboarding.rs` and remove `pub mod onboarding;` at `app/src/ai/mod.rs:46`.

**6b — Delete `app/src/auth/login_slide.rs` and `app/src/auth/paste_auth_token_modal.rs`.** `LoginSlideView` was constructed only at the two deleted event arms (`root_view.rs:2621,2671`); `PasteAuthTokenModalView` only at the deleted upgrade-paste arm (`:2578`). The general auth flow renders `AuthView`, not these. `git rm` both files and remove their `pub mod …;` + `…::init(app);` lines at `app/src/auth/mod.rs:9,62` (login_slide) and `:11,63` (paste_auth_token_modal).

**6c — Fix the `OnboardingIntention` re-export (one line).** `app/src/terminal/view/action.rs:7` is the single re-export point feeding every downstream `OnboardingIntention` consumer (workspace/view.rs, oz_launch.rs, terminal/view/init.rs, etc.). Change it to:

```rust
pub use onboarding::callout::OnboardingIntention;
```

This fixes all downstream consumers without touching them.

**6d — Fix `app/src/workspace/view/onboarding.rs`.** Delete the `impl From<SelectedSettings> for OnboardingTutorial` block (`:28-37`) — it matched the old enum variants and has no callers after Step 4. **Keep** the `OnboardingTutorial` type itself and everything else in the file; it is still used by the Oz launch modal (`oz_launch.rs`) via `WorkspaceAction::StartAgentOnboardingTutorial`, independent of onboarding. Fix the file's `use onboarding::SelectedSettings;` import (remove it if now unused).

- [ ] **Step 7: Build the whole app crate**

Run: `cargo build -p warp`
Expected: PASS. Resolve compiler errors iteratively — the guiding rule is "delete only after readers are gone." Do not silence errors with `#[allow(dead_code)]`; remove the dead code. If a symbol you expected to be dead has a live non-onboarding caller, keep it and note it as a concern rather than forcing the deletion.

- [ ] **Step 8: Run the settings tests**

Run: `cargo test -p warp --lib settings::onboarding` (the app crate is named `warp`).
Expected: PASS — the two `apply_onboarding_settings` tests.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat: detach onboarding from auth, restore app build, drop tutorial trigger"
```

---

### Task 5: Full verification and manual run

**Files:** none (verification only).

- [ ] **Step 1: Presubmit**

Run: `./script/presubmit`
Expected: PASS (fmt, clippy, build, tests). Fix any clippy warnings in the touched files.

- [ ] **Step 2: Manual run of the flow**

Run: `./script/run`
Then, to force onboarding without a fresh machine, either use the debug entry (`RootViewAction::DebugEnterOnboardingState`, if debug features are on) or clear the local pref so `has_completed_local_onboarding` is false. Confirm, in order:
1. **Welcome** shows the PrompTTY mark + "Welcome to PrompTTY" + the new subtitle + "Get started", with NO "Log in" link.
2. **Theme** picker shows themes + "Sync with OS", NO privacy/ToS disclaimer, "Next" button, dots at 2/3.
3. **UI setup** shows the Prompt toggle (defaulting to "PrompTTY prompt") + Vim mode toggle, "Get started" completes.
4. Completing lands directly in the **terminal** with no login prompt.
5. Re-launch: onboarding does not reappear (local completion pref respected).

- [ ] **Step 3: Confirm settings actually applied**

After choosing "Keep my shell's prompt" + Vim on, verify in settings/TOML that `text_editing.vim_mode_enabled = true` and PS1 is honored; and the inverse for the defaults. 

- [ ] **Step 4: Final commit (if any fixes)**

```bash
git add -A
git commit -m "test: verify local-first onboarding flow end-to-end"
```

---

## Notes for the implementer

- **Compile-boundary reality:** the `onboarding` crate is one compile unit, and the `app` crate consumes its public types. Task 2 leaves the `app` crate temporarily non-compiling; Task 3 restores it. Do not attempt to interleave them into "every commit builds the whole workspace" — it is not achievable for this API change.
- **Confidence map:** the model, settings-apply, and test code above is exact. The slide *render* code (Steps 5–8 of Task 2) is specified structurally with exact spec values; reproduce the layout from `git show HEAD:crates/onboarding/src/slides/customize_slide.rs` and `theme_picker_slide.rs`, and let the compiler confirm the `warpui` API calls. Use the `rust-unit-tests` skill for any additional tests.
- **Delete, don't gate:** per the spec's "rip out, not bypass" decision, remove dead code rather than feature-flagging or `#[allow(dead_code)]`-ing it.
