use crate::image_viewer::app::MyEguiApp;
use eframe::egui::DroppedFile;

pub mod app;
pub mod model;
pub mod plotter;
pub mod ui;
pub mod utils;

fn setup_custom_fonts(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();
    fonts.font_data.insert(
        "ark-pixel".to_owned(),
        std::sync::Arc::new(eframe::egui::FontData::from_static(include_bytes!(
            "../../assets/ark-pixel-12px-monospaced-zh_cn.otf"
        ))),
    );

    fn insert_font(
        font: &mut eframe::egui::FontDefinitions,
        family: eframe::egui::FontFamily,
        font_name: &str,
    ) {
        let ins = font.families.get_mut(&family);

        if let Some(font_list) = ins {
            font_list.insert(0, font_name.to_owned());
        } else {
            log::error!("Failed to get {family:?} font family for {font_name:?}");
        }
    }

    insert_font(
        &mut fonts,
        eframe::egui::FontFamily::Proportional,
        "ark-pixel",
    );
    insert_font(&mut fonts, eframe::egui::FontFamily::Monospace, "ark-pixel");

    ctx.set_fonts(fonts);
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
pub fn show_image(files: Vec<DroppedFile>) {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

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
                    setup_custom_fonts(&cc.egui_ctx);
                    Ok(Box::new(MyEguiApp::new(cc, files)))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn show_image(files: Vec<DroppedFile>) {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "ICU Preview",
        native_options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(MyEguiApp::new(cc, files)))
        }),
    )
    .expect("Failed to run eframe");
}
