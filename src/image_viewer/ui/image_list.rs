use crate::image_viewer::model::{SidebarItem, ViewerState};
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;

pub fn draw_left_panel(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    reset_callback: impl FnOnce(&mut ViewerState),
) {
    if state.items.len() > 1 {
        let frame = crate::image_viewer::ui::theme::side_panel_frame(ui.ctx());
        egui::Panel::left("ImagePicker")
            .exact_size(260.0)
            .frame(frame)
            .show(ui, |ui| {
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
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

    let (name, meta, badge_text, badge_color) = match item {
        SidebarItem::Image(img) => {
            let fname = std::path::Path::new(&img.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| img.path.clone());
            let meta_str = format!("{}×{}", img.width, img.height);
            let (badge, color) = match &img.midata {
                Some(icu_lib::midata::MiData::FONT(_)) => ("FONT", p.mauve),
                Some(icu_lib::midata::MiData::PATH(_)) => ("SVG", p.green),
                Some(icu_lib::midata::MiData::INDEXED(_)) => ("IDX", p.yellow),
                _ => ("IMG", p.accent()),
            };
            (fname, meta_str, badge, color)
        }
        SidebarItem::Glyph(g) => {
            let meta_str = if g.outline_approximate {
                "1 glyph · atlas".to_string()
            } else {
                format!("1 glyph · {} cmds", g.outline.len())
            };
            (g.name.clone(), meta_str, "GLYPH", p.peach)
        }
    };

    let desired = egui::vec2(ui.available_width(), 48.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if is_selected {
            p.accent_dim()
        } else if response.hovered() {
            p.surface1
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, egui::CornerRadius::same(4), fill);
        }

        let thumb_size = 36.0;
        let thumb_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, rect.center().y - thumb_size / 2.0),
            egui::vec2(thumb_size, thumb_size),
        );
        match item {
            SidebarItem::Image(image_item) => {
                ui.allocate_ui_with_layout(
                    thumb_rect.size(),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        let mut plotter = ImagePlotter::new(format!("thumb{}", index))
                            .anti_alias(state.context.anti_alias)
                            .show_grid(false)
                            .show_only(true);
                        plotter.show(ui, &Some(image_item.clone()));
                    },
                );
            }
            SidebarItem::Glyph(g) => {
                ui.painter().rect_filled(
                    thumb_rect,
                    egui::CornerRadius::same(4),
                    p.surface0,
                );
                ui.painter().text(
                    thumb_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &g.char_repr,
                    egui::FontId::proportional(18.0),
                    p.peach,
                );
            }
        }

        let text_x = thumb_rect.right() + 8.0;
        ui.painter().text(
            egui::pos2(text_x, rect.top() + 13.0),
            egui::Align2::LEFT_CENTER,
            &name,
            egui::FontId::proportional(12.0),
            p.text,
        );
        ui.painter().text(
            egui::pos2(text_x, rect.top() + 30.0),
            egui::Align2::LEFT_CENTER,
            &meta,
            egui::FontId::monospace(10.0),
            p.overlay0,
        );

        let badge_galley = ui.painter().layout_no_wrap(
            badge_text.to_string(),
            egui::FontId::proportional(9.0),
            Color32::TRANSPARENT,
        );
        let badge_w = badge_galley.size().x + 10.0;
        let badge_h = badge_galley.size().y + 2.0;
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - badge_w - 8.0, rect.center().y - badge_h / 2.0),
            egui::vec2(badge_w, badge_h),
        );
        ui.painter().rect_filled(badge_rect, egui::CornerRadius::same(3), badge_color);
        ui.painter().galley(
            badge_rect.center() - 0.5 * badge_galley.size(),
            badge_galley,
            p.base,
        );
    }

    if response.clicked() {
        state.selected_index = Some(index);
        if let SidebarItem::Image(image_item) = item {
            state.current_image = Some(image_item.clone());
        }
    }
    if response.hovered() {
        state.hovered_index = Some(index);
    }

    response.context_menu(|ui| {
        if ui.button("Open").clicked() {
            state.selected_index = Some(index);
            if let SidebarItem::Image(image_item) = item {
                state.current_image = Some(image_item.clone());
            }
            ui.close();
        }
        if ui.button("Info").clicked() {
            state.context.right_tab = crate::image_viewer::model::RightTab::Info;
            state.selected_index = Some(index);
            ui.close();
        }
        if ui.button("Export…").clicked() {
            state.context.right_tab = crate::image_viewer::model::RightTab::Convert;
            state.selected_index = Some(index);
            ui.close();
        }
        ui.separator();
        if ui.button("Remove").clicked() {
            if state.selected_index == Some(index) {
                state.selected_index = None;
                state.current_image = None;
            }
            state.items.remove(index);
            ui.close();
        }
    });

    if state.context.diff_active {
        if let SidebarItem::Image(_) = item {
            let diff_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - 16.0),
                egui::vec2(rect.width(), 16.0),
            );
            ui.allocate_ui_with_layout(
                diff_rect.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    draw_diff_selection_buttons(ui, state, index);
                },
            );
        }
    }
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

