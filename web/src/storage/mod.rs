use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::error::{AppError, Result};

/// Storage manager for uploaded files and transcoded outputs
pub struct StorageManager {
    uploads_dir: PathBuf,
    outputs_dir: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager and ensure directories exist
    pub async fn new(config: &Config) -> Result<Self> {
        let uploads_dir = config.uploads_dir();
        let outputs_dir = config.outputs_dir();

        fs::create_dir_all(&uploads_dir)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create uploads dir: {}", e)))?;

        fs::create_dir_all(&outputs_dir)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create outputs dir: {}", e)))?;

        info!("Storage initialized: uploads={}, outputs={}",
              uploads_dir.display(), outputs_dir.display());

        Ok(Self {
            uploads_dir,
            outputs_dir,
        })
    }

    /// Get the upload directory for a specific job
    pub fn job_upload_dir(&self, job_id: &str) -> PathBuf {
        self.uploads_dir.join(job_id)
    }

    /// Get the output file path for a job
    pub fn job_output_path(&self, job_id: &str, extension: &str) -> PathBuf {
        self.outputs_dir.join(format!("{}.{}", job_id, extension))
    }

    /// Get the output directory for demucs stems
    pub async fn job_demucs_output_dir(&self, job_id: &str) -> Result<PathBuf> {
        let dir = self.outputs_dir.join(format!("{}_stems", job_id));
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create demucs output dir: {}", e)))?;
        Ok(dir)
    }

    /// Store an uploaded file for a job
    pub async fn store_upload(
        &self,
        job_id: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<PathBuf> {
        let job_dir = self.job_upload_dir(job_id);
        fs::create_dir_all(&job_dir)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to create job dir: {}", e)))?;

        // Sanitize filename
        let safe_filename = sanitize_filename::sanitize(filename);
        let path = job_dir.join(&safe_filename);

        fs::write(&path, data)
            .await
            .map_err(|e| AppError::Storage(format!("Failed to write file: {}", e)))?;

        debug!("Stored upload: {} ({} bytes)", path.display(), data.len());

        Ok(path)
    }

    /// Check if an output file exists
    pub async fn output_exists(&self, job_id: &str, extension: &str) -> bool {
        let path = self.job_output_path(job_id, extension);
        fs::metadata(&path).await.is_ok()
    }

    /// Get the size of an output file
    pub async fn output_size(&self, job_id: &str, extension: &str) -> Result<u64> {
        let path = self.job_output_path(job_id, extension);
        let metadata = fs::metadata(&path)
            .await
            .map_err(|_| AppError::FileNotFound(path.display().to_string()))?;
        Ok(metadata.len())
    }

    /// Clean up files for a job
    pub async fn cleanup_job(&self, job_id: &str) -> Result<()> {
        // Remove upload directory
        let upload_dir = self.job_upload_dir(job_id);
        if fs::metadata(&upload_dir).await.is_ok() {
            fs::remove_dir_all(&upload_dir)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to remove upload dir: {}", e)))?;
            debug!("Cleaned up upload dir: {}", upload_dir.display());
        }

        Ok(())
    }

    /// Clean up output file for a job
    pub async fn cleanup_output(&self, job_id: &str, extension: &str) -> Result<()> {
        let output_path = self.job_output_path(job_id, extension);
        if fs::metadata(&output_path).await.is_ok() {
            fs::remove_file(&output_path)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to remove output: {}", e)))?;
            debug!("Cleaned up output: {}", output_path.display());
        }

        Ok(())
    }

    /// Clean up demucs output directory for a job
    pub async fn cleanup_demucs_output(&self, job_id: &str) -> Result<()> {
        let output_dir = self.outputs_dir.join(format!("{}_stems", job_id));
        if fs::metadata(&output_dir).await.is_ok() {
            fs::remove_dir_all(&output_dir)
                .await
                .map_err(|e| AppError::Storage(format!("Failed to remove demucs output dir: {}", e)))?;
            debug!("Cleaned up demucs output dir: {}", output_dir.display());
        }

        Ok(())
    }

    /// Get the path to the outputs directory (for serving files)
    pub fn outputs_dir(&self) -> &Path {
        &self.outputs_dir
    }
}

/// Clean up old files based on retention policy
pub async fn cleanup_old_files(storage: &StorageManager, retention_hours: u64) {
    if retention_hours == 0 {
        return; // Keep forever
    }

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(retention_hours * 3600);

    // Clean up old output files
    if let Ok(mut entries) = fs::read_dir(storage.outputs_dir()).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        if let Err(e) = fs::remove_file(entry.path()).await {
                            warn!("Failed to remove old file {}: {}", entry.path().display(), e);
                        } else {
                            info!("Cleaned up old file: {}", entry.path().display());
                        }
                    }
                }
            }
        }
    }
}
