use crate::klwp_import::{self, ImportResult};
use crate::project::Project;
use std::fs;
use std::io;
use std::path::Path;

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

/// List font files in a project's asset directory
#[tauri::command]
pub fn list_project_fonts(asset_dir: String) -> Result<Vec<String>, String> {
    let fonts_dir = Path::new(&asset_dir).join("fonts");
    if !fonts_dir.exists() {
        return Ok(vec![]);
    }
    let mut fonts = Vec::new();
    let entries = fs::read_dir(&fonts_dir)
        .map_err(|e| format!("Failed to read fonts dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "ttf" | "otf" | "woff" | "woff2" => {
                    fonts.push(path.to_string_lossy().to_string());
                }
                _ => {}
            }
        }
    }
    Ok(fonts)
}

/// Copy a file into the project's asset directory under a subfolder
#[tauri::command]
pub fn copy_asset_to_project(
    source_path: String,
    asset_dir: String,
    subfolder: String,
) -> Result<String, String> {
    let target_dir = Path::new(&asset_dir).join(&subfolder);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create dir: {}", e))?;

    let filename = Path::new(&source_path)
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy()
        .to_string();

    let target_path = target_dir.join(&filename);
    fs::copy(&source_path, &target_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Write an icon file (SVG content) to the project's icons directory
#[tauri::command]
pub fn write_icon_file(asset_dir: String, filename: String, content: String) -> Result<String, String> {
    let icons_dir = std::path::Path::new(&asset_dir).join("icons");
    std::fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Failed to create icons dir: {}", e))?;

    let file_path = icons_dir.join(&filename);
    std::fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write icon file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Extract the launcher icon from an APK file
#[tauri::command]
pub fn extract_apk_icon(apk_path: String, asset_dir: String) -> Result<String, String> {
    let file = fs::File::open(&apk_path)
        .map_err(|e| format!("Failed to open APK: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read APK as ZIP: {}", e))?;

    // Search for launcher icon in priority order
    let icon_patterns = [
        "res/mipmap-xxxhdpi/ic_launcher.png",
        "res/mipmap-xxxhdpi/ic_launcher_round.png",
        "res/mipmap-xxhdpi/ic_launcher.png",
        "res/mipmap-xxhdpi/ic_launcher_round.png",
        "res/mipmap-xhdpi/ic_launcher.png",
        "res/drawable-xxxhdpi/ic_launcher.png",
        "res/drawable-xxhdpi/ic_launcher.png",
    ];

    let mut found_name: Option<String> = None;

    // Try exact matches first
    for pattern in &icon_patterns {
        if archive.by_name(pattern).is_ok() {
            found_name = Some(pattern.to_string());
            break;
        }
    }

    // Fallback: search for any ic_launcher PNG
    if found_name.is_none() {
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name.contains("ic_launcher") && name.ends_with(".png") && !name.contains("xml")
                {
                    found_name = Some(name);
                    break;
                }
            }
        }
    }

    let icon_name = found_name.ok_or("No launcher icon found in APK")?;

    // Extract the icon
    let icons_dir = Path::new(&asset_dir).join("icons");
    fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Failed to create icons dir: {}", e))?;

    // Name the output after the APK
    let apk_stem = Path::new(&apk_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_path = icons_dir.join(format!("{}.png", apk_stem));

    let mut icon_file = archive
        .by_name(&icon_name)
        .map_err(|e| format!("Failed to read icon from APK: {}", e))?;
    let mut output = fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    io::copy(&mut icon_file, &mut output)
        .map_err(|e| format!("Failed to write icon: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

