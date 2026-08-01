use std::{
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use block2::RcBlock;
use objc2::rc::autoreleasepool;
use objc2_foundation::{NSArray, NSError, NSString};
use objc2_user_notifications::{
    UNMutableNotificationContent, UNNotification, UNNotificationRequest, UNNotificationSound,
    UNUserNotificationCenter,
};

type DeliveryHandler = Arc<dyn Fn(Result<(), String>) + Send + Sync>;

pub(crate) fn show(request_id: &str, title: &str, body: &str, category: &str) {
    let logged_request_id = request_id.to_string();
    show_with_error_handler(request_id, title, body, category, move |error| {
        eprintln!("notification {logged_request_id} could not be delivered: {error}");
    });
}

pub(crate) fn show_informational(request_id: &str, title: &str, body: &str) {
    show(request_id, title, body, "");
}

pub(crate) fn show_with_error_handler(
    request_id: &str,
    title: &str,
    body: &str,
    category: &str,
    on_error: impl Fn(String) + Send + Sync + 'static,
) {
    show_with_completion_handler(request_id, title, body, category, move |result| {
        if let Err(error) = result {
            on_error(error);
        }
    });
}

pub(crate) fn show_with_completion_handler(
    request_id: &str,
    title: &str,
    body: &str,
    category: &str,
    on_complete: impl Fn(Result<(), String>) + Send + Sync + 'static,
) {
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));
    content.setCategoryIdentifier(&NSString::from_str(category));
    content.setSound(Some(&UNNotificationSound::defaultSound()));
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
        &NSString::from_str(request_id),
        &content,
        None,
    );
    let expected_request_id = request_id.to_string();
    let on_complete = Arc::new(on_complete);
    let completion = RcBlock::new(move |error: *mut NSError| {
        if let Some(error) = unsafe { error.as_ref() } {
            on_complete(Err(error.localizedDescription().to_string()));
            return;
        }
        confirm_delivery(expected_request_id.clone(), on_complete.clone());
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
}

fn confirm_delivery(expected_request_id: String, on_complete: DeliveryHandler) {
    thread::spawn(move || {
        for _ in 0..6 {
            thread::sleep(Duration::from_millis(100));
            if notification_was_delivered(&expected_request_id) {
                on_complete(Ok(()));
                return;
            }
        }
        on_complete(Err("Notification was not delivered".to_string()));
    });
}

fn notification_was_delivered(expected_request_id: &str) -> bool {
    let (sender, receiver) = mpsc::channel();
    autoreleasepool(|_| {
        let expected_request_id = expected_request_id.to_string();
        let delivered = RcBlock::new(
            move |notifications: std::ptr::NonNull<NSArray<UNNotification>>| {
                let found = unsafe { notifications.as_ref() }
                    .iter()
                    .any(|notification| {
                        notification.request().identifier().to_string() == expected_request_id
                    });
                let _ = sender.send(found);
            },
        );
        UNUserNotificationCenter::currentNotificationCenter()
            .getDeliveredNotificationsWithCompletionHandler(&delivered);
    });
    receiver
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or(false)
}

pub(crate) fn remove(request_ids: &[String]) {
    if request_ids.is_empty() {
        return;
    }
    let identifiers = request_ids
        .iter()
        .map(|request_id| NSString::from_str(request_id))
        .collect::<Vec<_>>();
    let identifiers = NSArray::from_retained_slice(&identifiers);
    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);
    center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
}
