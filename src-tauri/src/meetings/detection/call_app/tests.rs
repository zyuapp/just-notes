use super::{classify, CallKind};
use crate::platform::microphone_activity::{MicrophoneClient, RunningApp};

fn client(audio_bundle_id: Option<&str>, app: Option<&str>) -> MicrophoneClient {
    MicrophoneClient {
        pid: 42,
        audio_bundle_id: audio_bundle_id.map(str::to_string),
        app: app.map(|bundle_id| RunningApp {
            bundle_id: bundle_id.to_string(),
            name: bundle_id.to_string(),
        }),
    }
}

fn name(client: &MicrophoneClient) -> Option<&'static str> {
    classify(client, |_| false).map(|app| app.name)
}

#[test]
fn helpers_resolve_to_their_host_app() {
    assert_eq!(
        name(&client(
            Some("com.google.Chrome.helper"),
            Some("com.google.Chrome")
        )),
        Some("Google Chrome")
    );
    assert_eq!(
        name(&client(Some("com.tinyspeck.slackmacgap.helper"), None)),
        Some("Slack")
    );
    assert_eq!(name(&client(Some("us.zoom.CptHost"), None)), Some("Zoom"));
    assert_eq!(
        name(&client(
            Some("org.mozilla.plugincontainer"),
            Some("org.mozilla.firefox")
        )),
        Some("Firefox")
    );
}

#[test]
fn similar_bundle_ids_do_not_cross_match() {
    assert_eq!(
        name(&client(Some("com.microsoft.teams2"), None)),
        Some("Microsoft Teams")
    );
    assert_eq!(name(&client(Some("com.google.Chromecast"), None)), None);
    assert_eq!(name(&client(Some("com.apple.VoiceMemos"), None)), None);
}

#[test]
fn system_media_services_need_their_app_running() {
    let webkit = client(Some("com.apple.WebKit.GPU"), None);
    assert_eq!(name(&webkit), None);
    assert_eq!(
        classify(&webkit, |bundle_id| bundle_id == "com.apple.Safari").map(|app| app.kind),
        Some(CallKind::Browser)
    );
    let facetime = client(Some("com.apple.avconferenced"), None);
    assert_eq!(
        classify(&facetime, |bundle_id| bundle_id == "com.apple.FaceTime").map(|app| app.name),
        Some("FaceTime")
    );
}

#[test]
fn titles_name_the_app_or_browser() {
    let zoom = classify(&client(Some("us.zoom.xos"), None), |_| false).unwrap();
    assert_eq!(zoom.thread_title(), "Zoom call");
    assert_eq!(zoom.detected_title(), "Zoom call detected");
    let slack = classify(&client(Some("com.tinyspeck.slackmacgap"), None), |_| false).unwrap();
    assert_eq!(slack.thread_title(), "Slack huddle");
    let arc = classify(&client(None, Some("company.thebrowser.Browser")), |_| false).unwrap();
    assert_eq!(arc.thread_title(), "Call in Arc");
    assert_eq!(arc.detected_title(), "Call detected in Arc");
}

#[test]
fn link_hosts_match_the_app_or_any_host_for_browsers() {
    let zoom = classify(&client(Some("us.zoom.xos"), None), |_| false).unwrap();
    assert!(zoom.opens_link_host("us02web.zoom.us"));
    assert!(!zoom.opens_link_host("meet.google.com"));
    assert!(!zoom.opens_link_host("notzoom.us"));
    let chrome = classify(&client(Some("com.google.Chrome"), None), |_| false).unwrap();
    assert!(chrome.opens_link_host("meet.google.com"));
}
