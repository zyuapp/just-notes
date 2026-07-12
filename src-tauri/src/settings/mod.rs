mod model;
mod store;

pub(crate) use model::{AppPreferencesUpdate, AppSettings, PersistedSettings};
pub(crate) use store::{
    effective_paths, load_persisted_settings, save_persisted_settings, validate_settings,
    SettingsState,
};
