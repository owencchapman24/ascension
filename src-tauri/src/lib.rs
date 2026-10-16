use serde::Serialize;

#[derive(Serialize)]
struct AppStatus {
    app_name: &'static str,
    schema_version: u32,
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus {
        app_name: "Ascension",
        schema_version: 1,
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("Ascension could not start");
}
