use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::backend::settings::read_app_settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmersiveWindowState {
    pub position: Option<(i32, i32)>,
    pub size: Option<(u32, u32)>,
    pub always_on_top: bool,
    pub sidebar_visible: bool,
}

impl Default for ImmersiveWindowState {
    fn default() -> Self {
        Self {
            position: None,
            size: Some((1200, 800)),
            always_on_top: false,
            sidebar_visible: false,
        }
    }
}

pub struct ImmersiveWindowManager {
    window_label: String,
    state: Mutex<ImmersiveWindowState>,
    main_window_state: Mutex<Option<MainWindowState>>,
}

#[derive(Debug, Clone)]
struct MainWindowState {
    visible: bool,
    minimized: bool,
}

impl ImmersiveWindowManager {
    pub fn new() -> Self {
        Self {
            window_label: "immersive".to_string(),
            state: Mutex::new(ImmersiveWindowState::default()),
            main_window_state: Mutex::new(None),
        }
    }

    pub fn open<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        // Check if window already exists
        if let Some(window) = app.get_webview_window(&self.window_label) {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            info!("Immersive window already exists, focusing");
            return Ok(());
        }

        // Read settings
        let settings = read_app_settings(app).unwrap_or_default();

        let state = self.state.lock().unwrap().clone();
        let (width, height) = state.size.unwrap_or((1200, 800));

        // Use saved always_on_top if we have a previous window state
        let always_on_top = if state.position.is_some() {
            state.always_on_top
        } else {
            settings.immersive_always_on_top
        };
        // Create window builder
        let mut builder = WebviewWindowBuilder::new(
            app,
            &self.window_label,
            WebviewUrl::App("immersive.html".into()),
        )
        .title("Immersive Reading")
        .inner_size(width as f64, height as f64)
        .resizable(true)
        .decorations(true)
        .always_on_top(always_on_top)
        .visible(true);

        // Set position
        if let Some((x, y)) = state.position {
            builder = builder.position(x as f64, y as f64);
        } else {
            // Calculate bottom-right position with 20px margin
            if let Some(monitor) = app.primary_monitor().ok().flatten() {
                let screen_size = monitor.size();
                let x = (screen_size.width.saturating_sub(width).saturating_sub(20)) as f64;
                let y = (screen_size.height.saturating_sub(height).saturating_sub(20)) as f64;
                builder = builder.position(x, y);
            }
        }

        // Build window
        let _window = builder.build().map_err(|e| e.to_string())?;

        // Track main window state before applying on-open action
        if let Some(main_window) = app.get_webview_window("main") {
            let visible = main_window.is_visible().unwrap_or(true);
            let minimized = main_window.is_minimized().unwrap_or(false);
            let mut main_state = self.main_window_state.lock().unwrap();
            *main_state = Some(MainWindowState { visible, minimized });

            match settings.immersive_on_open_action.as_str() {
                "hide_to_tray" => {
                    main_window.hide().map_err(|e| e.to_string())?;
                    info!("Main window hidden to tray");
                }
                "minimize" => {
                    main_window.minimize().map_err(|e| e.to_string())?;
                    info!("Main window minimized");
                }
                "none" => {
                    info!("Main window kept as is");
                }
                _ => {
                    main_window.hide().map_err(|e| e.to_string())?;
                    info!("Main window hidden to tray (default)");
                }
            }
        }

        info!("Immersive window created");
        Ok(())
    }

    pub fn close<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        // Save window state before closing
        if let Some(window) = app.get_webview_window(&self.window_label) {
            window.close().map_err(|e| e.to_string())?;
            info!("Immersive window closed");
        }

        Ok(())
    }

    pub fn handle_window_close<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        window: &WebviewWindow<R>,
    ) -> Result<(), String> {
        self.save_window_state(window)?;
        self.restore_main_window(app)
    }

    fn save_window_state<R: Runtime>(&self, window: &WebviewWindow<R>) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();

        // Save position
        if let Ok(position) = window.outer_position() {
            state.position = Some((position.x, position.y));
        }

        // Save size
        if let Ok(size) = window.outer_size() {
            state.size = Some((size.width, size.height));
        }

        // Save always_on_top
        if let Ok(always_on_top) = window.is_always_on_top() {
            state.always_on_top = always_on_top;
        }

        info!("Immersive window state saved: {:?}", *state);
        Ok(())
    }

    fn restore_main_window<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        if let Some(main_window) = app.get_webview_window("main") {
            let mut main_state = self.main_window_state.lock().unwrap();
            if let Some(state) = main_state.take() {
                if !state.visible {
                    let _ = main_window.hide();
                } else {
                    let _ = main_window.show();
                    if state.minimized {
                        let _ = main_window.minimize();
                    } else {
                        let _ = main_window.unminimize();
                        let _ = main_window.set_focus();
                    }
                }
                info!("Main window restored to original state");
            }
        }
        Ok(())
    }

    pub fn get_state(&self) -> ImmersiveWindowState {
        self.state.lock().unwrap().clone()
    }

    pub fn set_state(&self, new_state: ImmersiveWindowState) {
        let mut state = self.state.lock().unwrap();
        *state = new_state;
    }
}

// Tauri commands
#[tauri::command]
pub async fn open_immersive_window(
    app: AppHandle,
    manager: tauri::State<'_, ImmersiveWindowManager>,
) -> Result<(), String> {
    manager.open(&app)
}

#[tauri::command]
pub async fn close_immersive_window(
    app: AppHandle,
    manager: tauri::State<'_, ImmersiveWindowManager>,
) -> Result<(), String> {
    manager.close(&app)
}

#[tauri::command]
pub async fn get_immersive_window_state(
    manager: tauri::State<'_, ImmersiveWindowManager>,
) -> Result<ImmersiveWindowState, String> {
    Ok(manager.get_state())
}

#[tauri::command]
pub async fn set_immersive_window_state(
    state: ImmersiveWindowState,
    manager: tauri::State<'_, ImmersiveWindowManager>,
) -> Result<(), String> {
    manager.set_state(state);
    Ok(())
}
