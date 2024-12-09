use crate::image_plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::{Color32, Sense};
use icu_lib::midata::MiData;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

struct ImageItem {
    path: String,

    width: u32,
    height: u32,
    image_data: Vec<Color32>,
}

#[derive(Default)]
struct MyEguiApp {
    width: u32,
    height: u32,
    image_data: Option<Vec<Color32>>,

    image_items: Vec<ImageItem>,
    selected_image_item_index: Option<usize>,
    hovered_image_item_index: Option<usize>,

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

            image_items: vec![],
            selected_image_item_index: None,
            hovered_image_item_index: None,
            dropped_files: Default::default(),

            context,
        }
    }
}

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

        if !self.image_items.is_empty() {
            egui::SidePanel::left("ImagePicker").show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, image_item) in self.image_items.iter().enumerate() {
                        let is_selected = self.selected_image_item_index == Some(index);
                        egui::containers::Frame::default()
                            .inner_margin(10.0)
                            .outer_margin(10.0)
                            .rounding(10.0)
                            .show(ui, |ui| {
                                ui.set_height(100.0);
                                let one_sample = ui.vertical_centered(|ui| {
                                    ui.vertical_centered(|ui| {
                                        let mut image_plotter = ImagePlotter::new()
                                            .anti_alias(self.context.anti_alias)
                                            .show_grid(false)
                                            .show_only(true);

                                        image_plotter.show(
                                            ui,
                                            Some(image_item.image_data.clone()),
                                            [image_item.width as f32, image_item.height as f32]
                                                .into(),
                                        );
                                        ui.add(egui::Label::new(&image_item.path).truncate(true));
                                    })
                                });

                                let response = one_sample.response;

                                let visuals =
                                    ui.style().interact_selectable(&response, is_selected);

                                let rect = response.rect;
                                let response = ui.allocate_rect(rect, Sense::click());
                                if response.clicked() {
                                    if is_selected {
                                        self.selected_image_item_index = None;
                                    } else {
                                        self.selected_image_item_index = Some(index);
                                        self.width = image_item.width;
                                        self.height = image_item.height;
                                        self.image_data = Some(image_item.image_data.clone());
                                    }
                                }
                                if response.hovered() {
                                    self.hovered_image_item_index = Some(index);
                                }

                                if is_selected
                                    || response.hovered()
                                    || response.highlighted()
                                    || response.has_focus()
                                {
                                    let rect = rect.expand(10.0);
                                    let painter = ui.painter_at(rect);
                                    let rect = rect.expand(-2.0);
                                    painter.rect(
                                        rect,
                                        10.0,
                                        egui::Color32::TRANSPARENT,
                                        egui::Stroke::new(2.0, ui.style().visuals.hyperlink_color),
                                    );
                                    painter.rect(
                                        rect,
                                        10.0,
                                        visuals.text_color().linear_multiply(0.3),
                                        egui::Stroke::NONE,
                                    );
                                }
                            });
                    }
                })
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut image_plotter = ImagePlotter::new()
                .anti_alias(self.context.anti_alias)
                .show_grid(self.context.show_grid);

            image_plotter.show(
                ui,
                self.image_data.clone(),
                [self.width as f32, self.height as f32].into(),
            );
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
            for file in &self.dropped_files {
                let info = if let Some(path) = &file.path {
                    path.display().to_string()
                } else if !file.name.is_empty() {
                    file.name.clone()
                } else {
                    continue;
                };

                let mi_data = match &file.bytes {
                    Some(bytes) => {
                        if let Some(coder) = icu_lib::endecoder::find_endecoder(bytes) {
                            coder.decode(bytes.to_vec())
                        } else {
                            continue;
                        }
                    }
                    None => {
                        let data = std::fs::read(&info);
                        match data {
                            Ok(data) => {
                                if let Some(coder) = icu_lib::endecoder::find_endecoder(&data) {
                                    coder.decode(data)
                                } else {
                                    continue;
                                }
                            }
                            _ => continue,
                        }
                    }
                };

                match mi_data {
                    MiData::RGBA(img_buffer) => {
                        let width = img_buffer.width();
                        let height = img_buffer.height();
                        let image_data = Some(
                            img_buffer
                                .chunks(4)
                                .map(|pixel| {
                                    Color32::from_rgba_unmultiplied(
                                        pixel[0], pixel[1], pixel[2], pixel[3],
                                    )
                                })
                                .collect::<Vec<Color32>>(),
                        );

                        self.width = width;
                        self.height = height;
                        self.image_data = image_data.clone();

                        self.image_items.push(ImageItem {
                            path: info,
                            width,
                            height,
                            image_data: image_data.unwrap(),
                        });
                    }
                    MiData::GRAY(_) => {}
                    MiData::PATH => {}
                };
            }

            self.dropped_files.clear();
        }
    }
}
