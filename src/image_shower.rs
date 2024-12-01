use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, PointerButton};
use egui_plot::{CoordinatesFormatter, Corner, PlotImage, PlotPoint};
use icu_lib::midata::MiData;
use serde::{Deserialize, Serialize};

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

    dropped_files: Vec<egui::DroppedFile>,

    context: AppContext,
}

#[derive(Serialize, Deserialize)]
struct AppContext {
    show_grid: bool,
    anti_alias: bool,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            show_grid: true,
            anti_alias: true,
        }
    }
}

impl MyEguiApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        width: u32,
        height: u32,
        image_data: Option<Vec<Color32>>,
    ) -> Self {
        let context = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        Self {
            width,
            height,
            image_data,

            dropped_files: Default::default(),

            context,
        }
    }
}

static mut COLOR_DATA: Option<Color32> = None;
static mut CURSOR_POS: Option<[f64; 2]> = None;

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
                ui.toggle_value(&mut self.context.show_grid, "Show Grid");
                ui.toggle_value(&mut self.context.anti_alias, "Anti-Aliasing");
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| match &self.image_data {
            None => {}
            Some(image_data) => {
                let image = ColorImage {
                    size: [self.width as usize, self.height as usize],
                    pixels: image_data.clone(),
                };
                let texture = ui.ctx().load_texture(
                    "showing_image",
                    image,
                    if self.context.anti_alias {
                        egui::TextureOptions::LINEAR
                    } else {
                        egui::TextureOptions::NEAREST
                    },
                );

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
                            .y_axis_formatter(move |y, _, _| format!("{:.0}", -y.value))
                            .coordinates_formatter(
                                Corner::LeftBottom,
                                CoordinatesFormatter::new(|p, _b| unsafe {
                                    match COLOR_DATA {
                                        None => {
                                            format!("Nothing {:.0} {:.0}", p.x.floor(), p.y.ceil())
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
                                if pos.x > 0.0 && pos.x < img_w && pos.y < 0.0 && pos.y > -img_h {
                                    let row = -pos.y as usize;
                                    let col = pos.x as usize;
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
                            .show_grid([self.context.show_grid, self.context.show_grid])
                            .clamp_grid(true)
                            .sharp_grid_lines(false);

                        plot.show(ui, |plot_ui| {
                            plot_ui.image(PlotImage::new(
                                texture.id,
                                PlotPoint::new(img_w / 2.0, -img_h / 2.0),
                                texture.size,
                            ));

                            let plot_bounds = plot_ui.plot_bounds();
                            let plot_size = plot_ui.response().rect;
                            let scale = 1.0 / (plot_bounds.width() as f32 / plot_size.width());

                            if let Some(pos) = unsafe { CURSOR_POS } {
                                if let Some(pixel) = unsafe { COLOR_DATA } {
                                    let pos = [pos[0].floor() + 0.5, pos[1].floor() + 0.5];

                                    plot_ui.points(
                                        egui_plot::Points::new(vec![pos])
                                            .shape(egui_plot::MarkerShape::Square)
                                            .radius(scale)
                                            .color(pixel),
                                    );
                                }
                            }
                        });
                    },
                );
            }
        });

        self.ui_file_drag_and_drop(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.context);
    }
}

impl MyEguiApp {
    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("file_drop_target"),
            ));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files = i.raw.dropped_files.clone();
            }
        });

        // Show dropped files (if any):
        if !self.dropped_files.is_empty() {
            let mut open = true;
            egui::Window::new("Dropped files")
                .open(&mut open)
                .show(ctx, |ui| {
                    for file in &self.dropped_files {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];
                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        ui.label(info);
                    }
                });
            if !open {
                self.dropped_files.clear();
            }
        }
    }
}
