use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::{QPawError, QPawResult};
use crate::models::{AvatarKind, AvatarManifest};

pub struct AvatarStore {
    root: PathBuf,
}

impl AvatarStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn import_avatar(&self, source: PathBuf) -> QPawResult<AvatarManifest> {
        fs::create_dir_all(&self.root)?;
        let id = Uuid::new_v4().to_string();
        let target_dir = self.root.join(&id);

        if is_supported_image(&source) {
            fs::create_dir_all(&target_dir)?;
            let file_name = source
                .file_name()
                .ok_or_else(|| QPawError::Message("image file has no file name".to_string()))?;
            let target_image = target_dir.join(file_name);
            fs::copy(&source, &target_image)?;
            let path = target_image.to_string_lossy().to_string();
            let name = source
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Static Avatar")
                .to_string();

            return Ok(AvatarManifest {
                id,
                name,
                kind: AvatarKind::Image,
                path: path.clone(),
                model_json_path: None,
                image_path: Some(path),
                imported_at: Utc::now(),
            });
        }

        let model_file = resolve_model_json(&source)?;
        let model_dir = model_file
            .parent()
            .ok_or_else(|| QPawError::Message("model file has no parent directory".to_string()))?;
        copy_dir(model_dir, &target_dir)?;

        let relative_model = model_file.strip_prefix(model_dir).map_err(|_| {
            QPawError::Message("failed to calculate imported model path".to_string())
        })?;
        let target_model = target_dir.join(relative_model);
        let path = target_model.to_string_lossy().to_string();
        let name = model_file
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Live2D Avatar")
            .to_string();

        Ok(AvatarManifest {
            id,
            name,
            kind: AvatarKind::Live2d,
            path: path.clone(),
            model_json_path: Some(path),
            image_path: None,
            imported_at: Utc::now(),
        })
    }
}

fn is_supported_image(source: &Path) -> bool {
    source.is_file()
        && source
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "webp"
                )
            })
            .unwrap_or(false)
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
        "请选择 Live2D Cubism 的 .model3.json 文件，或 png/jpg/jpeg/webp 静态图片".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("qpaw-avatar-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn imports_static_image_as_image_avatar() {
        let source_dir = temp_path("source-image");
        let target_dir = temp_path("target-image");
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("avatar.png");
        fs::write(&source, b"image bytes").unwrap();

        let store = AvatarStore::new(target_dir.clone());
        let manifest = store.import_avatar(source).unwrap();

        assert_eq!(manifest.kind, AvatarKind::Image);
        assert!(manifest.model_json_path.is_none());
        assert!(manifest
            .image_path
            .as_deref()
            .unwrap()
            .ends_with("avatar.png"));
        assert!(PathBuf::from(manifest.image_path.unwrap()).exists());

        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(target_dir);
    }

    #[test]
    fn imports_model3_json_as_live2d_avatar() {
        let source_dir = temp_path("source-live2d");
        let target_dir = temp_path("target-live2d");
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("pet.model3.json");
        fs::write(&source, "{}").unwrap();

        let store = AvatarStore::new(target_dir.clone());
        let manifest = store.import_avatar(source).unwrap();

        assert_eq!(manifest.kind, AvatarKind::Live2d);
        assert!(manifest.image_path.is_none());
        assert!(manifest
            .model_json_path
            .as_deref()
            .unwrap()
            .ends_with("pet.model3.json"));
        assert!(PathBuf::from(manifest.model_json_path.unwrap()).exists());

        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(target_dir);
    }
}
