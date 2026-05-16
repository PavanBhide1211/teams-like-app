// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // ---- System tray ----
            let quit_i  = MenuItem::with_id(app, "quit",  "Quit Cowork Chat", true, None::<&str>)?;
            let show_i  = MenuItem::with_id(app, "show",  "Show window",      true, None::<&str>)?;
            let menu    = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("cowork-tray")
                .tooltip("Cowork Chat")
                .menu(&menu)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "quit" => { app.exit(0); }
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    // Left-click brings the main window forward.
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = ev {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running Cowork Chat");
}
