use crate::klwp_import::{self, ImportResult};
use crate::project::Project;
use std::fs;
use std::path::Path;

#[tauri::command]
pub fn new_project(width: u32, height: u32) -> Project {
    Project::new(width, height)
}

#[tauri::command]
pub fn save_project(path: String, project: Project) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_project(path: String) -> Result<Project, String> {
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_klwp(path: String, target_width: u32, target_height: u32) -> Result<ImportResult, String> {
    // Create output directory next to the klwp file
    let klwp_path = Path::new(&path);
    let stem = klwp_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    let output_dir = klwp_path.parent()
        .unwrap_or(Path::new("."))
        .join(format!("{}_assets", stem));

    fs::create_dir_all(&output_dir).map_err(|e| format!("Cannot create output dir: {}", e))?;

    klwp_import::import_klwp_file(
        &path,
        output_dir.to_str().unwrap_or("."),
        target_width,
        target_height,
    )
}

#[tauri::command]
pub fn resolve_asset(path: String, asset_dir: String) -> String {
    klwp_import::resolve_asset_path(&path, &asset_dir)
}
