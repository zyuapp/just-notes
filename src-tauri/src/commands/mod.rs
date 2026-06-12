pub(crate) mod indicator;
pub(crate) mod recording;
pub(crate) mod settings;
pub(crate) mod system;
pub(crate) mod threads;

use crate::{app::AppPaths, settings::SettingsState};

fn effective_paths(paths: &AppPaths, settings: &SettingsState) -> AppPaths {
    crate::settings::effective_paths(paths, &settings.snapshot())
}
