#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod i18n;
mod platform;

use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let icon = load_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1650.0, 980.0]).with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Hangul Syllable Editor",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::editor::FontEditor::default()))
        }),
    )?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    crate::platform::install_unload_guard();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("No window").document().expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    setup_fonts(&cc.egui_ctx);
                    Ok(Box::new(app::editor::FontEditor::default()))
                }),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => loading_text.remove(),
                Err(e) => {
                    loading_text.set_inner_html("<p>앱 시작 실패. 개발자 콘솔을 확인하세요.</p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).expect("icon.png 로드 실패");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    }
}

fn setup_fonts(ctx: &egui::Context) {
    egui_system_fonts::add_with_presets(ctx, [egui_system_fonts::FontPreset::Korean], egui_system_fonts::FontStyle::Sans);
}
