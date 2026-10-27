#[tauri::command]
fn save_svg(path: String, svg_content: String) -> Result<(), String> {
    std::fs::write(&path, svg_content).map_err(|e| e.to_string())
}

#[tauri::command]
fn convert_png_to_svg(png_data_url: String) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let data = png_data_url
        .strip_prefix("data:image/png;base64,")
        .or_else(|| png_data_url.strip_prefix("data:image/png; base64,"))
        .ok_or("Invalid PNG data URL")?;
    let bytes = general_purpose::STANDARD
        .decode(data)
        .map_err(|e| e.to_string())?;
    let raster = hector::Raster::from_bytes(&bytes, "upload.png").map_err(|e| e.to_string())?;
    let config = hector::TraceConfig::default();
    let (vector, _stats) = hector::trace(&raster, &config).map_err(|e| e.to_string())?;
    let svg = hector::to_svg(&vector, &hector::SvgOptions::default());
    Ok(svg)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![convert_png_to_svg, save_svg])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
