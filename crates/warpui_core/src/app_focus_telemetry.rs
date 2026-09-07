use chrono::{DateTime, Duration, Utc};

use crate::time::get_current_time;

// `DailyAppFocusDuration` records the cumulative duration for focus events that end on each day.
struct DailyAppFocusDuration {
    duration: Duration,
    last_synced_time: DateTime<Utc>,
}

impl DailyAppFocusDuration {
    // If calendar date has advanced since the last sync, reset the running total.
    fn try_reset_for_new_day(&mut self) {
        if get_current_time().date_naive() > self.last_synced_time.date_naive() {
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.duration = Duration::seconds(0);
        self.last_synced_time = get_current_time();
    }

    fn add_duration(&mut self, duration: Duration) {
        self.try_reset_for_new_day();
        if let Some(new_duration) = self.duration.checked_add(&duration) {
            self.duration = new_duration;
        } else {
            log::info!("Unable to increase the running total daily app focus duration.");
        }
    }
}

pub struct AppFocusInfo {
    last_time_app_focused: DateTime<Utc>,
    daily_app_focus_duration: DailyAppFocusDuration,
}

impl AppFocusInfo {
    pub fn new() -> Self {
        let now = get_current_time();
        Self {
            last_time_app_focused: now,
            daily_app_focus_duration: DailyAppFocusDuration {
                duration: Duration::seconds(0),
                last_synced_time: now,
            },
        }
    }

    pub fn record_app_focus(&mut self) {
        self.last_time_app_focused = get_current_time();
        self.try_record_daily_app_focus_duration();
    }

    pub fn try_record_daily_app_focus_duration(&mut self) {
        self.daily_app_focus_duration.try_reset_for_new_day();
    }

    pub fn record_app_blur(&mut self) {
        let app_focus_duration =
            get_current_time().signed_duration_since(self.last_time_app_focused);
        self.daily_app_focus_duration.add_duration(app_focus_duration);
    }
}
