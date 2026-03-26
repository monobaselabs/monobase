use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Set up the system tray icon and menu
pub fn setup_system_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Monobase Patient")
        .on_menu_event(move |app, event| {
            handle_tray_menu_event(app, event);
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Build the tray menu
fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;

    let show_item = MenuItem::with_id(app, "show_window", "Show Window", true, None::<&str>)?;
    menu.append(&show_item)?;

    let check_updates_item = MenuItem::with_id(app, "check_updates", "Check for Updates", true, None::<&str>)?;
    menu.append(&check_updates_item)?;

    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    menu.append(&separator)?;

    let quit_item = tauri::menu::PredefinedMenuItem::quit(app, Some("Quit"))?;
    menu.append(&quit_item)?;

    Ok(menu)
}

/// Handle tray menu events
fn handle_tray_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show_window" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "check_updates" => {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::check_for_updates(app_clone, true).await;
            });
        }
        _ => {}
    }
}
