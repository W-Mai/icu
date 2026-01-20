use crate::image_viewer::model::ViewerState;
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::{Color32, Sense};

/// Draws the left panel for image selection and list management.
pub fn draw_left_panel(
    ctx: &egui::Context,
    state: &mut ViewerState,
    reset_callback: impl FnOnce(&mut ViewerState),
) {
    if state.image_items.len() > 1 {
        egui::SidePanel::left("ImagePicker").show(ctx, |ui| {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(egui::RichText::new("🗑").color(Color32::RED))
                    .clicked()
                {
                    state.image_items.clear();
                    reset_callback(state);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, image_item) in state.image_items.clone().iter().enumerate() {
                    draw_image_picker_item(ui, state, index, image_item);
                }
            });
        });
    }
}

/// Draws a single image item in the left panel list.
fn draw_image_picker_item(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    index: usize,
    image_item: &crate::image_viewer::model::ImageItem,
) {
    let is_selected = state.selected_image_item_index == Some(index);
    egui::containers::Frame::default()
        .inner_margin(6.0)
        .outer_margin(6.0)
        .corner_radius(10.0)
        .show(ui, |ui| {
            ui.set_height(100.0);
            let one_sample = ui.vertical_centered(|ui| {
                ui.vertical_centered(|ui| {
                    let mut image_plotter = ImagePlotter::new(index.to_string())
                        .anti_alias(state.context.anti_alias)
                        .show_grid(false)
                        .show_only(true);

                    image_plotter.show(ui, &Some(image_item.clone()));
                    ui.add(egui::Label::new(&image_item.path).truncate());
                });
            });

            if state.context.image_diff {
                ui.add_space(8.0);
                draw_diff_selection_buttons(ui, state, index);
            }

            handle_item_interaction(
                ui,
                state,
                index,
                image_item,
                one_sample.response,
                is_selected,
            );
        });
}

/// Draws the buttons for selecting images for difference comparison.
fn draw_diff_selection_buttons(ui: &mut egui::Ui, state: &mut ViewerState, index: usize) {
    ui.horizontal(|ui| {
        let diff1_selected = state.diff_image1_index == Some(index);
        let diff2_selected = state.diff_image2_index == Some(index);
        if ui.selectable_label(diff1_selected, t!("diff1")).clicked() {
            if state.diff_image1_index == Some(index) {
                state.diff_image1_index = None;
            } else {
                state.diff_image1_index = Some(index);
                // avoid selecting the same image
                if state.diff_image2_index == Some(index) {
                    state.diff_image2_index = None;
                }
            }
        }
        if ui.selectable_label(diff2_selected, t!("diff2")).clicked() {
            if state.diff_image2_index == Some(index) {
                state.diff_image2_index = None;
            } else {
                state.diff_image2_index = Some(index);
                if state.diff_image1_index == Some(index) {
                    state.diff_image1_index = None;
                }
            }
        }
    });
}

/// Handles interactions (click, hover) for an image item.
fn handle_item_interaction(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    index: usize,
    image_item: &crate::image_viewer::model::ImageItem,
    response: egui::Response,
    is_selected: bool,
) {
    let visuals = ui.style().interact_selectable(&response, is_selected);
    let rect = response.rect;
    let response = ui.allocate_rect(rect, Sense::click());
    if response.clicked() {
        state.selected_image_item_index = Some(index);
        state.current_image = Some(image_item.clone());
    }
    if response.hovered() {
        state.hovered_image_item_index = Some(index);
    }

    if is_selected || response.hovered() || response.highlighted() || response.has_focus() {
        let rect = rect.expand(10.0);
        let painter = ui.painter_at(rect);
        let rect = rect.expand(-2.0);
        painter.rect(
            rect,
            10.0,
            Color32::TRANSPARENT,
            egui::Stroke::new(2.0, ui.style().visuals.hyperlink_color),
            egui::StrokeKind::Inside,
        );
        painter.rect(
            rect,
            10.0,
            visuals.text_color().linear_multiply(0.3),
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
}
