use ui_components::{Component as _, Options as _, button};
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warpui_core::elements::{
    ClippedScrollStateHandle, Container, CrossAxisAlignment, Flex, FormattedTextElement,
    MainAxisSize, MouseStateHandle, ParentElement,
};
use warpui_core::fonts::Weight;
use warpui_core::keymap::Keystroke;
use warpui_core::prelude::Align;
use warpui_core::text_layout::TextAlignment;
use warpui_core::ui_components::components::{UiComponent as _, UiComponentStyles};
use warpui_core::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity as _, TypedActionView, View,
    ViewContext,
};

use super::OnboardingSlide;
use super::toggle_card::{ToggleCardSpec, render_toggle_card};
use crate::model::OnboardingStateModel;
use crate::slides::{bottom_nav, layout, slide_content};

/// Which setting card is currently selected (expanded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiCard {
    Prompt,
    Vim,
}

#[derive(Debug, Clone)]
pub enum UiSetupAction {
    SelectCard { card_index: usize },
    SetUsePromptTtyPrompt { value: bool },
    SetVimMode { value: bool },
    BackClicked,
    NextClicked,
}

pub struct UiSetupSlide {
    onboarding_state: ModelHandle<OnboardingStateModel>,
    selected_card: Option<UiCard>,
    prompt_card_mouse: MouseStateHandle,
    prompt_seg_left_mouse: MouseStateHandle,
    prompt_seg_right_mouse: MouseStateHandle,
    vim_card_mouse: MouseStateHandle,
    vim_seg_left_mouse: MouseStateHandle,
    vim_seg_right_mouse: MouseStateHandle,
    back_button: button::Button,
    next_button: button::Button,
    scroll_state: ClippedScrollStateHandle,
}

impl UiSetupSlide {
    pub(crate) fn new(onboarding_state: ModelHandle<OnboardingStateModel>) -> Self {
        Self {
            onboarding_state,
            selected_card: None,
            prompt_card_mouse: MouseStateHandle::default(),
            prompt_seg_left_mouse: MouseStateHandle::default(),
            prompt_seg_right_mouse: MouseStateHandle::default(),
            vim_card_mouse: MouseStateHandle::default(),
            vim_seg_left_mouse: MouseStateHandle::default(),
            vim_seg_right_mouse: MouseStateHandle::default(),
            back_button: button::Button::default(),
            next_button: button::Button::default(),
            scroll_state: ClippedScrollStateHandle::new(),
        }
    }

    fn render_content(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let bottom_nav = Align::new(self.render_bottom_nav(appearance, app)).finish();

        slide_content::onboarding_slide_content(
            vec![
                Align::new(self.render_header(appearance)).left().finish(),
                self.render_setting_cards(appearance, app),
            ],
            bottom_nav,
            self.scroll_state.clone(),
            appearance,
        )
    }

    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        let title = appearance
            .ui_builder()
            .paragraph("Set up your terminal")
            .with_style(UiComponentStyles {
                font_size: Some(36.),
                font_weight: Some(Weight::Medium),
                ..Default::default()
            })
            .build()
            .finish();

