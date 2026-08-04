use std::{sync::mpsc, sync::Arc, time::Duration};

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObjectProtocol, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_foundation::{NSArray, NSError, NSObject, NSSet, NSString};
use objc2_user_notifications::{
    UNAlertStyle, UNAuthorizationOptions, UNAuthorizationStatus, UNNotification,
    UNNotificationAction, UNNotificationActionOptions, UNNotificationCategory,
    UNNotificationCategoryOptions, UNNotificationPresentationOptions, UNNotificationResponse,
    UNNotificationSetting, UNNotificationSettings, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use tauri::AppHandle;

use super::permission_request_result;

mod delivery;
pub(crate) use delivery::{
    remove, show, show_informational, show_with_completion_handler, show_with_error_handler,
};

#[derive(Clone, Debug)]
pub(crate) struct NotificationResponseAction {
    pub(crate) action_id: String,
    pub(crate) request_id: String,
}

pub(crate) struct NotificationActionSpec {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) foreground: bool,
}

pub(crate) struct NotificationCategorySpec {
    pub(crate) id: &'static str,
    pub(crate) actions: Vec<NotificationActionSpec>,
}

type ActionHandler = Arc<dyn Fn(NotificationResponseAction) + Send + Sync>;

struct NotificationDelegateIvars {
    handler: ActionHandler,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[ivars = NotificationDelegateIvars]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion_handler.call((UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::List
                | UNNotificationPresentationOptions::Sound,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion_handler: &DynBlock<dyn Fn()>,
        ) {
            let action_id = response.actionIdentifier().to_string();
            let request_id = response.notification().request().identifier().to_string();
            (self.ivars().handler)(NotificationResponseAction {
                action_id,
                request_id,
            });
            completion_handler.call(());
        }
    }
);

impl NotificationDelegate {
    fn new(handler: ActionHandler) -> Retained<Self> {
        let this = Self::alloc().set_ivars(NotificationDelegateIvars { handler });
        unsafe { msg_send![super(this), init] }
    }
}

pub(crate) fn initialize(
    categories: Vec<NotificationCategorySpec>,
    handler: impl Fn(NotificationResponseAction) + Send + Sync + 'static,
) {
    let center = UNUserNotificationCenter::currentNotificationCenter();
    register_categories(&center, categories);
    let delegate = NotificationDelegate::new(Arc::new(handler));
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // UNUserNotificationCenter's delegate property is weak. The delegate is
    // process-lifetime infrastructure, so retain it for the lifetime of the app.
    std::mem::forget(delegate);
}

pub(crate) fn authorization_status() -> Result<String, String> {
    query_settings(
        |settings| notification_status_label(settings.authorizationStatus()),
        "Timed out reading notification permission",
    )
}

pub(crate) fn automation_authorized() -> Result<bool, String> {
    query_settings(
        |settings| {
            settings.authorizationStatus() == UNAuthorizationStatus::Authorized
                && settings.alertSetting() == UNNotificationSetting::Enabled
                && settings.alertStyle() != UNAlertStyle::None
        },
        "Timed out reading notification automation permission",
    )
}

fn query_settings<T: Send + 'static>(
    project: impl Fn(&UNNotificationSettings) -> T + Send + Sync + 'static,
    timeout_message: &'static str,
) -> Result<T, String> {
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (sender, receiver) = mpsc::channel();
    let completion = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        let settings = unsafe { settings.as_ref() };
        let _ = sender.send(project(settings));
    });
    center.getNotificationSettingsWithCompletionHandler(&completion);
    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| timeout_message.to_string())
}

pub(crate) fn request_access(app: &AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || request_access_on_main(sender))
        .map_err(|err| {
            format!("Failed to request notification access on the main thread: {err}")
        })?;
    receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Timed out waiting for notification permission".to_string())?
}

fn request_access_on_main(sender: mpsc::Sender<Result<(), String>>) {
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let completion = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
        let error = unsafe { error.as_ref() }.map(|error| error.localizedDescription().to_string());
        let result = permission_request_result("notification", error);
        let _ = sender.send(result);
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
        &completion,
    );
    std::mem::forget(completion);
}

fn register_categories(
    center: &UNUserNotificationCenter,
    categories: Vec<NotificationCategorySpec>,
) {
    let no_intents = NSArray::<NSString>::from_slice(&[]);
    let categories = categories
        .into_iter()
        .map(|category| {
            let actions = category
                .actions
                .into_iter()
                .map(|action| {
                    let options = if action.foreground {
                        UNNotificationActionOptions::Foreground
                    } else {
                        UNNotificationActionOptions::empty()
                    };
                    UNNotificationAction::actionWithIdentifier_title_options(
                        &NSString::from_str(action.id),
                        &NSString::from_str(action.title),
                        options,
                    )
                })
                .collect::<Vec<_>>();
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(category.id),
                &NSArray::from_retained_slice(&actions),
                &no_intents,
                UNNotificationCategoryOptions::empty(),
            )
        })
        .collect::<Vec<_>>();
    center.setNotificationCategories(&NSSet::from_retained_slice(&categories));
}

fn notification_status_label(status: UNAuthorizationStatus) -> String {
    match status {
        UNAuthorizationStatus::Authorized | UNAuthorizationStatus::Provisional => "authorized",
        UNAuthorizationStatus::Denied => "denied",
        UNAuthorizationStatus::NotDetermined => "notDetermined",
        _ => "unknown",
    }
    .to_string()
}
