use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use objc2::rc::autoreleasepool;
use objc2_event_kit::EKEventStore;

use super::{
    store::{read_calendars, read_upcoming_events},
    CalendarEvent, CalendarInfo,
};

const STORE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BLOCKED_WORKERS: usize = 1;
static CALENDAR_SERVICE: OnceLock<CalendarService> = OnceLock::new();

struct CalendarService {
    state: Mutex<CalendarServiceState>,
}

struct CalendarServiceState {
    worker: CalendarWorker,
    retired_workers: Vec<thread::JoinHandle<()>>,
}

struct CalendarWorker {
    generation: u64,
    busy: Arc<AtomicBool>,
    timed_out: Arc<AtomicBool>,
    requests: mpsc::SyncSender<CalendarRequest>,
    thread: thread::JoinHandle<()>,
}

struct WorkerBusyGuard {
    busy: Arc<AtomicBool>,
    timed_out: Arc<AtomicBool>,
    armed: bool,
}

impl WorkerBusyGuard {
    fn new(busy: Arc<AtomicBool>, timed_out: Arc<AtomicBool>) -> Self {
        Self {
            busy,
            timed_out,
            armed: true,
        }
    }

    fn complete_before_reply(&mut self) {
        self.timed_out.store(false, Ordering::Release);
        self.busy.store(false, Ordering::Release);
        self.armed = false;
    }
}

impl Drop for WorkerBusyGuard {
    fn drop(&mut self) {
        if self.armed {
            self.timed_out.store(false, Ordering::Release);
            self.busy.store(false, Ordering::Release);
        }
    }
}

enum CalendarRequest {
    List(mpsc::Sender<Vec<CalendarInfo>>),
    Upcoming {
        calendar_ids: Vec<String>,
        from_ms: u64,
        to_ms: u64,
        reply: mpsc::Sender<Vec<CalendarEvent>>,
    },
}

pub(super) fn list_calendars() -> Result<Vec<CalendarInfo>, String> {
    let (reply, response) = mpsc::channel();
    let generation = send_calendar_request(CalendarRequest::List(reply))?;
    receive_calendar_response(generation, response, "Timed out while listing calendars")
}

pub(super) fn upcoming_events(
    calendar_ids: &[String],
    from_ms: u64,
    to_ms: u64,
) -> Result<Vec<CalendarEvent>, String> {
    let (reply, response) = mpsc::channel();
    let generation = send_calendar_request(CalendarRequest::Upcoming {
        calendar_ids: calendar_ids.to_vec(),
        from_ms,
        to_ms,
        reply,
    })?;
    receive_calendar_response(
        generation,
        response,
        "Timed out while reading calendar events",
    )
}

fn calendar_service() -> &'static CalendarService {
    CALENDAR_SERVICE.get_or_init(|| CalendarService {
        state: Mutex::new(CalendarServiceState {
            worker: spawn_calendar_worker(0),
            retired_workers: Vec::new(),
        }),
    })
}

fn send_calendar_request(request: CalendarRequest) -> Result<u64, String> {
    let service = calendar_service();
    let state = service
        .state
        .lock()
        .map_err(|_| "Calendar worker state is unavailable".to_string())?;
    let generation = state.worker.generation;
    let busy = Arc::clone(&state.worker.busy);
    let timed_out = Arc::clone(&state.worker.timed_out);
    if busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        drop(state);
        if timed_out.load(Ordering::Acquire) {
            restart_calendar_worker(generation);
        }
        return Err("Calendar worker is busy".to_string());
    }
    match state.worker.requests.try_send(request) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => {
            busy.store(false, Ordering::Release);
            return Err("Calendar worker is busy".to_string());
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {
            busy.store(false, Ordering::Release);
            drop(state);
            restart_calendar_worker(generation);
            return Err("Calendar worker is unavailable".to_string());
        }
    }
    Ok(generation)
}

fn receive_calendar_response<T>(
    generation: u64,
    response: mpsc::Receiver<T>,
    timeout_message: &str,
) -> Result<T, String> {
    match response.recv_timeout(STORE_REQUEST_TIMEOUT) {
        Ok(value) => Ok(value),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            mark_calendar_request_timed_out(generation);
            restart_calendar_worker(generation);
            Err(timeout_message.to_string())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            restart_calendar_worker(generation);
            Err("Calendar worker is unavailable".to_string())
        }
    }
}

fn mark_calendar_request_timed_out(generation: u64) {
    let service = calendar_service();
    if let Ok(state) = service.state.lock() {
        if state.worker.generation == generation {
            state.worker.timed_out.store(true, Ordering::Release);
        }
    }
}

fn restart_calendar_worker(generation: u64) {
    let service = calendar_service();
    if let Ok(mut state) = service.state.lock() {
        let mut blocked_workers = Vec::new();
        for worker in state.retired_workers.drain(..) {
            if worker.is_finished() {
                let _ = worker.join();
            } else {
                blocked_workers.push(worker);
            }
        }
        state.retired_workers = blocked_workers;
        if state.worker.generation == generation
            && state.retired_workers.len() < MAX_BLOCKED_WORKERS
        {
            let replacement = spawn_calendar_worker(generation.wrapping_add(1));
            let retired = std::mem::replace(&mut state.worker, replacement);
            state.retired_workers.push(retired.thread);
        }
    }
}

fn spawn_calendar_worker(generation: u64) -> CalendarWorker {
    // Capacity one removes the scheduling race before the worker reaches recv.
    let (requests, receiver) = mpsc::sync_channel(1);
    let busy = Arc::new(AtomicBool::new(false));
    let timed_out = Arc::new(AtomicBool::new(false));
    let worker_busy = Arc::clone(&busy);
    let worker_timed_out = Arc::clone(&timed_out);
    let worker_thread =
        thread::spawn(move || run_calendar_worker(receiver, worker_busy, worker_timed_out));
    CalendarWorker {
        generation,
        busy,
        timed_out,
        requests,
        thread: worker_thread,
    }
}

fn run_calendar_worker(
    receiver: mpsc::Receiver<CalendarRequest>,
    busy: Arc<AtomicBool>,
    timed_out: Arc<AtomicBool>,
) {
    let store = autoreleasepool(|_| unsafe { EKEventStore::new() });
    while let Ok(request) = receiver.recv() {
        let mut busy_guard = WorkerBusyGuard::new(Arc::clone(&busy), Arc::clone(&timed_out));
        autoreleasepool(|_| {
            unsafe { store.refreshSourcesIfNecessary() };
            match request {
                CalendarRequest::List(reply) => {
                    let calendars = read_calendars(&store);
                    busy_guard.complete_before_reply();
                    let _ = reply.send(calendars);
                }
                CalendarRequest::Upcoming {
                    calendar_ids,
                    from_ms,
                    to_ms,
                    reply,
                } => {
                    let events = read_upcoming_events(&store, &calendar_ids, from_ms, to_ms);
                    busy_guard.complete_before_reply();
                    let _ = reply.send(events);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests;
