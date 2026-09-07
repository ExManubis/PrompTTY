use std::rc::Rc;

use warpui::ViewContext;

use super::Slide;

/// A callback function for custom CTA button actions.
type CustomCallback<S> = Rc<dyn Fn(&mut ViewContext<super::LaunchModal<S>>)>;

#[derive(Clone)]
pub struct CTAButton<S: Slide> {
    pub label: String,
    pub action: CTAButtonAction<S>,
}

impl<S: Slide> CTAButton<S> {
    // Constructor methods
    pub fn next_slide(next: S, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: CTAButtonAction::NextSlide(next),
        }
    }

    pub fn close(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: CTAButtonAction::Close,
        }
    }

    #[allow(dead_code)]
    pub fn open_url(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: CTAButtonAction::OpenUrl(url.into()),
        }
    }

    pub fn custom<F>(label: impl Into<String>, callback: F) -> Self
    where
        F: Fn(&mut ViewContext<super::LaunchModal<S>>) + 'static,
    {
        Self {
            label: label.into(),
            action: CTAButtonAction::Custom(Rc::new(callback)),
        }
    }
}

#[derive(Clone)]
pub enum CTAButtonAction<S: Slide> {
    NextSlide(S),
    Close,
    #[allow(dead_code)]
    OpenUrl(String),
    Custom(CustomCallback<S>),
}
