use warpui::{AppContext, SingletonEntity};

use super::AIAgentCitation;
use crate::CloudModel;
use crate::server::telemetry::AgentModeCitation as CitationForTelemetry;

pub trait ForTelemetry {
    type Output;

    fn for_telemetry(&self, ctx: &AppContext) -> Option<Self::Output>;
}

impl ForTelemetry for AIAgentCitation {
    type Output = CitationForTelemetry;

    fn for_telemetry(&self, ctx: &AppContext) -> Option<Self::Output> {
        match self {
            Self::WarpDriveObject { uid } => {
                CloudModel::as_ref(ctx).get_by_uid(uid).map(|object| {
                    CitationForTelemetry::WarpDriveObject {
                        object_type: object.object_type(),
                        uid: object.uid(),
                    }
                })
            }
            Self::WarpDocumentation { path } => {
                Some(CitationForTelemetry::WarpDocs { page: path.clone() })
            }
            Self::WebPage { url } => Some(CitationForTelemetry::WebPage { url: url.clone() }),
            Self::AgentMemory {
                memory_store_id,
                memory_id,
                ..
            } => Some(CitationForTelemetry::AgentMemory {
                memory_store_id: memory_store_id.clone(),
                memory_id: memory_id.clone(),
            }),
        }
    }
}
