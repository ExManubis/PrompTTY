# Onboarding rebuild: local-first, UX-only (PIX-175)

**Date:** 2026-09-09
**Branch:** `mikkel/pix-175-update-onboarding`
**Status:** Design — awaiting review

## Problem

PrompTTY is a fork of Warp that is removing all Warp-hosted cloud services
(AI accounts, Drive, Oz, teams, billing, telemetry, autoupdate). The current
first-run onboarding is Warp's growth funnel: of its eight slides, roughly six
exist to create a Warp account and sell/set up hosted AI, and the whole flow is
wired into Warp's auth routing (`RootView` sends first-run users through login;
completion is synced to the Warp server). Only two strings were rebranded to
"PrompTTY"; the logo, all "Warp Agent"/"Warp Drive" naming, every `warp.dev`
URL, and the account-server dependency remain. On a fork with the backend gone,
most of the flow points at services that no longer exist.

## Goal

Replace the first-run onboarding with a lean, fully offline, **UX-only** flow
that sells nothing and depends on no account. It configures the user's terminal
and gets out of the way. AI is deliberately absent — BYOK and local models are
a later, separate effort, and onboarding must stay AI-agnostic so that work
owns those defaults cleanly.

### Target flow

Three steps, then straight into the terminal:

1. **Welcome** — rebranded intro (PrompTTY logo, "Get started"). No login link,
   no account pitch.
2. **Theme** — theme picker + "Sync light/dark with OS". No telemetry/ToS
   consent surface.
