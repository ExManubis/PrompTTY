use warp_core::features::FeatureFlag;
use warpui::{App, SingletonEntity};

use crate::auth::AuthStateProvider;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workspaces::user_workspaces::UserWorkspaces;

#[test]
fn byok_and_byoe_are_available_for_logged_out_solo_users() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| AuthStateProvider::new_logged_out_for_test());
        app.add_singleton_model(UserWorkspaces::default_mock);
        let _byok = FeatureFlag::SoloUserByok.override_enabled(true);

        UserWorkspaces::handle(&app).read(&app, |workspaces, ctx| {
            assert!(workspaces.is_byo_api_key_enabled(ctx));
            assert!(workspaces.is_byo_endpoint_enabled(ctx));
        });
    });
}
