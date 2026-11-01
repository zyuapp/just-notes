use crate::platform::microphone_activity::MicrophoneClient;

const WEBKIT_GPU: &str = "com.apple.WebKit.GPU";
const FACETIME_DAEMON: &str = "com.apple.avconferenced";
const SAFARI: &str = "com.apple.Safari";
const FACETIME: &str = "com.apple.FaceTime";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::meetings) enum CallKind {
    App,
    Browser,
}

/// A calling app or browser, named the same on every detection. The name is
/// also the key stored for apps the user chose to ignore.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::meetings) struct CallApp {
    pub(in crate::meetings) name: &'static str,
    pub(in crate::meetings) kind: CallKind,
    bundle_ids: &'static [&'static str],
    link_hosts: &'static [&'static str],
}

const fn app(
    name: &'static str,
    bundle_ids: &'static [&'static str],
    link_hosts: &'static [&'static str],
) -> CallApp {
    CallApp {
        name,
        kind: CallKind::App,
        bundle_ids,
        link_hosts,
    }
}

const fn browser(name: &'static str, bundle_ids: &'static [&'static str]) -> CallApp {
    CallApp {
        name,
        kind: CallKind::Browser,
        bundle_ids,
        link_hosts: &[],
    }
}

const KNOWN_APPS: &[CallApp] = &[
    app(
        "Zoom",
        &["us.zoom.xos", "us.zoom.CptHost"],
        &["zoom.us", "zoom.com", "zoomgov.com"],
    ),
    app(
        "Microsoft Teams",
        &["com.microsoft.teams2", "com.microsoft.teams"],
        &["teams.microsoft.com", "teams.live.com"],
    ),
    app("Slack", &["com.tinyspeck.slackmacgap"], &["slack.com"]),
    app("FaceTime", &[FACETIME], &["facetime.apple.com"]),
    app(
        "Webex",
        &[
            "Cisco-Systems.Spark",
            "com.cisco.webexmeetingsapp",
            "com.webex.meetingmanager",
        ],
        &["webex.com"],
    ),
    app("Discord", &["com.hnc.Discord"], &[]),
    browser("Google Chrome", &["com.google.Chrome"]),
    browser("Safari", &[SAFARI, "com.apple.SafariTechnologyPreview"]),
    browser("Arc", &["company.thebrowser.Browser"]),
    browser("Dia", &["company.thebrowser.dia"]),
    browser("Brave", &["com.brave.Browser"]),
    browser("Microsoft Edge", &["com.microsoft.edgemac"]),
    browser(
        "Firefox",
        &["org.mozilla.firefox", "org.mozilla.firefoxdeveloperedition"],
    ),
    browser("Opera", &["com.operasoftware.Opera"]),
    browser("Vivaldi", &["com.vivaldi.Vivaldi"]),
    browser("Orion", &["com.kagi.kagimacOS"]),
];

impl CallApp {
    pub(in crate::meetings) fn thread_title(&self) -> String {
        match (self.kind, self.name) {
            (CallKind::Browser, name) => format!("Call in {name}"),
            (CallKind::App, "Slack") => "Slack huddle".to_string(),
            (CallKind::App, name) => format!("{name} call"),
        }
    }

    pub(in crate::meetings) fn detected_title(&self) -> String {
        match self.kind {
            CallKind::Browser => format!("Call detected in {}", self.name),
            CallKind::App => format!("{} detected", self.thread_title()),
        }
    }

    /// Whether a calendar join link could open this call. A browser can open
    /// any web conferencing link.
    pub(in crate::meetings) fn opens_link_host(&self, host: &str) -> bool {
        self.kind == CallKind::Browser
            || self
                .link_hosts
                .iter()
                .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    }

    fn owns_bundle_id(&self, bundle_id: &str) -> bool {
        self.bundle_ids.iter().any(|known| {
            bundle_id.eq_ignore_ascii_case(known)
                || bundle_id
                    .get(..known.len() + 1)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&format!("{known}.")))
        })
    }
}

/// The calling app behind a process capturing the microphone, if it is one
/// Just Notes offers to record.
pub(in crate::meetings) fn classify(
    client: &MicrophoneClient,
    application_running: impl Fn(&str) -> bool,
) -> Option<CallApp> {
    let app_bundle_id = client.app.as_ref().map(|app| app.bundle_id.as_str());
    let audio_bundle_id = client.audio_bundle_id.as_deref();
    if let Some(known) = [app_bundle_id, audio_bundle_id]
        .into_iter()
        .flatten()
        .find_map(known_app)
    {
        return Some(known);
    }
    // Safari's media and FaceTime's audio run in system services whose parent
    // is launchd, so they are credited to the app only while it is running.
    match audio_bundle_id {
        Some(id) if id.eq_ignore_ascii_case(WEBKIT_GPU) && application_running(SAFARI) => {
            known_app(SAFARI)
        }
        Some(id) if id.eq_ignore_ascii_case(FACETIME_DAEMON) && application_running(FACETIME) => {
            known_app(FACETIME)
        }
        _ => None,
    }
}

fn known_app(bundle_id: &str) -> Option<CallApp> {
    KNOWN_APPS
        .iter()
        .find(|app| app.owns_bundle_id(bundle_id))
        .copied()
}

#[cfg(test)]
mod tests;
