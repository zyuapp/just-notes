use std::{
    sync::mpsc::{self, SyncSender},
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};

use super::{
    call_app::{classify, CallApp},
    respond,
    state::CallDetectionState,
    tracker::CallTracker,
};
use crate::{
    meetings::notifications,
    platform::microphone_activity::{application_running, MicrophoneClient, MicrophoneWatch},
    recording::RecorderState,
    settings::SettingsState,
};

/// Disabled detection sleeps until settings wake it.
const DISABLED_WAIT: Duration = Duration::from_secs(60 * 60);
/// Listeners report microphone changes; this only bounds a missed one.
const SAFETY_WAIT: Duration = Duration::from_secs(60);
/// Recheck while this app's microphone and the recorder disagree, which
/// happens while a recording starts or stops. A disagreement that outlasts
/// the window (fixture audio, a lost input device) falls back to listeners.
const RECORDER_SETTLING_WAIT: Duration = Duration::from_millis(500);
const RECORDER_SETTLING_WINDOW: Duration = Duration::from_secs(10);
const MIN_WAIT: Duration = Duration::from_millis(100);

pub(crate) fn spawn(app: AppHandle, start_recording: impl Fn(&AppHandle, &str) + Send + 'static) {
    let (wake_sender, wake_receiver) = mpsc::sync_channel(1);
    app.state::<CallDetectionState>()
        .set_waker(wake_sender.clone());
    thread::spawn(move || {
        let mut watcher = Watcher {
            app,
            wake_sender,
            watch: None,
            tracker: CallTracker::default(),
            last_error: None,
            settling_since: None,
        };
        loop {
            let wait = watcher.tick(&start_recording);
            let _ = wake_receiver.recv_timeout(wait);
        }
    });
}

struct Watcher {
    app: AppHandle,
    wake_sender: SyncSender<()>,
    watch: Option<MicrophoneWatch>,
    tracker: CallTracker,
    last_error: Option<String>,
    settling_since: Option<Instant>,
}

impl Watcher {
    fn tick(&mut self, start_recording: &impl Fn(&AppHandle, &str)) -> Duration {
        let settings = self.app.state::<SettingsState>().snapshot();
        if !settings.call_detection_enabled {
            self.stop();
            return DISABLED_WAIT;
        }
        let wake_sender = self.wake_sender.clone();
        let watch = self.watch.get_or_insert_with(|| {
            MicrophoneWatch::start(move || {
                let _ = wake_sender.try_send(());
            })
        });
        let snapshot = match watch.snapshot() {
            Ok(snapshot) => {
                self.last_error = None;
                snapshot
            }
            Err(error) => {
                if self.last_error.as_ref() != Some(&error) {
                    eprintln!("call detection could not read microphone use: {error}");
                    self.last_error = Some(error);
                }
                return SAFETY_WAIT;
            }
        };
        let present = calling_apps(&snapshot.clients, &settings.call_detection_ignored_apps);
        let session_id = self.app.state::<RecorderState>().active_session_id();
        let now = Instant::now();
        let events = self.tracker.observe(now, &present, session_id.is_some());
        respond::reconcile(&self.app, session_id);
        for event in events {
            respond::handle(&self.app, &settings, session_id, event, start_recording);
        }
        let ceiling = self.ceiling(now, snapshot.own_input == session_id.is_some());
        self.tracker
            .next_deadline(session_id.is_some())
            .map(|deadline| deadline.saturating_duration_since(now).max(MIN_WAIT))
            .map_or(ceiling, |wait| wait.min(ceiling))
    }

    fn ceiling(&mut self, now: Instant, recorder_settled: bool) -> Duration {
        if recorder_settled {
            self.settling_since = None;
            return SAFETY_WAIT;
        }
        let since = *self.settling_since.get_or_insert(now);
        if now < since + RECORDER_SETTLING_WINDOW {
            RECORDER_SETTLING_WAIT
        } else {
            SAFETY_WAIT
        }
    }

    fn stop(&mut self) {
        if self.watch.take().is_none() {
            return;
        }
        self.tracker = CallTracker::default();
        let state = self.app.state::<CallDetectionState>();
        notifications::remove(&state.clear_offers());
        if let Some(request_id) = state.clear_stale_ended_prompt(None) {
            notifications::remove(&[request_id]);
        }
    }
}

fn calling_apps(clients: &[MicrophoneClient], ignored: &[String]) -> Vec<CallApp> {
    let mut apps = Vec::new();
    for app in clients
        .iter()
        .filter_map(|client| classify(client, application_running))
    {
        if !ignored.iter().any(|name| name == app.name) && !apps.contains(&app) {
            apps.push(app);
        }
    }
    apps
}
