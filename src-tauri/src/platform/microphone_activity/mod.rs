//! Processes capturing audio input, read from CoreAudio's process objects.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use objc2_core_audio::AudioObjectID;

mod coreaudio;
mod process;

pub(crate) use process::application_running;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunningApp {
    pub(crate) bundle_id: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MicrophoneClient {
    pub(crate) pid: i32,
    /// Bundle id CoreAudio reports for the capturing process, often a helper.
    pub(crate) audio_bundle_id: Option<String>,
    /// Nearest application registered with LaunchServices in the process's
    /// parent chain, starting with the process itself.
    pub(crate) app: Option<RunningApp>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MicrophoneSnapshot {
    /// Other processes capturing audio input.
    pub(crate) clients: Vec<MicrophoneClient>,
    /// Whether this process is capturing audio input.
    pub(crate) own_input: bool,
}

type ChangeHandler = Arc<dyn Fn() + Send + Sync>;

static CHANGE_HANDLER: Mutex<Option<ChangeHandler>> = Mutex::new(None);

/// Listens for processes starting or stopping audio input. Listeners stay
/// registered until the watch is dropped.
pub(crate) struct MicrophoneWatch {
    watched_processes: HashSet<AudioObjectID>,
}

impl MicrophoneWatch {
    /// Only one watch may exist at a time; `on_change` runs on a CoreAudio thread.
    pub(crate) fn start(on_change: impl Fn() + Send + Sync + 'static) -> Self {
        if let Ok(mut handler) = CHANGE_HANDLER.lock() {
            *handler = Some(Arc::new(on_change));
        }
        coreaudio::listen_to_process_list(true);
        Self {
            watched_processes: HashSet::new(),
        }
    }

    /// Processes capturing audio input now.
    pub(crate) fn snapshot(&mut self) -> Result<MicrophoneSnapshot, String> {
        objc2::rc::autoreleasepool(|_| {
            let objects = coreaudio::process_objects()?;
            self.sync_listeners(&objects);
            let own_pid = std::process::id() as i32;
            let mut snapshot = MicrophoneSnapshot::default();
            for object in objects {
                if !coreaudio::is_running_input(object) {
                    continue;
                }
                let Some(pid) = coreaudio::pid(object) else {
                    continue;
                };
                if pid == own_pid {
                    snapshot.own_input = true;
                    continue;
                }
                snapshot.clients.push(MicrophoneClient {
                    pid,
                    audio_bundle_id: coreaudio::bundle_id(object),
                    app: process::registered_ancestor(pid),
                });
            }
            Ok(snapshot)
        })
    }

    fn sync_listeners(&mut self, objects: &[AudioObjectID]) {
        let current = objects.iter().copied().collect::<HashSet<_>>();
        for gone in self.watched_processes.difference(&current) {
            coreaudio::listen_to_input_state(*gone, false);
        }
        for added in current.difference(&self.watched_processes) {
            coreaudio::listen_to_input_state(*added, true);
        }
        self.watched_processes = current;
    }
}

impl Drop for MicrophoneWatch {
    fn drop(&mut self) {
        coreaudio::listen_to_process_list(false);
        for object in self.watched_processes.drain() {
            coreaudio::listen_to_input_state(object, false);
        }
        if let Ok(mut handler) = CHANGE_HANDLER.lock() {
            *handler = None;
        }
    }
}

fn notify_change() {
    let handler = CHANGE_HANDLER
        .lock()
        .ok()
        .and_then(|handler| handler.clone());
    if let Some(handler) = handler {
        handler();
    }
}
