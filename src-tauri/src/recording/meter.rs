use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter};

use crate::{capture::SharedBuffers, ipc::MeterPayload, tray};

pub(super) fn spawn_meter_thread(
    app: AppHandle,
    thread_id: String,
    buffers: Arc<Mutex<SharedBuffers>>,
    should_stop: Arc<AtomicBool>,
    started: Instant,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last_tray_second = u64::MAX;
        while !should_stop.load(Ordering::Relaxed) {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            if let Ok(shared) = buffers.lock() {
                let _ = app.emit(
                    "meter-update",
                    MeterPayload {
                        thread_id: thread_id.clone(),
                        mic_level: shared.mic.level,
                        system_level: shared.system.level,
                        elapsed_ms,
                    },
                );
            }
            let second = elapsed_ms / 1000;
            if second != last_tray_second {
                tray::set_tray_elapsed(&app, elapsed_ms);
                last_tray_second = second;
            }

            thread::sleep(Duration::from_millis(100));
        }
    })
}