3. **UI setup** — two toggles: **Prompt** ("Use PrompTTY's prompt" vs "Keep my
   shell's prompt / PS1") and **Vim mode** (on/off). Its primary button
   completes onboarding.

## Approach

Approach **A** (chosen): keep the existing `crates/onboarding/` framework — its
state machine, animated slide host, progress dots, bottom nav, and layout
helpers are brand-neutral UX plumbing that already works. Strip the state
machine to three steps, delete the AI/account/billing slides and model state
outright (rip out, not bypass), rewrite the three surviving slides, and detach
the flow from `RootView`'s auth routing.

Rejected: B (fresh module from scratch — re-implements working layout/nav/focus
for the same three screens) and C (in-place edits leaving dead AI/account slides
referenced from the flow — the "bypass" flavor explicitly not wanted).

## Design

### 1. State machine — `crates/onboarding/src/model.rs`

Collapse `OnboardingStep` (`model.rs:104`) to:

```rust
enum OnboardingStep { Intro, ThemePicker, UiSetup }
```

- `next()`: `Intro → ThemePicker → UiSetup`; `UiSetup` completes.
- `back()`: reverse; `Intro` is the first step.
- `progress()`: `(0,3) / (1,3) / (2,3)`. Remove the `AccountFirstOnboarding`
  and intention branching in `next`/`back`/`progress`/`set_models`.

Delete from the model and crate:
- `OnboardingIntention` (`lib.rs:8`) and its `Terminal`/`AgentDrivenDevelopment`
  split.
- `OnboardingAuthState` (`model.rs:55`), `AiSetupChoice` (`model.rs:118`),
  `AiAccessChoice` (`model.rs:135`), `NoAiConfirmationSource` (`model.rs:153`),
  `OfferVariant` re-export (`lib.rs:70`), `SessionDefault` if unused after.
- Autonomy, model lists, pricing/offer/checkout, `show_post_auth_offer`,
  `on_credit_availability_observed`, `on_checkout_succeeded`, the no-AI
  confirmation, and every AI/tabs/drive/code-review setter.
- `AI_FEATURES` and `WARP_DRIVE_FEATURES` constants (`lib.rs:28`, `lib.rs:41`).

New `OnboardingStateModel` state: `step`, `use_prompttty_prompt: bool`,
`vim_mode: bool` (plus their setters + `ctx.notify`). Theme selection and
sync-with-OS continue to flow out as live events (not stored in the model), as
today.

`SelectedSettings` (`model.rs:63`) collapses from a two-variant enum to a single
struct:

```rust
pub struct SelectedSettings { pub use_prompttty_prompt: bool, pub vim_mode: bool }
```

Remove `is_ai_enabled()` / `is_warp_drive_enabled()`.

### 2. Slides — `crates/onboarding/src/slides/`

- **`intro_slide.rs`** (rewrite): render the multi-color PrompTTY mark
  (`bundled/svg/promptty-mark.svg`) **by path** via
  `Image::new(AssetSource::Bundled { .. }, CacheOption::BySize)` — mirroring
  `about_page.rs:105` — instead of the single-fill `Icon::WarpLogoLight`
  (`intro_slide.rs:142`); the mark is multi-color, so it must NOT go through the
  `Icon` tint path. **Keep** the shimmering text title below it. **Remove** the
  "Already have an account? Log in" link and its jump-to-login
  (`intro_slide.rs:83,91`, and the
  `IntroSlideEvent::LoginRequested`/`IntroSlideAction::LoginClicked` paths). Keep
  the "Get started" primary button. Final copy — title **"Welcome to
  PrompTTY"** (`:150`), subtitle **"A fast, modern terminal. Let's set up your
  theme and prompt."** (`:162`).
  - **Asset:** the real mark is committed at `app/assets/bundled/svg/promptty-mark.svg`
    (C2PA metadata stripped). Its dark-navy shadow is invisible on dark themes;
    the teal glyph reads on both, so one file serves both themes (a dark variant
    is an optional follow-up).
- **`theme_picker_slide.rs`** (edit): keep the theme grid + "Sync light/dark
  theme with OS". **Remove** the whole `render_disclaimer_section` (`:538-616`)
  and its call (`:169-180`), the `TOS_URL` const (`:53`), the
  `tos_mouse_state`/`privacy_settings_mouse_state` fields (`:67,68`), the
  `PrivacySettingsClicked` action (`:50`) and `PrivacySettingsRequested` event
  (`:37`), and its handler in `agent_onboarding_view.rs:642`.
  - **Now a middle step, not the last:** its Next currently calls
    `model.complete()` (`:628`) and is labelled "Get Warping" (`:272`) with
    hardcoded progress `(3,4)/(4,5)` (`:290-298`). Change Next to `model.next()`
    (advance to UI setup), relabel to **"Next"**, and use
    `self.onboarding_state.as_ref(app).progress()` for the dots.
  - **Drop intention/tabs coupling:** the visual-path picker uses
    `OnboardingIntention` and `use_vertical_tabs` (both deleted). Collapse
    `theme_visual_path` to a fixed variant (terminal-intention dir, horizontal
    orientation) and remove the `IntentionChanged` subscription.
- **`ui_setup_slide.rs`** (new): Prompt toggle + Vim-mode toggle, using the
  existing slide `layout`, `bottom_nav`, and toggle components. Final slide;
  primary button emits completion.
- **Delete:** `intention_slide.rs`, `ai_setup_slide.rs`, `agent_slide.rs`,
  `ai_access_slide.rs`, `third_party_slide.rs`, `offer_slide.rs`,
  `customize_slide.rs`, and the "Are you sure you don't want AI?" opt-out dialog
  (`render_feature_optout_dialog` / `FeatureOptOutDialog`) plus any `components/`
  piece that served only it. The `callout/` module is re-exported and `init`'d
  separately (`lib.rs:23,73`) and may back in-terminal callouts elsewhere — leave
  it unless it proves unused after the deletions. Update `slides/mod.rs` and
  `crates/onboarding/src/lib.rs` re-exports.

### 3. View host — `crates/onboarding/src/agent_onboarding_view.rs`

- Remove optional-slide fields for the deleted slides and the `expect("fallback
  slide exists")` arms; the `render` match (`:692`) shrinks to three arms.
- Remove `handle_auth_state_changed` (`:534`), the plan-activated toast
  (`:563`), `handle_ai_sell_offer_satisfied` (`:524`), `render_no_ai_dialog`
  (`:458`), and the `PrivacySettingsRequested` handling.
- `preload_onboarding_images` (`:418`) preloads only surviving slides' assets;
  drop the `AccountFirstOnboarding` branch.
- Keep `handle_onboarding_completed` (`:519`) emitting
  `AgentOnboardingEvent::OnboardingCompleted(SelectedSettings)`.
- Trim `AgentOnboardingEvent` / `AgentOnboardingAction` variants that only
  served deleted surfaces (offer, no-AI, plan toast, privacy settings).

### 4. RootView detachment — `app/src/root_view.rs`

First-run gate stays (`root_view.rs:1827`): show onboarding when
`FeatureFlag::AgentOnboarding.is_enabled() && !has_completed_local_onboarding(ctx)`
and not a content deep link. But the flow no longer touches auth:

- **Remove** login-slide injection into the flow, `sync_local_onboarding_to_server`,
  the server `is_onboarded` checks (`:3242`, `:3837`), and
  `try_open_onboarding_slides` post-auth re-entry (`:3866`).
- On `OnboardingCompleted`: apply settings → `mark_local_onboarding_completed`
  (`:1718`) → open the Terminal workspace. `has_completed_local_onboarding`
  (`:1709`) and the `"HasCompletedOnboarding"` local pref (`:1706`) are retained
  as the sole first-run signal.
- Leave the broader auth system in place (separate de-Warp work); only detach
  the onboarding path from it.

### 5. Settings application — `app/src/settings/onboarding.rs`

Rewrite `apply_onboarding_settings` to take the new `SelectedSettings` struct and:

- Set `SessionSettings.honor_ps1` from the Prompt toggle — `true` = "Keep my
  shell's prompt / PS1", `false` = "Use PrompTTY's prompt". Flipping `honor_ps1`
  already re-syncs `input_box_type` via `settings/init.rs:206`. **Default: "Use
  PrompTTY's prompt"** (`honor_ps1 = false`) pre-selected on the slide.
- Set `editor.vim_mode` (`settings/editor.rs:204`, toml
  `text_editing.vim_mode_enabled`) from the Vim-mode toggle.
- **Write no AI settings at all** (no `is_any_ai_enabled`, no autonomy, no
  tabs/drive/code-review). Onboarding stays AI-agnostic.

Delete `apply_account_first_onboarding_settings` (`:16`), `apply_agent_settings`
(`:188`), `apply_ui_customization_settings` (`:135`),
`action_permissions_for_onboarding_autonomy` (`:276`), and their now-unused
imports.

### 6. Post-deck tutorial — `app/src/workspace/view/onboarding.rs`

The in-terminal prompt/PS1 "block onboarding" tutorial was gated on AI being
enabled (`:75`), which is now never true from onboarding. Remove the dead
`start_agent_onboarding_tutorial` trigger from the onboarding-completion path so
no dangling reference remains. (The prompt choice is a plain slide toggle; the
interactive builder is not part of the flow.)

## Testing

- Rewrite `app/src/settings/onboarding_tests.rs`: assert the new
  `apply_onboarding_settings` sets `honor_ps1` and `vim_mode` for each toggle
  combination and touches no AI/drive/tabs settings.
- Add `model.rs` unit tests for the three-step `next` / `back` / `progress`.
- Follow the **`rust-unit-tests`** skill for warp-idiomatic tests. Verify with a
  scoped `cargo test -p onboarding` and the settings tests, then `./script/presubmit`.

## Scope boundaries (explicitly out of scope)

- Broader auth/account-system removal (separate de-Warp ticket).
- The HOA existing-user feature-intro flow (`app/src/workspace/hoa_onboarding/`).
- Headless TUI onboarding markers (`app/src/tui_onboarding_markers.rs`, server
  API, graphql).
- Any BYOK / local-model AI — a later effort; onboarding leaves AI defaults
  untouched for it to own.

## Open items / dependencies

- **PrompTTY mark SVG:** resolved — committed at
  `app/assets/bundled/svg/promptty-mark.svg` and rendered by path (no `Icon`
  variant needed). Optional follow-up: a dark-theme variant of the mark.
- `AccountFirstOnboarding` feature flag becomes unused by onboarding. Leave the
  flag defined (removing it is auth-adjacent cleanup) but remove its onboarding
  references.
- **RootView detachment specifics** (from the wiring map):
  - First-run gate `root_view.rs:1828-1862`: remove the `if
    auth_state.is_logged_in()` short-circuit and the `Auth`/`ForceLogin`
    branches from the onboarding decision; key onboarding purely off
    `AgentOnboarding && !has_completed_local_onboarding`, then open `Terminal`.
  - `OnboardingCompleted` handler `:2491-2544`: keep `mark_local_onboarding_completed`
    (`:2521`), `apply_onboarding_settings` (`:2529`), and the `to_workspace →
    Terminal` transition (`:2536-2543`); **remove** the `set_user_onboarded`
    call (`:2531-2534`) and the `PostAuthOnboarding` branch (`:2492-2513`). The
    existing `LoginLaterConfirmed` path (`:2443-2464`) is the model to mirror.
  - Remove/neutralize: `complete_auth_and_create_workspace` onboarding gate
    (`:3831-3848`), `sync_local_onboarding_to_server` (`:3240-3248`), all
    `set_user_onboarded` calls, `LoginFromWelcomeRequested` (`:2646-2698`),
    `PrivacySettingsFromTerminalThemeSlideRequested` (`:2590-2645`),
    `complete_account_first` (`:2352-2415`) and account-first offer/upgrade arms,
    `refresh_onboarding_account_state`, `apply_account_first_onboarding_settings`,
    and the `auth_state` argument + auth subscriptions on the onboarding view
    (`:2100,2108,2132-2187`).
  - `AgentOnboardingView::new` (`:2102-2110`): drop `models`, `default_model_id`,
    `workspace_enforces_autonomy`, `auth_state` args (all AI/auth-derived);
    keep `themes` and `skippable`.
