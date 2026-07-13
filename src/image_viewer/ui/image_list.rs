use crate::image_viewer::model::{SidebarItem, ViewerState};
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::{Color32, Sense};

pub fn draw_left_panel(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    reset_callback: impl FnOnce(&mut ViewerState),
) {
    if state.items.len() > 1 {
        egui::Panel::left("ImagePicker").show(ui, |ui| {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(egui::RichText::new("🗑").color(Color32::RED))
                    .clicked()
                {
                    state.items.clear();
                    reset_callback(state);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, item) in state.items.clone().iter().enumerate() {
                    draw_sidebar_item(ui, state, index, item);
                }
            });
        });
    }
}

fn draw_sidebar_item(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    index: usize,
    item: &SidebarItem,
) {
    let is_selected = state.selected_index == Some(index);
    egui::containers::Frame::default()
        .inner_margin(6.0)
        .outer_margin(6.0)
        .corner_radius(10.0)
        .show(ui, |ui| {
            ui.set_height(100.0);
            let one_sample = ui.vertical_centered(|ui| {
                ui.vertical_centered(|ui| {
                    match item {
                        SidebarItem::Image(image_item) => {
                            let mut image_plotter = ImagePlotter::new(index.to_string())
                                .anti_alias(state.context.anti_alias)
                                .show_grid(false)
                                .show_only(true);
                            image_plotter.show(ui, &Some(image_item.clone()));
                            ui.add(egui::Label::new(&image_item.path).truncate());
                        }
                        SidebarItem::Glyph(g) => {
                            ui.vertical_centered(|ui| {
                                ui.set_height(60.0);
                                ui.label(
                                    egui::RichText::new(&g.char_repr)
                                        .size(32.0)
                                        .color(Color32::from_rgb(250, 179, 135)),
                                );
                            });
                            ui.add(egui::Label::new(&g.name).truncate());
                        }
                    }
                });
            });

            if state.context.diff_active {
                if let SidebarItem::Image(_) = item {
                    ui.add_space(8.0);
                    draw_diff_selection_buttons(ui, state, index);
                }
            }

            handle_item_interaction(ui, state, index, item, one_sample.response, is_selected);
        });
}

fn draw_diff_selection_buttons(ui: &mut egui::Ui, state: &mut ViewerState, index: usize) {
    ui.horizontal(|ui| {
        let diff1_selected = state.diff_image1_index == Some(index);
        let diff2_selected = state.diff_image2_index == Some(index);
        if ui.selectable_label(diff1_selected, t!("diff1")).clicked() {
            if state.diff_image1_index == Some(index) {
                state.diff_image1_index = None;
            } else {
                state.diff_image1_index = Some(index);
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

fn handle_item_interaction(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    index: usize,
    item: &SidebarItem,
    response: egui::Response,
    is_selected: bool,
) {
    let visuals = ui.style().interact_selectable(&response, is_selected);
    let rect = response.rect;
    let response = ui.allocate_rect(rect, Sense::click());
    if response.clicked() {
        state.selected_index = Some(index);
        if let SidebarItem::Image(image_item) = item {
            state.current_image = Some(image_item.clone());
        }
    }
    if response.hovered() {
        state.hovered_index = Some(index);
    }

    if is_selected || response.hovered() || response.highlighted() || response.has_focus() {
        let rect = rect.expand(10.0);
        let painter = ui.painter_at(rect);
        let rect = rect.expand(-2.0);
        painter.rect(
            rect,
            egui::CornerRadius::same(10),
            Color32::TRANSPARENT,
            egui::Stroke::new(2.0, ui.style().visuals.hyperlink_color),
            egui::StrokeKind::Inside,
        );
        painter.rect(
            rect,
            egui::CornerRadius::same(10),
            visuals.text_color().linear_multiply(0.3),
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
}
