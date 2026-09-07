use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::AppId;

#[derive(Debug, Deserialize, Serialize)]
pub struct ChannelConfig {
    /// The application ID for this channel.
    pub app_id: AppId,

    /// The name of the file to which logs should be written.
    pub logfile_name: Cow<'static, str>,

    /// Configuration for talking to Warp's servers, or [`None`] if this build is
    /// local-only and must not contact Warp-hosted endpoints.
    #[serde(default)]
    pub server_config: Option<WarpServerConfig>,
    /// Configuration for Oz/ambient agents, or [`None`] if cloud agents are
    /// disabled for this build.
    #[serde(default)]
    pub oz_config: Option<OzConfig>,
    /// Configuration for autoupdate functionality.
    pub autoupdate_config: Option<AutoupdateConfig>,
    /// Configuration for statically-bundled MCP OAuth credentials.
    pub mcp_static_config: Option<McpStaticConfig>,
}

impl ChannelConfig {
    /// A channel config with no Warp-hosted services: no server, Oz,
    /// autoupdate, or bundled MCP OAuth credentials.
    pub fn local_only(app_id: AppId, logfile_name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            app_id,
            logfile_name: logfile_name.into(),
            server_config: None,
            oz_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        }
    }

    /// Whether this config can talk to a Warp-hosted server.
    pub fn cloud_enabled(&self) -> bool {
        self.server_config.is_some()
    }
}

/// Configuration for GCP Identity-Aware Proxy authentication, present only on staging builds.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IapConfig {
    /// The IAP OAuth2 client ID used as the audience for identity tokens.
    pub audiences: Cow<'static, str>,
    /// The service account email to impersonate when acquiring IAP credentials.
    pub service_account_email: Cow<'static, str>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WarpServerConfig {
    /// The root URL for the standard server pool.
    pub server_root_url: Cow<'static, str>,
    /// The URL for the RTC server, which serves real-time updates for Warp Drive objects.
    pub rtc_server_url: Cow<'static, str>,
    /// The URL for the session sharing server, or [`None`] if session sharing is not
    /// supported.
    pub session_sharing_server_url: Option<Cow<'static, str>>,
    /// The API key to use when making requests to Firebase Authentication endpoints.
    pub firebase_auth_api_key: Cow<'static, str>,
    /// Configuration for GCP Identity-Aware Proxy authentication, present only on
    /// staging builds. [`None`] on production builds.
    #[serde(default)]
    pub iap_config: Option<IapConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OzConfig {
    /// Root URL for the Oz (ambient agent management) dashboard.
    pub oz_root_url: Cow<'static, str>,

    /// URL to use as the audience when issuing workload identity tokens. If [`None`], falls back
    /// to [`WarpServerConfig::server_root_url`]. This exists so the audience is not overridden
    /// when a custom server root URL is provided (e.g. an ngrok URL for local development).
    pub workload_audience_url: Option<Cow<'static, str>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AutoupdateConfig {
    /// The base URL for fetching autoupdate versions and updated release bundles.
    pub releases_base_url: Cow<'static, str>,
    /// Whether or not to display menu items relating to autoupdate.
    pub show_autoupdate_menu_items: bool,
}

/// Configuration for statically-bundled MCP OAuth credentials.
///
/// These are credentials for OAuth providers where dynamic client registration
/// is not supported and we instead ship pre-registered client IDs and secrets.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpStaticConfig {
    /// Per-provider OAuth credentials.
    pub providers: Vec<McpOAuthProviderConfig>,
}

/// A single OAuth provider's credentials for MCP authentication.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpOAuthProviderConfig {
    /// The issuer URL of the OAuth provider (e.g. `https://github.com/login/oauth`).
    pub issuer: Cow<'static, str>,
    /// The OAuth client ID registered for this channel.
    pub client_id: Cow<'static, str>,
    /// The OAuth client secret registered for this channel.
    pub client_secret: Cow<'static, str>,
    /// A separately registered native client that permits loopback redirects.
    /// This must never be inferred from the custom-scheme client.
    #[serde(default)]
    pub loopback_client: Option<McpOAuthLoopbackClientConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpOAuthLoopbackClientConfig {
    pub client_id: Cow<'static, str>,
    /// Native OAuth clients should normally be public, but retain optional
    /// secret support for providers that require one.
    #[serde(default)]
    pub client_secret: Option<Cow<'static, str>>,
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
