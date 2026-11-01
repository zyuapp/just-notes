use std::{
    collections::HashMap,
    sync::{mpsc::SyncSender, Arc, Mutex},
};

use super::call_app::CallApp;
use crate::meetings::{model::Meeting, scheduler::notification_id};

/// An offer to record a detected call, alive until answered or the call ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::meetings) struct CallOffer {
    pub(in crate::meetings) request_id: String,
    pub(in crate::meetings) app: CallApp,
    /// The calendar meeting the call belongs to; it names the thread.
    pub(in crate::meetings) meeting: Option<Meeting>,
    /// Started without asking because the meeting is set to record automatically.
    pub(in crate::meetings) automatic: bool,
}

impl CallOffer {
    pub(in crate::meetings) fn title(&self) -> String {
        self.meeting
            .as_ref()
            .map(|meeting| meeting.title.clone())
            .unwrap_or_else(|| self.app.thread_title())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::meetings) struct EndedPrompt {
    pub(in crate::meetings) request_id: String,
    pub(in crate::meetings) session_id: u64,
}

#[derive(Default)]
struct DetectionData {
    offers: HashMap<String, CallOffer>,
    ended_prompt: Option<EndedPrompt>,
    next_offer: u64,
    waker: Option<SyncSender<()>>,
}

#[derive(Clone, Default)]
pub(crate) struct CallDetectionState(Arc<Mutex<DetectionData>>);

impl CallDetectionState {
    pub(super) fn set_waker(&self, waker: SyncSender<()>) {
        if let Ok(mut data) = self.0.lock() {
            data.waker = Some(waker);
        }
    }

    /// Asks the watcher to re-read settings and the microphone now.
    pub(crate) fn wake(&self) {
        if let Some(waker) = self.0.lock().ok().and_then(|data| data.waker.clone()) {
            let _ = waker.try_send(());
        }
    }

    pub(super) fn add_offer(
        &self,
        app: CallApp,
        meeting: Option<Meeting>,
        automatic: bool,
    ) -> Option<CallOffer> {
        let mut data = self.0.lock().ok()?;
        data.next_offer = data.next_offer.wrapping_add(1);
        let offer = CallOffer {
            request_id: notification_id("call", &format!("{}:{}", app.name, data.next_offer)),
            app,
            meeting,
            automatic,
        };
        data.offers.insert(offer.request_id.clone(), offer.clone());
        Some(offer)
    }

    pub(in crate::meetings) fn take_offer(&self, request_id: &str) -> Option<CallOffer> {
        self.0.lock().ok()?.offers.remove(request_id)
    }

    pub(in crate::meetings) fn restore_offer(&self, offer: CallOffer) {
        if let Ok(mut data) = self.0.lock() {
            data.offers.insert(offer.request_id.clone(), offer);
        }
    }

    pub(super) fn remove_offers_for(&self, app: &CallApp) -> Vec<String> {
        let Ok(mut data) = self.0.lock() else {
            return Vec::new();
        };
        let removed = data
            .offers
            .values()
            .filter(|offer| offer.app == *app)
            .map(|offer| offer.request_id.clone())
            .collect::<Vec<_>>();
        data.offers.retain(|_, offer| offer.app != *app);
        removed
    }

    pub(super) fn clear_offers(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut data| {
                data.offers
                    .drain()
                    .map(|(request_id, _)| request_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Replaces the call-ended prompt, returning the one it replaced.
    pub(super) fn set_ended_prompt(&self, prompt: EndedPrompt) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        data.ended_prompt
            .replace(prompt)
            .map(|previous| previous.request_id)
    }

    pub(in crate::meetings) fn take_ended_prompt(&self, request_id: &str) -> Option<EndedPrompt> {
        let mut data = self.0.lock().ok()?;
        if data
            .ended_prompt
            .as_ref()
            .is_some_and(|prompt| prompt.request_id == request_id)
        {
            return data.ended_prompt.take();
        }
        None
    }

    /// Drops the call-ended prompt unless it belongs to `active_session_id`.
    pub(super) fn clear_stale_ended_prompt(
        &self,
        active_session_id: Option<u64>,
    ) -> Option<String> {
        let mut data = self.0.lock().ok()?;
        if data
            .ended_prompt
            .as_ref()
            .is_some_and(|prompt| Some(prompt.session_id) != active_session_id)
        {
            return data.ended_prompt.take().map(|prompt| prompt.request_id);
        }
        None
    }
}
