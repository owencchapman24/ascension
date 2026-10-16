mod commands;
pub mod domain;
pub mod service;
mod storage;

use tauri::Manager;

pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            app.manage(commands::AppState::open(app.path().app_data_dir().ok()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_workspace,
            commands::create_project,
            commands::update_project,
            commands::create_objective,
            commands::update_objective,
            commands::select_objective,
            commands::save_work,
            commands::export_workspace,
            commands::preview_import,
            commands::restore_import,
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        rfd::MessageDialog::new()
            .set_title("Ascension could not start")
            .set_description("The desktop window could not open. Your saved records have not been replaced. Check that WebView2 is installed, then reopen Ascension.")
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}
