use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Manager};

use crate::{
    app::{migration::legacy_custom_threads_path, AppPaths, StorageGate},
    platform::{self, FolderChoice},
    recording::RecorderState,
    threads::import::{
        plan_legacy_import, LegacyImportPlan, LegacyImportPreviewData, LegacyImportResultData,
    },
    transcription::FinalizeState,
};

struct PendingLegacyImport {
    plan: LegacyImportPlan,
    _source_access: FolderChoice,
    _custom_access: Option<FolderChoice>,
}

type PendingImportGuard<'a> = std::sync::MutexGuard<'a, Option<PendingLegacyImport>>;

#[derive(Clone, Default)]
pub(crate) struct LegacyImportState(Arc<Mutex<Option<PendingLegacyImport>>>);

impl LegacyImportState {
    fn replace(&self, pending: PendingLegacyImport) -> Result<(), String> {
        *self.lock()? = Some(pending);
        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), String> {
        *self.lock()? = None;
        Ok(())
    }

    fn take_for(&self, paths: &AppPaths) -> Result<PendingLegacyImport, String> {
        let mut pending = self.lock()?;
        let session = pending
            .as_ref()
            .ok_or_else(|| "Review the legacy import before confirming it".to_string())?;
        if !session.plan.matches_destination(paths) {
            return Err("The import destination changed. Review the import again".to_string());
        }
        Ok(pending.take().expect("pending import checked above"))
    }

    fn lock(&self) -> Result<PendingImportGuard<'_>, String> {
        self.0
            .lock()
            .map_err(|_| "The legacy import session is unavailable".to_string())
    }
}

pub(crate) async fn prepare(
    app: AppHandle,
    paths: AppPaths,
    session: LegacyImportState,
) -> Result<Option<LegacyImportPreviewData>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        session.clear()?;
        let home = app
            .path()
            .home_dir()
            .map_err(|error| format!("Could not locate your home folder: {error}"))?;
        let Some(source_access) = platform::choose_folder(
            &app,
            "Select the hidden .just-notes folder from the previous version",
            true,
            Some(&home),
            Some("Grant Access"),
        )?
        else {
            return Ok(None);
        };
        let source = Path::new(&source_access.path);
        let custom_access =
            choose_custom_folder(&app, legacy_custom_threads_path(source)?.as_deref())?;
        let plan = plan_legacy_import(
            source,
            custom_access.as_ref().map(|choice| Path::new(&choice.path)),
            &paths,
        )?;
        let preview = plan.preview();
        session.replace(PendingLegacyImport {
            plan,
            _source_access: source_access,
            _custom_access: custom_access,
        })?;
        Ok(Some(preview))
    })
    .await
    .map_err(|error| format!("Legacy import preview task failed: {error}"))?
}

pub(crate) async fn confirm(
    app: AppHandle,
    paths: AppPaths,
    recorder: RecorderState,
    finalize: FinalizeState,
    session: LegacyImportState,
) -> Result<LegacyImportResultData, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<StorageGate>().run(|| {
            if recorder.is_busy() || finalize.is_active() {
                return Err(
                    "Previous recordings can be imported after recording and transcription finish"
                        .to_string(),
                );
            }
            session.take_for(&paths)?.plan.execute()
        })
    })
    .await
    .map_err(|error| format!("Legacy import task failed: {error}"))?
}

fn choose_custom_folder(
    app: &AppHandle,
    custom_path: Option<&Path>,
) -> Result<Option<FolderChoice>, String> {
    let Some(custom_path) = custom_path else {
        return Ok(None);
    };
    let prompt = format!(
        "The previous version stored transcripts in {}. Select that folder to continue",
        custom_path.display()
    );
    platform::choose_folder(
        app,
        &prompt,
        false,
        Some(custom_path.parent().unwrap_or(custom_path)),
        Some("Grant Access"),
    )?
    .ok_or_else(|| "The import needs access to the previous custom transcripts folder".to_string())
    .map(Some)
}
