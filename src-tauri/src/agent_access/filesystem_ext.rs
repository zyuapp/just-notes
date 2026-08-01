use rustix::fs::fsync;

use super::filesystem::SafeDirectory;

impl SafeDirectory {
    pub(super) fn type_error(&self, name: &str, expected: &str) -> String {
        format!("{}/{} is not {expected}", self.path.display(), name)
    }

    pub(super) fn sync(&self) -> Result<(), String> {
        fsync(&self.fd).map_err(|error| format!("Failed to sync {}: {error}", self.path.display()))
    }
}
