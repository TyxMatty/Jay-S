//! Jay-S's window. No tray: nothing runs in the background, and closing the window also stops Jay-AI
//! (docs/ARQUITETURA.md, section 2).

// Prevents an additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod library;
mod study;
#[cfg(windows)]
mod taskbar_icon;

use tauri::{DragDropEvent, Manager, WindowEvent};

fn main() {
    tauri::Builder::default()
        // First plugin: a second launch exits at once, shows the running Jay-S and hands it its files.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            library::import(app, library::paths_from_args(&args, cwd.as_ref()));
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The student's data: %APPDATA%\io.github.tyxmatty.jays (docs/ARQUITETURA.md, section 6).
            let root = app.path().app_data_dir()?;
            app.manage(ai::Ai::new(&root));
            app.manage(library::Library::new(jay_study::Store::open(root)?));
            app.manage(study::Jobs::default());
            ai::spawn_idle_watch(app.handle().clone());
            let args: Vec<String> = std::env::args().collect();
            let cwd = std::env::current_dir().unwrap_or_default();
            app.manage(library::StartupFiles(library::paths_from_args(&args, &cwd).into()));
            // An app, not a browser: no right-click menu and no browser shortcuts (F5, Ctrl+R, Ctrl+P,
            // Ctrl+F, F12, Alt+Left…), which Tauri's config doesn't expose. Dev builds keep them, for
            // Inspect and reload.
            #[cfg(all(windows, not(debug_assertions)))]
            if let Some(window) = app.get_webview_window("main") {
                window.with_webview(|webview| unsafe {
                    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
                    use windows::core::Interface;
                    if let Ok(settings) = webview.controller().CoreWebView2().and_then(|w| w.Settings()) {
                        let _ = settings.SetAreDefaultContextMenusEnabled(false);
                        if let Ok(settings) = settings.cast::<ICoreWebView2Settings3>() {
                            let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
                        }
                    }
                })?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // A drop on the window: Rust gets the paths, the page only hears about the materials.
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                library::import(window.app_handle(), paths.clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            library::materials_list,
            library::materials_pick,
            library::materials_set_kind,
            library::materials_text,
            library::materials_delete,
            ai::ai_status,
            ai::ai_set_choice,
            ai::ai_install,
            ai::ai_cancel,
            ai::ai_remove_model,
            ai::ai_stop,
            ai::ai_test,
            study::styles_list,
            study::exam_material_info,
            study::exam_build,
            study::exams_list,
            study::exam_delete,
            study::attempt_start,
            study::attempt_resume,
            study::attempt_answer,
            study::attempt_finish,
            study::attempt_analyze,
            study::attempts_list,
            study::attempt_result,
            study::attempt_delete,
            study::profile_get,
            study::decks_list,
            study::deck_build,
            study::deck_due,
            study::card_review,
            study::card_vote,
            study::deck_delete,
            study::job_cancel,
            study::explain_get,
            study::explain_build,
            study::explain_check,
            study::explain_ask,
            #[cfg(windows)]
            taskbar_icon::set_taskbar_icon
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jay-S");
}
