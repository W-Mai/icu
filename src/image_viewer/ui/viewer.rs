use crate::image_viewer::model::ViewerState;
use crate::image_viewer::plotter::ImagePlotter;
use crate::image_viewer::ui::panels;
use eframe::egui;
use icu_lib::midata::MiData;
use serde::Serialize;

pub fn draw_central_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    use crate::image_viewer::model::SidebarItem;

    if let Some(idx) = state.selected_index {
        if let Some(SidebarItem::Glyph(_)) = state.items.get(idx) {
            panels::draw_glyph_panel(ui, state);
            return;
        }
    }

    if let Some(image) = &state.current_image {
        if let Some(midata) = &image.midata {
            match midata {
                MiData::FONT(_) => {
                    panels::draw_font_panel(ui, state);
                    return;
                }
                MiData::PATH(_) => {
                    panels::draw_path_panel(ui, state);
                    return;
                }
                MiData::INDEXED(_) => {
                    panels::draw_indexed_panel(ui, state);
                    return;
                }
                _ => {}
            }
        }
    }

    egui::CentralPanel::default().show(ui, |ui| {
        let image_plotter = ImagePlotter::new("viewer")
            .anti_alias(state.context.anti_alias)
            .show_grid(state.context.show_grid)
            .background_color(state.context.background_color)
            .highlight(if state.hovered_diff_pixel.is_none() {
                state.selected_diff_pixel
            } else {
                state.hovered_diff_pixel
            })
            .on_hover(&mut state.hovered_diff_pixel_from_plot);

        if state.context.only_show_diff {
            if let Some((diff_img, _)) = &state.diff_result {
                image_plotter.badge(format!("{}×{} · diff", diff_img.width, diff_img.height)).show(ui, &Some(diff_img.clone()));
            }
        } else if let Some((diff_img, _)) = &state.diff_result
            && state.context.diff_active
        {
            image_plotter.badge(format!("{}×{} · diff", diff_img.width, diff_img.height)).show(ui, &Some(diff_img.clone()));
        } else if let Some(image) = &state.current_image {
            image_plotter.badge(format!("{}×{} · {}", image.width, image.height, image.info.format)).show(ui, &Some(image.clone()));
        } else {
            let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
            let avail = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click());
            let hovered = response.hovered();
            if ui.is_rect_visible(rect) {
                let stroke_color = if hovered { p.accent() } else { p.surface1 };
                let dash_len = 8.0;
                let gap = 6.0;
                paint_dashed_rect(ui.painter(), rect, egui::CornerRadius::same(12), stroke_color, dash_len, gap);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    t!("drag_here"),
                    egui::FontId::proportional(16.0),
                    if hovered { p.accent() } else { p.overlay0 },
                );
            }
            if response.clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let files: Vec<eframe::egui::DroppedFile> = rfd::FileDialog::new()
                        .pick_files()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| eframe::egui::DroppedFile {
                            path: Some(p),
                            ..Default::default()
                        })
                        .collect();
                    if !files.is_empty() {
                        let new_items: Vec<crate::image_viewer::model::SidebarItem> =
                            crate::image_viewer::utils::process_images(&files)
                                .into_iter()
                                .map(crate::image_viewer::model::SidebarItem::Image)
                                .collect();
                        state.items.extend(new_items);
                        if let Some(crate::image_viewer::model::SidebarItem::Image(img)) =
                            state.items.first().cloned()
                        {
                            state.current_image = Some(img);
                            state.selected_index = Some(0);
                        }
                    }
                }
            }
        }
    });
}

/// Draws the image info window.
pub fn draw_image_info(ctx: &egui::Context, state: &mut ViewerState) {
    if let Some(current_image) = &state.current_image {
        egui::Window::new(t!("image_info")).show(ctx, |ui| {
            egui::Grid::new("info_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(t!("width"));
                    ui.label(format!("{}", current_image.info.width));
                    ui.end_row();

                    ui.label(t!("height"));
                    ui.label(format!("{}", current_image.info.height));
                    ui.end_row();

                    ui.label(t!("format"));
                    ui.label(&current_image.info.format);
                    ui.end_row();

                    ui.label(t!("size"));
                    ui.label(format!("{} bytes", current_image.info.data_size));
                    ui.end_row();
                });

            ui.separator();
            ui.label(t!("other_info"));
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui_tree_view(ui, &current_image.info.other_info);
            });
        });
    }
}

/// Renders a serializable value as a YAML tree.
fn ui_tree_view(ui: &mut egui::Ui, value: &impl Serialize) {
    if let Ok(yaml_value) = serde_yaml::to_value(value) {
        ui_yaml_tree(ui, &yaml_value);
    } else {
        ui.label("Error displaying data");
    }
}

/// Recursive helper to render YAML data.
fn ui_yaml_tree(ui: &mut egui::Ui, value: &serde_yaml::Value) {
    match value {
        serde_yaml::Value::Null => {
            ui.label("~");
        }
        serde_yaml::Value::Bool(b) => {
            ui.label(b.to_string());
        }
        serde_yaml::Value::Number(n) => {
            ui.label(n.to_string());
        }
        serde_yaml::Value::String(s) => {
            ui.label(format!("{s:?}"));
        }
        serde_yaml::Value::Sequence(seq) => {
            ui.collapsing(format!("Sequence [{}]", seq.len()), |ui| {
                for (i, v) in seq.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("- [{i}]"));
                        ui_yaml_tree(ui, v);
                    });
                }
            });
        }
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => format!("{k:?}"),
                };

                if v.is_mapping() || v.is_sequence() {
                    ui.collapsing(key_str, |ui| {
                        ui_yaml_tree(ui, v);
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(format!("{key_str}: "));
                        ui_yaml_tree(ui, v);
                    });
                }
            }
        }
        serde_yaml::Value::Tagged(tagged) => {
            ui.horizontal(|ui| {
                ui.label(format!("!{}", tagged.tag));
                ui_yaml_tree(ui, &tagged.value);
            });
        }
    }
}

fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner: egui::CornerRadius,
    color: eframe::egui::Color32,
    dash: f32,
    gap: f32,
) {
    let stroke = egui::Stroke::new(2.0, color);
    let step = dash + gap;
    let tl = rect.left_top();
    let tr = rect.right_top();
    let br = rect.right_bottom();
    let mut t = 0.0;
    while t < rect.width() {
        let x1 = rect.left() + t;
        let x2 = (x1 + dash).min(rect.right());
        painter.line_segment(
            [egui::pos2(x1, tl.y), egui::pos2(x2, tl.y)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(x1, br.y), egui::pos2(x2, br.y)],
            stroke,
        );
        t += step;
    }
    let mut t = 0.0;
    while t < rect.height() {
        let y1 = rect.top() + t;
        let y2 = (y1 + dash).min(rect.bottom());
        painter.line_segment(
            [egui::pos2(tl.x, y1), egui::pos2(tl.x, y2)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(tr.x, y1), egui::pos2(tr.x, y2)],
            stroke,
        );
        t += step;
    }
    let _ = corner;
}