        let subtitle = FormattedTextElement::from_str(
            "Choose your prompt and editing style. You can change these later in settings.",
            appearance.ui_font_family(),
            16.,
        )
        .with_color(internal_colors::text_sub(
            appearance.theme(),
            appearance.theme().background().into_solid(),
        ))
        .with_weight(Weight::Normal)
        .with_alignment(TextAlignment::Left)
        .with_line_height_ratio(1.0)
        .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(title)
            .with_child(
                Container::new(subtitle)
                    .with_margin_top(16.)
                    .with_margin_bottom(40.)
                    .finish(),
            )
            .finish()
    }

    fn render_setting_cards(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let prompt_card = self.render_prompt_card(appearance, app);
        let vim_card = self.render_vim_card(appearance, app);

        Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Min)
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(12.)
                .with_child(prompt_card)
                .with_child(vim_card)
                .finish(),
        )
        .with_margin_top(12.)
        .finish()
    }

    fn render_prompt_card(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let is_selected = self.selected_card == Some(UiCard::Prompt);
        let is_left_selected = self.onboarding_state.as_ref(app).use_prompttty_prompt();

        render_toggle_card(
            appearance,
            ToggleCardSpec {
                title: "Prompt",
                is_expanded: is_selected,
                is_left_selected,
                left_label: "PrompTTY prompt",
                right_label: "Keep my shell's prompt",
                card_mouse_state: self.prompt_card_mouse.clone(),
                on_expand: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SelectCard { card_index: 0 });
                }),
                left_mouse: self.prompt_seg_left_mouse.clone(),
                right_mouse: self.prompt_seg_right_mouse.clone(),
                on_left: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SetUsePromptTtyPrompt { value: true });
                }),
                on_right: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SetUsePromptTtyPrompt {
                        value: false,
                    });
                }),
                chips: vec![],
            },
        )
    }

    fn render_vim_card(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let is_selected = self.selected_card == Some(UiCard::Vim);
        let is_left_selected = self.onboarding_state.as_ref(app).vim_mode();

        render_toggle_card(
            appearance,
            ToggleCardSpec {
                title: "Vim mode",
                is_expanded: is_selected,
                is_left_selected,
                left_label: "On",
                right_label: "Off",
                card_mouse_state: self.vim_card_mouse.clone(),
                on_expand: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SelectCard { card_index: 1 });
                }),
                left_mouse: self.vim_seg_left_mouse.clone(),
                right_mouse: self.vim_seg_right_mouse.clone(),
                on_left: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SetVimMode { value: true });
                }),
                on_right: Box::new(|ctx, _, _| {
                    ctx.dispatch_typed_action(UiSetupAction::SetVimMode { value: false });
                }),
                chips: vec![],
            },
        )
    }

    fn render_bottom_nav(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let back_button = self.back_button.render(
            appearance,
            button::Params {
                content: button::Content::Label("Back".into()),
                theme: &button::themes::Naked,
                options: button::Options {
                    on_click: Some(Box::new(|ctx, _app, _pos| {
                        ctx.dispatch_typed_action(UiSetupAction::BackClicked);
                    })),
                    ..button::Options::default(appearance)
                },
            },
        );

        let enter = Keystroke::parse("enter").unwrap_or_default();
        let next_button = self.next_button.render(
            appearance,
            button::Params {
                content: button::Content::Label("Get started".into()),
                theme: &button::themes::Primary,
                options: button::Options {
                    keystroke: Some(enter),
                    on_click: Some(Box::new(|ctx, _app, _pos| {
                        ctx.dispatch_typed_action(UiSetupAction::NextClicked);
                    })),
                    ..button::Options::default(appearance)
                },
            },
        );

        let (step_index, step_count) = self.onboarding_state.as_ref(app).progress();
        bottom_nav::onboarding_bottom_nav(
            appearance,
            step_index,
            step_count,
            Some(back_button),
            Some(next_button),
        )
    }

    fn render_visual(&self, app: &AppContext) -> Box<dyn Element> {
        // Mirror the theme the user picked on the theme slide.
        let path =
            layout::theme_screenshot_path(self.onboarding_state.as_ref(app).selected_theme_name());
        layout::onboarding_right_panel_with_bg(path, layout::FOREGROUND_LAYOUT_DEFAULT)
    }

    fn select_card(&mut self, card_index: usize, ctx: &mut ViewContext<Self>) {
        let card = match card_index {
            0 => UiCard::Prompt,
            1 => UiCard::Vim,
            _ => return,
        };
        self.selected_card = Some(card);
        ctx.notify();
    }

    fn next(&mut self, ctx: &mut ViewContext<Self>) {
        // UI setup is the last step, so advancing completes onboarding.
        self.onboarding_state.update(ctx, |model, ctx| {
            model.complete(ctx);
        });
    }
}

impl Entity for UiSetupSlide {
    type Event = ();
}

impl View for UiSetupSlide {
    fn ui_name() -> &'static str {
        "UiSetupSlide"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        layout::static_left(
            || self.render_content(appearance, app),
            || self.render_visual(app),
        )
    }
}

impl OnboardingSlide for UiSetupSlide {
    fn on_up(&mut self, ctx: &mut ViewContext<Self>) {
        self.selected_card = match self.selected_card {
            Some(UiCard::Vim) => Some(UiCard::Prompt),
            None => Some(UiCard::Prompt),
            other => other,
        };
        ctx.notify();
    }

    fn on_down(&mut self, ctx: &mut ViewContext<Self>) {
        self.selected_card = match self.selected_card {
            Some(UiCard::Prompt) => Some(UiCard::Vim),
            None => Some(UiCard::Prompt),
            other => other,
        };
        ctx.notify();
    }

    fn on_left(&mut self, ctx: &mut ViewContext<Self>) {
        match self.selected_card {
            Some(UiCard::Prompt) => {
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_use_prompttty_prompt(true, ctx);
                });
                ctx.notify();
            }
            Some(UiCard::Vim) => {
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_vim_mode(true, ctx);
                });
                ctx.notify();
            }
            None => {}
        }
    }

    fn on_right(&mut self, ctx: &mut ViewContext<Self>) {
        match self.selected_card {
            Some(UiCard::Prompt) => {
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_use_prompttty_prompt(false, ctx);
                });
                ctx.notify();
            }
            Some(UiCard::Vim) => {
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_vim_mode(false, ctx);
                });
                ctx.notify();
            }
            None => {}
        }
    }

    fn on_enter(&mut self, ctx: &mut ViewContext<Self>) {
        self.next(ctx);
    }
}

impl TypedActionView for UiSetupSlide {
    type Action = UiSetupAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            UiSetupAction::SelectCard { card_index } => {
                self.select_card(*card_index, ctx);
            }
            UiSetupAction::SetUsePromptTtyPrompt { value } => {
                let value = *value;
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_use_prompttty_prompt(value, ctx);
                });
                ctx.notify();
            }
            UiSetupAction::SetVimMode { value } => {
                let value = *value;
                self.onboarding_state.update(ctx, |model, ctx| {
                    model.set_vim_mode(value, ctx);
                });
                ctx.notify();
            }
            UiSetupAction::BackClicked => {
                let onboarding_state = self.onboarding_state.clone();
                onboarding_state.update(ctx, |model, ctx| {
                    model.back(ctx);
                });
            }
            UiSetupAction::NextClicked => {
                self.next(ctx);
            }
        }
    }
}
