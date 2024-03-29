use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, PointerButton};
use egui_plot::{CoordinatesFormatter, Corner, PlotImage, PlotPoint, Polygon};
use icu_lib::midata::MiData;

pub fn show_image(image: MiData) {
    let native_options = eframe::NativeOptions::default();

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

            eframe::run_native(
                "ICU Preview",
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

static mut COLOR_DATA: Option<Color32> = None;
static mut CURSOR_POS: Option<[f64; 2]> = None;

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

                let img_w = self.width as f64;
                let img_h = self.height as f64;
                let copy_image_data = image_data.clone();

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        let plot = egui_plot::Plot::new("plot")
                            .data_aspect(1.0)
                            .coordinates_formatter(
                                Corner::LeftBottom,
                                CoordinatesFormatter::new(|p, _b| unsafe {
                                    match COLOR_DATA {
                                        None => {
                                            format!("Nothing {:.2} {:.2}", p.x, p.y)
                                        }
                                        Some(pixel) => {
                                            format!(
                                                "RGBA: #{:02X}_{:02X}_{:02X}_{:02X}",
                                                pixel.r(),
                                                pixel.g(),
                                                pixel.b(),
                                                pixel.a(),
                                            )
                                        }
                                    }
                                }),
                            )
                            .label_formatter(move |_text, pos| {
                                if pos.x >= (-img_w / 2.0)
                                    && pos.x < (img_w / 2.0)
                                    && pos.y >= (-img_h / 2.0)
                                    && pos.y < (img_h / 2.0)
                                {
                                    let row = (img_h - (pos.y + img_h / 2.0)) as usize;
                                    let col = (pos.x + img_w / 2.0) as usize;
                                    let index = row * img_w as usize + col;
                                    let pixel = copy_image_data[index];

                                    unsafe {
                                        COLOR_DATA = Some(pixel);
                                        CURSOR_POS = Some([pos.x, pos.y]);
                                    }

                                    format!("Pos: {:.2} {:.2}", pos.x, pos.y)
                                } else {
                                    unsafe {
                                        COLOR_DATA = None;
                                        CURSOR_POS = None;
                                    }
                                    "".into()
                                }
                            })
                            .boxed_zoom_pointer_button(PointerButton::Extra2)
                            .show_grid([false, false]);

                        plot.show(ui, |plot_ui| {
                            plot_ui.image(PlotImage::new(
                                texture.id,
                                PlotPoint::new(0.0, 0.0),
                                texture.size,
                            ));
                            if let Some(pos) = unsafe { CURSOR_POS } {
                                if let Some(pixel) = unsafe { COLOR_DATA } {
                                    let pos_floored = [pos[0].floor(), pos[1].floor()];
                                    let pos_ceiled = [pos[0].ceil(), pos[1].ceil()];
                                    plot_ui.polygon(
                                        Polygon::new(vec![
                                            [pos_floored[0], pos_floored[1]],
                                            [pos_floored[0], pos_ceiled[1]],
                                            [pos_ceiled[0], pos_ceiled[1]],
                                            [pos_ceiled[0], pos_floored[1]],
                                            [pos_floored[0], pos_floored[1]],
                                        ])
                                        .width(10.0)
                                        .fill_color(pixel),
                                    )
                                }
                            }
                        });
                    },
                );
            }
        });
    }
}
