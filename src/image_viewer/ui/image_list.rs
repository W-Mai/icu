use crate::image_viewer::model::{SidebarItem, ViewerState};
use eframe::egui;
use eframe::egui::Color32;

pub fn draw_left_panel(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    reset_callback: impl FnOnce(&mut ViewerState),
) {
    let frame = crate::image_viewer::ui::theme::side_panel_frame(ui.ctx());
    egui::Panel::left("ImagePicker")
        .exact_size(260.0)
        .frame(frame)
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::same(4)).show(ui, |ui| {
            let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

            let header_h = 28.0;
            let (hdr_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), header_h),
                egui::Sense::hover(),
            );
            if ui.is_rect_visible(hdr_rect) {
                ui.painter().text(
                    egui::pos2(hdr_rect.left() + 4.0, hdr_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("FILES ({})", state.items.len()),
                    egui::FontId::proportional(11.0),
                    p.overlay0,
                );
                let btn_y = hdr_rect.center().y;
                let add_rect = egui::Rect::from_center_size(
                    egui::pos2(hdr_rect.right() - 40.0, btn_y),
                    egui::vec2(24.0, 20.0),
                );
                let add_resp = ui.interact(add_rect, ui.id().with("sb_add"), egui::Sense::click());
                let add_fill = if add_resp.hovered() {
                    p.surface1
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect(
                    add_rect,
                    egui::CornerRadius::same(4),
                    add_fill,
                    egui::Stroke::NONE,
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    add_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "＋",
                    egui::FontId::proportional(14.0),
                    p.subtext0,
                );
                if add_resp.clicked() {
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
                            let new_items: Vec<SidebarItem> =
                                crate::image_viewer::utils::process_images(&files)
                                    .into_iter()
                                    .map(SidebarItem::Image)
                                    .collect();
                            state.items.extend(new_items);
                            if state.selected_index.is_none() {
                                if let Some(SidebarItem::Image(img)) = state.items.first().cloned()
                                {
                                    state.current_image = Some(img);
                                    state.selected_index = Some(0);
                                }
                            }
                        }
                    }
                }
                let clr_rect = egui::Rect::from_center_size(
                    egui::pos2(hdr_rect.right() - 16.0, btn_y),
                    egui::vec2(24.0, 20.0),
                );
                let clr_resp =
                    ui.interact(clr_rect, ui.id().with("sb_clear"), egui::Sense::click());
                let clr_fill = if clr_resp.hovered() {
                    p.surface1
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect(
                    clr_rect,
                    egui::CornerRadius::same(4),
                    clr_fill,
                    egui::Stroke::NONE,
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    clr_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "✕",
                    egui::FontId::proportional(12.0),
                    p.red,
                );
                if clr_resp.clicked() {
                    state.items.clear();
                    reset_callback(state);
                }
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.allocate_space(egui::vec2(4.0, 0.0));
                for (index, item) in state.items.clone().iter().enumerate() {
                    draw_sidebar_item(ui, state, index, item);
                    ui.add_space(2.0);
                }
            });
            });
        });
}

fn draw_sidebar_item(ui: &mut egui::Ui, state: &mut ViewerState, index: usize, item: &SidebarItem) {
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
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(4), fill);
        }

        if let SidebarItem::Glyph(_) = item {
            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 1.0, rect.top() + 4.0),
                egui::vec2(2.0, rect.height() - 8.0),
            );
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(0), p.peach);
        }

        let thumb_size = 36.0;
        let thumb_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, rect.center().y - thumb_size / 2.0),
            egui::vec2(thumb_size, thumb_size),
        );
        match item {
            SidebarItem::Image(image_item) => {
                ui.painter()
                    .rect_filled(thumb_rect, egui::CornerRadius::same(4), p.surface0);
                ui.painter().rect_stroke(
                    thumb_rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.0, p.surface1),
                    egui::StrokeKind::Inside,
                );
                let tex = ui.ctx().load_texture(
                    format!("sb_thumb_{}", index),
                    egui::ColorImage {
                        size: [image_item.width as usize, image_item.height as usize],
                        source_size: egui::vec2(image_item.width as f32, image_item.height as f32),
                        pixels: image_item.image_data.clone(),
                    },
                    egui::TextureOptions::LINEAR,
                );
                let img_aspect = image_item.width as f32 / image_item.height as f32;
                let inner = thumb_rect.shrink(2.0);
                let draw_h = inner.height();
                let draw_w = draw_h * img_aspect;
                let img_rect = egui::Rect::from_center_size(
                    inner.center(),
                    egui::vec2(draw_w.min(inner.width()), draw_h),
                );
                ui.painter().image(
                    tex.id(),
                    img_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            SidebarItem::Glyph(g) => {
                ui.painter()
                    .rect_filled(thumb_rect, egui::CornerRadius::same(4), p.surface0);
                ui.painter().rect_stroke(
                    thumb_rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.0, p.surface1),
                    egui::StrokeKind::Inside,
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
            p.base,
        );
        let badge_w = badge_galley.size().x + 10.0;
        let badge_h = badge_galley.size().y + 2.0;
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.right() - badge_w - 8.0,
                rect.center().y - badge_h / 2.0,
            ),
            egui::vec2(badge_w, badge_h),
        );
        ui.painter()
            .rect_filled(badge_rect, egui::CornerRadius::same(3), badge_color);
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
