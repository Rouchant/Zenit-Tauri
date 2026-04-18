use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime, WebviewWindow};
use serde::{Serialize, Deserialize};
use crate::state::{get_user_data_dir, chrono_millis};

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoMeta {
    pub path: String,
    pub name: String,
}

pub fn load_video_meta(app: &AppHandle) -> Vec<VideoMeta> {
    let meta_path = get_user_data_dir(app).join("custom-videos").join("meta.json");
    if let Ok(data) = fs::read_to_string(&meta_path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn save_video_meta(app: &AppHandle, meta: &Vec<VideoMeta>) {
    let meta_path = get_user_data_dir(app).join("custom-videos").join("meta.json");
    if let Ok(data) = serde_json::to_string_pretty(meta) {
        let _ = fs::write(&meta_path, data);
    }
}

#[tauri::command]
pub async fn select_video<R: Runtime>(window: WebviewWindow<R>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};
    let (tx, rx) = std::sync::mpsc::channel();

    window
        .dialog()
        .file()
        .add_filter("Videos", &["mp4"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });

    let result = rx.recv().map_err(|e| e.to_string())?;
    match result {
        Some(FilePath::Path(p)) => Ok(Some(p.to_string_lossy().into_owned())),
        _ => Ok(None),
    }
}

#[tauri::command]
pub async fn save_custom_video(app: AppHandle, source_path: String, custom_name: Option<String>) -> Result<Option<String>, String> {
    let src = PathBuf::from(&source_path);
    if !src.exists() {
        return Ok(None);
    }

    let custom_dir = get_user_data_dir(&app).join("custom-videos");
    fs::create_dir_all(&custom_dir).map_err(|e| format!("Error creando directorio: {}", e))?;

    let metadata = fs::metadata(&src).map_err(|e| e.to_string())?;
    let file_size = metadata.len();
    let original_name = src.file_name().unwrap_or_default().to_string_lossy();

    // 1. Detección de Duplicados (Heurística: Nombre de archivo original y Tamaño)
    let meta = load_video_meta(&app);
    for entry in &meta {
        let entry_path = PathBuf::from(&entry.path);
        if entry_path.exists() {
            if let Ok(entry_meta) = fs::metadata(&entry_path) {
                // Si el tamaño coincide y el nombre del archivo contiene el nombre original
                // (Los archivos guardados tienen formato {timestamp}_{original_name})
                if entry_meta.len() == file_size && entry_path.to_string_lossy().contains(&*original_name) {
                    return Ok(Some(entry.path.clone()));
                }
            }
        }
    }

    // 2. Validar límite migración/capacidad (Máximo 5 videos) - Solo para NUEVOS videos
    if meta.len() >= 5 {
        return Err("La bóveda está llena (máximo 5 videos). Elimina uno para continuar.".to_string());
    }

    // 3. Validar tamaño (Máximo 40MB = 41,943,040 bytes)
    if file_size > 41_943_040 {
        return Err("El video supera el límite de 40MB permitido.".to_string());
    }

    // 4. Validar nombre alfanumérico si se proporciona
    if let Some(ref n) = custom_name {
        if !n.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') {
            return Err("El nombre solo puede contener letras, números, espacios, guiones o guiones bajos.".to_string());
        }
    }

    let file_ext = src.extension().and_then(|s| s.to_str()).unwrap_or("mp4");

    let clean_name = if let Some(n) = &custom_name { n.clone() } else { src.file_stem().unwrap_or_default().to_string_lossy().into_owned() };
    
    let new_name = if custom_name.is_some() {
        let safe_name = clean_name.replace(|c: char| !c.is_alphanumeric() && c != ' ' && c != '-', "_");
        format!("{}_{}.{}", safe_name, chrono_millis() % 10000, file_ext) 
    } else {
        format!("{}_{}", chrono_millis(), original_name)
    };

    let dest = custom_dir.join(&new_name);

    fs::copy(&src, &dest).map_err(|e| format!("Error copiando video: {}", e))?;
    
    let dest_str = dest.to_string_lossy().into_owned();
    let mut new_meta = meta;
    new_meta.push(VideoMeta { path: dest_str.clone(), name: clean_name });
    save_video_meta(&app, &new_meta);

    Ok(Some(dest_str))
}

#[tauri::command]
pub async fn list_custom_videos(app: AppHandle) -> Result<Vec<VideoMeta>, String> {
    let custom_dir = get_user_data_dir(&app).join("custom-videos");
    if !custom_dir.exists() {
        return Ok(Vec::new());
    }

    let mut videos = Vec::new();
    let meta = load_video_meta(&app);
    
    if let Ok(entries) = fs::read_dir(custom_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() && entry.file_name() != "meta.json" {
                    let path = entry.path().to_string_lossy().into_owned();
                    let name = meta.iter().find(|m| m.path == path).map(|m| m.name.clone()).unwrap_or_else(|| {
                        entry.path().file_stem().unwrap_or_default().to_string_lossy().into_owned()
                    });
                    videos.push(VideoMeta { path, name });
                }
            }
        }
    }
    Ok(videos)
}

#[tauri::command]
pub async fn delete_custom_video(app: AppHandle, path: String) -> Result<(), String> {
    let file = PathBuf::from(&path);
    if file.exists() {
        fs::remove_file(file).map_err(|e| format!("Error eliminando video: {}", e))?;
    }
    
    let mut meta = load_video_meta(&app);
    meta.retain(|m| m.path != path);
    save_video_meta(&app, &meta);
    
    Ok(())
}

#[tauri::command]
pub async fn rename_custom_video(app: AppHandle, path: String, new_name: String) -> Result<(), String> {
    let mut meta = load_video_meta(&app);
    if let Some(m) = meta.iter_mut().find(|m| m.path == path) {
        m.name = new_name;
    } else {
        meta.push(VideoMeta { path, name: new_name });
    }
    save_video_meta(&app, &meta);
    Ok(())
}


#[tauri::command]
pub fn check_file_exists(file_path: String) -> bool {
    PathBuf::from(file_path).exists()
}

/// Borra archivos físicos que no estén en meta.json
pub fn cleanup_orphan_videos(app: &AppHandle) {
    let custom_dir = get_user_data_dir(app).join("custom-videos");
    if !custom_dir.exists() { return; }
    
    let meta = load_video_meta(app);
    let meta_paths: Vec<PathBuf> = meta.iter().map(|m| PathBuf::from(&m.path)).collect();

    if let Ok(entries) = fs::read_dir(custom_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // No borrar meta.json ni carpetas
            if path.is_file() && path.file_name().unwrap_or_default() != "meta.json" {
                // Si la ruta del archivo no está en nuestro catálogo de metadatos, es un huérfano
                if !meta_paths.contains(&path) {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}
