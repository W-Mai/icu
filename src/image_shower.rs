use crate::image_plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::color_picker::Alpha;
use eframe::egui::{Color32, DroppedFile, Sense};
use icu_lib::midata::MiData;
use serde::{Deserialize, Serialize};

pub fn show_image(files: Vec<DroppedFile>) {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "ICU Preview",
        native_options,
        Box::new(move |cc| Box::new(MyEguiApp::new(cc, files))),
    )
    .expect("Failed to run eframe");
}

fn process_images(files: &[DroppedFile]) -> Vec<ImageItem> {
    files
        .iter()
        .map_while(|file| {
            let info = if let Some(path) = &file.path {
                path.display().to_string()
            } else if !file.name.is_empty() {
                file.name.clone()
            } else {
                return None;
            };

            let mi_data = match &file.bytes {
                Some(bytes) => {
                    if let Some(coder) = icu_lib::endecoder::find_endecoder(bytes) {
                        coder.decode(bytes.to_vec())
                    } else {
                        return None;
                    }
                }
                None => {
                    let data = std::fs::read(&info);
                    match data {
                        Ok(data) => {
                            if let Some(coder) = icu_lib::endecoder::find_endecoder(&data) {
                                coder.decode(data)
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                }
            };

            match mi_data {
                MiData::RGBA(img_buffer) => {
                    let width = img_buffer.width();
                    let height = img_buffer.height();
                    let image_data = img_buffer
                        .chunks(4)
                        .map(|pixel| {
                            Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
                        })
                        .collect::<Vec<Color32>>();

                    Some(ImageItem {
                        path: info,
                        width,
                        height,
                        image_data,
                    })
                }
                MiData::GRAY(_) => None,
                MiData::PATH => None,
            }
        })
        .collect()
}

#[derive(Clone)]
pub struct ImageItem {
    pub path: String,

    pub width: u32,
    pub height: u32,
    pub image_data: Vec<Color32>,
}

#[derive(Default)]
struct MyEguiApp {
    current_image: Option<ImageItem>,

    image_items: Vec<ImageItem>,
    selected_image_item_index: Option<usize>,
    hovered_image_item_index: Option<usize>,

    dropped_files: Vec<DroppedFile>,

    context: AppContext,
}

#[derive(Serialize, Deserialize)]
struct AppContext {
    show_grid: bool,
    anti_alias: bool,
    background_color: Color32,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            show_grid: true,
            anti_alias: true,
            background_color: Default::default(),
        }
    }
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, files: Vec<DroppedFile>) -> Self {
        let context = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let image_items = process_images(&files);

        Self {
            current_image: image_items.first().cloned(),

            image_items,
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
                ui.set_height(30.0);
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
                ui.toggle_value(&mut self.context.show_grid, "Show Grid");
                ui.toggle_value(&mut self.context.anti_alias, "Anti-Aliasing");
                ui.separator();
                if ui.button("Clear").clicked() {
                    self.context.background_color =
                        self.context.background_color.linear_multiply(0.0);
                }
                egui::widgets::color_picker::color_edit_button_srgba(
                    ui,
                    &mut self.context.background_color,
                    Alpha::BlendOrAdditive,
                );
            });
        });

        if self.image_items.len() > 1 {
            egui::SidePanel::left("ImagePicker").show(ctx, |ui| {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(egui::RichText::new("🗑").color(Color32::RED))
                        .clicked()
                    {
                        self.image_items.clear();
                    }
                });
                ui.separator();
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
                                        let mut image_plotter =
                                            ImagePlotter::new(index.to_string())
                                                .anti_alias(self.context.anti_alias)
                                                .show_grid(false)
                                                .show_only(true);

                                        image_plotter.show(ui, &Some(image_item.clone()));
                                        ui.add(egui::Label::new(&image_item.path).truncate(true));
                                    })
                                });

                                let response = one_sample.response;

                                let visuals =
                                    ui.style().interact_selectable(&response, is_selected);

                                let rect = response.rect;
                                let response = ui.allocate_rect(rect, Sense::click());
                                if response.clicked() {
                                    self.selected_image_item_index = Some(index);
                                    self.current_image = Some(image_item.clone());
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
                                        Color32::TRANSPARENT,
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
            let mut image_plotter = ImagePlotter::new("viewer")
                .anti_alias(self.context.anti_alias)
                .show_grid(self.context.show_grid)
                .background_color(self.context.background_color);

            image_plotter.show(ui, &self.current_image);
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
            self.image_items = process_images(&self.dropped_files);
            if let Some(image) = self.image_items.first() {
                self.current_image = Some(image.clone());
            }
            self.dropped_files.clear();
        }
    }
}
