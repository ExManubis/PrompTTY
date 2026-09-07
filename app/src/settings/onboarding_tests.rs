use onboarding::SelectedSettings;
use onboarding::slides::AgentDevelopmentSettings;
use settings::Setting as _;
use warpui::{App, SingletonEntity};

use super::apply_onboarding_settings;
use crate::LaunchMode;
use crate::ai::execution_profiles::profiles::AIExecutionProfilesModel;
use crate::ai::llms::LLMId;
use crate::ai::mcp::TemplatableMCPServerManager;
use crate::auth::AuthStateProvider;
use crate::cloud_object::model::actions::ObjectActions;
use crate::cloud_object::model::persistence::CloudModel;
use crate::network::NetworkStatus;
use crate::server::cloud_objects::update_manager::UpdateManager;
use crate::server::sync_queue::SyncQueue;
use crate::settings::AISettings;
use crate::settings::privacy::PrivacySettings;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workspaces::team_tester::TeamTesterStatus;
use crate::workspaces::user_profiles::UserProfiles;
use crate::workspaces::user_workspaces::{TeamContextForOperation, UserWorkspaces};

#[test]
fn apply_onboarding_settings_enables_ai_for_agent_intent_without_an_account() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| AuthStateProvider::new_logged_out_for_test());
        app.add_singleton_model(SyncQueue::mock);
        app.add_singleton_model(|_| NetworkStatus::new());
        app.add_singleton_model(TeamTesterStatus::mock);
        app.add_singleton_model(UpdateManager::mock);
        app.add_singleton_model(CloudModel::mock);
        app.add_singleton_model(|_| ObjectActions::new(Vec::new()));
        app.add_singleton_model(|_| TemplatableMCPServerManager::default());
        app.add_singleton_model(PrivacySettings::mock);
        app.add_singleton_model(|_| UserProfiles::new(Vec::new()));
        app.add_singleton_model(UserWorkspaces::default_mock);
        app.add_singleton_model(|ctx| {
            AIExecutionProfilesModel::new(&LaunchMode::new_for_unit_test(), ctx)
        });

        AISettings::handle(&app).update(&mut app, |settings, ctx| {
            settings.is_any_ai_enabled.set_value(false, ctx).unwrap();
        });

        app.update(|ctx| {
            apply_onboarding_settings(
                &SelectedSettings::AgentDrivenDevelopment {
                    agent_settings: AgentDevelopmentSettings::new(LLMId::from("test-model")),
                    ui_customization: None,
                },
                TeamContextForOperation::new_unscoped_for_test(),
                ctx,
            );
        });

        AISettings::handle(&app).read(&app, |settings, ctx| {
            assert!(*settings.is_any_ai_enabled);
            assert!(settings.is_any_ai_enabled(ctx));
        });
    });
}
