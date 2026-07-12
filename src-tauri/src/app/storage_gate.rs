use std::sync::{Arc, Mutex, MutexGuard};

/// Serializes storage-root changes with operations that capture effective paths.
#[derive(Clone, Default)]
pub(crate) struct StorageGate(Arc<Mutex<()>>);

impl StorageGate {
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, ()>, String> {
        self.0
            .lock()
            .map_err(|_| "Storage operation lock was poisoned".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::StorageGate;
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn blocks_competing_storage_operations_until_guard_drops() {
        let gate = StorageGate::default();
        let guard = gate.lock().unwrap();
        let contender = gate.clone();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _guard = contender.lock().unwrap();
            sender.send(()).unwrap();
        });

        assert!(receiver.recv_timeout(Duration::from_millis(25)).is_err());
        drop(guard);
        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
    }
}
