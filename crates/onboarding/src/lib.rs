// Onboarding library crate

mod agent_onboarding_view;
pub mod callout;
mod model;
pub mod slides;

pub use callout::{OnboardingCalloutView, OnboardingKeybindings};

pub mod components;

pub use agent_onboarding_view::{AgentOnboardingAction, AgentOnboardingEvent, AgentOnboardingView};
pub use model::SelectedSettings;

pub fn init(app: &mut warpui_core::AppContext) {
    agent_onboarding_view::init(app);
    callout::init(app);
}
