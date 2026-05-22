use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::{QPawError, QPawResult};
use crate::models::AvatarManifest;

pub struct AvatarStore {
    root: PathBuf,
}

impl AvatarStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn import_model(&self, source: PathBuf) -> QPawResult<AvatarManifest> {
        let model_file = resolve_model_json(&source)?;
        let model_dir = model_file
            .parent()
            .ok_or_else(|| QPawError::Message("model file has no parent directory".to_string()))?;

        fs::create_dir_all(&self.root)?;
        let id = Uuid::new_v4().to_string();
        let target_dir = self.root.join(&id);
        copy_dir(model_dir, &target_dir)?;

        let relative_model = model_file.strip_prefix(model_dir).map_err(|_| {
            QPawError::Message("failed to calculate imported model path".to_string())
        })?;
        let target_model = target_dir.join(relative_model);
        let name = model_file
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Live2D Avatar")
            .to_string();

        Ok(AvatarManifest {
            id,
            name,
            model_json_path: target_model.to_string_lossy().to_string(),
            imported_at: Utc::now(),
        })
    }
}

fn resolve_model_json(source: &Path) -> QPawResult<PathBuf> {
    if source.is_file()
        && source
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.ends_with(".model3.json"))
            .unwrap_or(false)
    {
        return Ok(source.to_path_buf());
    }

    if source.is_dir() {
        for entry in WalkDir::new(source)
            .max_depth(4)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.ends_with(".model3.json"))
                .unwrap_or(false)
            {
                return Ok(path.to_path_buf());
            }
        }
    }

    Err(QPawError::Message(
        "请选择 Live2D Cubism 的 .model3.json 文件".to_string(),
    ))
}

fn copy_dir(source: &Path, target: &Path) -> QPawResult<()> {
    fs::create_dir_all(target)?;
    for entry in WalkDir::new(source).into_iter().filter_map(Result::ok) {
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(source)
            .map_err(|_| QPawError::Message("failed to copy avatar files".to_string()))?;
        let target_path = target.join(relative);

        if source_path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else if source_path.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}
