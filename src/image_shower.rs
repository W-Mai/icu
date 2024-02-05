use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage};
use icu_lib::midata::MiData;

pub fn show_image(image: MiData) {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport.resizable = Some(false);
    native_options.viewport.transparent = Some(true);

    match image {
        MiData::RGBA(img_buffer) => {
            let width = img_buffer.width();
            let height = img_buffer.height();
            let image_data = Some(
                img_buffer
                    .chunks(4)
                    .map(|pixel| {
                        Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
                    })
                    .collect::<Vec<Color32>>(),
            );

            // native_options.viewport.inner_size = Some(Vec2::new(width as f32 * 1.2, height as f32 * 1.2));
            eframe::run_native(
                "My egui App",
                native_options,
                Box::new(move |cc| Box::new(MyEguiApp::new(cc, width, height, image_data))),
            )
            .expect("Failed to run eframe");
        }
        MiData::GRAY(_) => {}
        MiData::PATH => {}
    };
}

#[derive(Default)]
struct MyEguiApp {
    width: u32,
    height: u32,
    image_data: Option<Vec<Color32>>,
}

impl MyEguiApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        width: u32,
        height: u32,
        image_data: Option<Vec<Color32>>,
    ) -> Self {
        Self {
            width,
            height,
            image_data,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| match &self.image_data {
            None => {}
            Some(image_data) => {
                let image = ColorImage {
                    size: [self.width as usize, self.height as usize],
                    pixels: image_data.clone(),
                };
                let texture = ui
                    .ctx()
                    .load_texture("showing_image", image, Default::default());

                let texture =
                    SizedTexture::new(texture.id(), [self.width as f32, self.height as f32]);

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.add(egui::Image::from_texture(texture));
                    },
                );
            }
        });
    }
}
