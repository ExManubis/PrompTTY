//! Module containing utilities to query the currently running antivirus / EDR software on the
//! user's machine.
use warpui::{Entity, ModelContext, SingletonEntity};

/// Singleton model that reports the currently running antivirus software.
#[derive(Debug, Clone, Default)]
pub struct AntivirusInfo;

impl AntivirusInfo {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }
}

pub enum AntivirusInfoEvent {}

impl Entity for AntivirusInfo {
    type Event = AntivirusInfoEvent;
}

impl SingletonEntity for AntivirusInfo {}
