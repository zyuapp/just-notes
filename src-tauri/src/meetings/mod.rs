mod actions;
mod model;
mod notifications;
mod scheduler;
mod state;

use tauri::{AppHandle, Manager};

use crate::platform::{calendar, notifications as platform_notifications};

pub(crate) use actions::{handle_notification_action, start_meeting_recording};
pub(crate) use model::{MeetingAccess, MeetingCalendar, MeetingPrompt};
pub(crate) use notifications::categories as notification_categories;
pub(crate) use notifications::remove as remove_notifications;
pub(crate) use notifications::show_start_failure;
pub(crate) use scheduler::spawn as spawn_scheduler;
pub(crate) use state::MeetingSchedulerState;

pub(crate) fn dismiss_prompt(state: &MeetingSchedulerState, request_id: &str) -> bool {
    if state.dismiss_start_prompt(request_id) {
        platform_notifications::remove(&[request_id.to_string()]);
        return true;
    }
    false
}

pub(crate) fn normalize_settings(settings: &mut crate::settings::AppSettings) {
    if !settings.meeting_reminders_enabled {
        settings.meeting_auto_record_enabled = false;
    }
}

pub(crate) fn access_status() -> MeetingAccess {
    let calendar_authorization = calendar::authorization_status();
    let calendars = if calendar_authorization == "authorized" {
        calendar::list_calendars()
            .unwrap_or_default()
            .into_iter()
            .map(|calendar| MeetingCalendar {
                id: calendar.id,
                title: calendar.title,
                account: calendar.account,
                color: calendar.color,
            })
            .collect()
    } else {
        Vec::new()
    };
    MeetingAccess {
        calendar_authorization,
        notification_authorization: platform_notifications::authorization_status()
            .unwrap_or_else(|_| "unknown".to_string()),
        calendars,
    }
}

pub(crate) fn request_calendar_access(app: &AppHandle) -> Result<MeetingAccess, String> {
    calendar::request_access(app)?;
    Ok(access_status())
}

pub(crate) fn request_notification_access(app: &AppHandle) -> Result<MeetingAccess, String> {
    platform_notifications::request_access(app)?;
    Ok(access_status())
}

pub(crate) fn settings_updated(
    app: &AppHandle,
    previous: &crate::settings::AppSettings,
    current: &crate::settings::AppSettings,
) {
    let state = app.state::<MeetingSchedulerState>().inner().clone();
    let start_settings_changed = previous.meeting_reminders_enabled
        != current.meeting_reminders_enabled
        || previous.meeting_calendar_ids != current.meeting_calendar_ids
        || previous.meeting_reminder_minutes != current.meeting_reminder_minutes
        || previous.meeting_auto_record_enabled != current.meeting_auto_record_enabled;
    if start_settings_changed {
        platform_notifications::remove(&state.clear_start_prompts());
    }
    let end_settings_changed = previous.meeting_end_reminders != current.meeting_end_reminders;
    if !current.meeting_reminders_enabled {
        if let Some(request_id) = state.clear_active() {
            platform_notifications::remove(&[request_id]);
        }
    } else if end_settings_changed {
        if let Some(request_id) = state.update_active_end_reminders(current.meeting_end_reminders) {
            platform_notifications::remove(&[request_id]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_settings;
    use crate::settings::AppSettings;

    #[test]
    fn disabling_meeting_reminders_also_disables_automatic_recording() {
        let mut settings = AppSettings {
            meeting_auto_record_enabled: true,
            ..AppSettings::default()
        };
        normalize_settings(&mut settings);
        assert!(!settings.meeting_auto_record_enabled);
    }
}
