use crate::image_viewer::model::ViewerState;
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use serde::Serialize;

/// Draws the central panel displaying the image or drag-drop area.
pub fn draw_central_panel(ctx: &egui::Context, state: &mut ViewerState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let mut image_plotter = ImagePlotter::new("viewer")
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
                image_plotter.show(ui, &Some(diff_img.clone()));
            }
        } else if let Some((diff_img, _)) = &state.diff_result
            && state.context.image_diff
        {
            image_plotter.show(ui, &Some(diff_img.clone()));
        } else if let Some(image) = &state.current_image {
            image_plotter.show(ui, &Some(image.clone()));
        } else {
            ui.centered_and_justified(|ui| {
                ui.heading(
                    egui::RichText::new(t!("drag_here"))
                        .size(50.0)
                        .line_height(Some(60.0))
                        .color(ui.style().visuals.weak_text_color()),
                );
            });
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
