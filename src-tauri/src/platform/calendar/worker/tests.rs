use super::*;

#[test]
fn busy_guard_clears_after_unwind_or_early_exit() {
    let busy = Arc::new(AtomicBool::new(true));
    let timed_out = Arc::new(AtomicBool::new(true));
    drop(WorkerBusyGuard::new(
        Arc::clone(&busy),
        Arc::clone(&timed_out),
    ));
    assert!(!busy.load(Ordering::Acquire));
    assert!(!timed_out.load(Ordering::Acquire));
}

#[test]
fn completed_guard_does_not_clear_the_next_request() {
    let busy = Arc::new(AtomicBool::new(true));
    let timed_out = Arc::new(AtomicBool::new(true));
    let mut guard = WorkerBusyGuard::new(Arc::clone(&busy), Arc::clone(&timed_out));
    guard.complete_before_reply();
    busy.store(true, Ordering::Release);
    drop(guard);
    assert!(busy.load(Ordering::Acquire));
    assert!(!timed_out.load(Ordering::Acquire));
}
