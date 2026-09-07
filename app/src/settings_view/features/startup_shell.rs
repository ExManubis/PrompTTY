use warp_errors::report_if_error;
use warpui::elements::{CrossAxisAlignment, Fill, Flex, ParentElement, Shrinkable};
use warpui::presenter::ChildView;
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::{Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

use crate::appearance::Appearance;
use crate::editor::{EditorView, Event, SingleLineEditorOptions, TextOptions};

use crate::terminal::available_shells::{AvailableShell, AvailableShells};
use crate::terminal::local_tty::shell::is_valid_path_or_command_for_supported_shell;
use crate::terminal::session_settings::{SessionSettings, SessionSettingsChangedEvent};
use crate::view_components::dropdown::TOP_MENU_BAR_HEIGHT;
use crate::view_components::{Dropdown, DropdownItem};

/// A view for configuring the initial shell for new sessions. This can be the
/// user's login shell, the default installed version of zsh, bash, or fish,
/// or an arbitrary user-provided path.
pub struct StartupShellView {
    /// This dropdown is for selecting between the login shell, supported shells,
    /// and a custom shell.
    shell_dropdown: ViewHandle<Dropdown<NewSessionShellAction>>,
    /// This flags whether or not to show the custom path editor. It's toggled
    /// when the user chooses different dropdown options.
    should_display_editor: bool,
    /// If the user chose a custom shell path, they enter it in this editor.
    custom_path_editor: ViewHandle<EditorView>,
    /// This holds the current validity of the user's custom shell path, for
    /// drawing an error border if it's invalid.
    is_custom_path_valid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NewSessionShellAction {
    /// Changes the user's startup shell to the given option. This also hides
    /// the custom shell path editor if a non-custom shell was chosen.
    Set(AvailableShell),
    /// Displays the custom shell path editor.
    ShowCustomPathInput,
}


impl Entity for StartupShellView {
    type Event = ();
}

impl View for StartupShellView {
    fn ui_name() -> &'static str {
        "StartupShellView"
    }

    /// Renders controls to change the default shell for new sessions.
    fn render(&self, app: &warpui::AppContext) -> Box<dyn warpui::Element> {
        let appearance = Appearance::as_ref(app);
        let ui_builder = appearance.ui_builder();
        let theme = appearance.theme();

        let mut row = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);
        row.add_child(ChildView::new(&self.shell_dropdown).finish());

        if self.should_display_editor {
            let border_color: Option<Fill> = if self.is_custom_path_valid {
                None
            } else {
                Some(crate::themes::theme::Fill::error().into())
            };

            row.add_child(
                Shrinkable::new(
                    1.,
                    ui_builder
                        .text_input(self.custom_path_editor.clone())
                        .with_style(UiComponentStyles {
                            border_color,
                            // Make sure the editor is the same height as the dropdown it's next to.
                            height: Some(TOP_MENU_BAR_HEIGHT),
                            padding: Some(Coords::uniform(7.)),
                            margin: Some(Coords::default().left(8.).right(8.)),
                            font_size: Some(appearance.ui_font_size()),
                            background: Some(theme.surface_2().into()),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                )
                .finish(),
            )
        }

        row.finish()
    }
}

impl TypedActionView for StartupShellView {
    type Action = NewSessionShellAction;

    /// Handles a `NewSessionShellAction`, either triggered by the shell dropdown
    /// or by custom path editor events.
    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            NewSessionShellAction::ShowCustomPathInput => {
                self.should_display_editor = true;
                ctx.notify();
            }
            NewSessionShellAction::Set(shell) => {
                if shell.get_custom_path().is_none() && self.should_display_editor {
                    self.should_display_editor = false;
                    ctx.notify();
                }
                AvailableShells::handle(ctx).update(ctx, |shells, ctx| {
                    report_if_error!(shells.set_user_preferred_shell(shell.clone(), ctx));
                });
            }
        }
    }
}
