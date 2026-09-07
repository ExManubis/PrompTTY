//! Keep-types formerly co-located with product-analytics telemetry.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupGuideStep {
    /// Quick start banner: Visit Oz
    VisitOz,
    /// Step 1: Create environment (slash command)
    CreateEnvironment,
    /// Step 1: Create environment (CLI command)
    CreateEnvironmentCli,
    /// Step 2: Create Slack integration
    CreateSlackIntegration,
    /// Step 2: Create Linear integration
    CreateLinearIntegration,
}
