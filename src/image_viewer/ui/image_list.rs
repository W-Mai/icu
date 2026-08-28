use crate::image_viewer::model::{FrameSource, SidebarItem, ViewerState};
use eframe::egui;
use eframe::egui::Color32;

pub(crate) fn keyboard_focus_id() -> egui::Id {
    egui::Id::new("image_list_keyboard_focus")
}

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
            egui::Frame::new()
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

                    let header_h = 36.0;
                    let (hdr_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), header_h),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(hdr_rect) {
                        ui.painter().text(
                            egui::pos2(hdr_rect.left() + 8.0, hdr_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            t!("files_header", count = state.len()).to_string(),
                            egui::FontId::proportional(11.0),
                            p.overlay0,
                        );
                        let btn_y = hdr_rect.center().y;
                        let add_rect = egui::Rect::from_center_size(
                            egui::pos2(hdr_rect.right() - 76.0, btn_y),
                            egui::vec2(24.0, 24.0),
                        );
                        let add_resp =
                            ui.interact(add_rect, ui.id().with("sb_add"), egui::Sense::click());
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
                                        crate::image_viewer::utils::process_images_with_format(
                                            &files,
                                            state.input_format,
                                        )
                                        .into_iter()
                                        .map(SidebarItem::Image)
                                        .collect();
                                    state.insert_and_select_first(new_items);
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::image_viewer::utils::pick_files_web(
                                    state.pending_dropped.clone(),
                                    ui.ctx().clone(),
                                );
                            }
                        }
                        let folder_rect = egui::Rect::from_center_size(
                            egui::pos2(hdr_rect.right() - 48.0, btn_y),
                            egui::vec2(24.0, 24.0),
                        );
                        let folder_resp = ui.interact(
                            folder_rect,
                            ui.id().with("sb_folder"),
                            egui::Sense::click(),
                        );
                        let folder_fill = if folder_resp.hovered() {
                            p.surface1
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect(
                            folder_rect,
                            egui::CornerRadius::same(4),
                            folder_fill,
                            egui::Stroke::NONE,
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            folder_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "📁",
                            egui::FontId::proportional(12.0),
                            p.subtext0,
                        );
                        if folder_resp.clicked() {
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                let files = [eframe::egui::DroppedFile {
                                    path: Some(path),
                                    ..Default::default()
                                }];
                                let new_items =
                                    crate::image_viewer::utils::process_images_with_format(
                                        &files,
                                        state.input_format,
                                    )
                                    .into_iter()
                                    .map(SidebarItem::Image)
                                    .collect::<Vec<_>>();
                                state.insert_and_select_first(new_items);
                            }
                            #[cfg(target_arch = "wasm32")]
                            crate::image_viewer::utils::pick_directory_web(
                                state.pending_dropped.clone(),
                                ui.ctx().clone(),
                            );
                        }
                        let clr_rect = egui::Rect::from_center_size(
                            egui::pos2(hdr_rect.right() - 20.0, btn_y),
                            egui::vec2(24.0, 24.0),
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
                            reset_callback(state);
                        }
                    }

                    if state.selected_ids.len() > 1
                        || state
                            .selected_ids
                            .iter()
                            .any(|id| state.is_sequence_group(*id))
                    {
                        ui.horizontal_wrapped(|ui| {
                            if state.selected_ids.len() > 1 && ui.button(t!("ctx_group")).clicked()
                            {
                                state.group_selected();
                            }
                            if state
                                .selected_ids
                                .iter()
                                .any(|id| state.is_sequence_group(*id))
                                && ui.button(t!("ctx_ungroup")).clicked()
                            {
                                state.ungroup_selected();
                            }
                            if ui.button(t!("ctx_remove")).clicked() {
                                state.remove_selected();
                            }
                            if ui.button(t!("ctx_export")).clicked() {
                                state.context.right_tab =
                                    crate::image_viewer::model::RightTab::Convert;
                                state.blur_list();
                            }
                        });
                    }

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.allocate_space(egui::vec2(4.0, 0.0));
                        for workspace_item in state.items_snapshot() {
                            draw_sidebar_item(
                                ui,
                                state,
                                workspace_item.id(),
                                workspace_item.content(),
                            );
                            ui.add_space(2.0);
                        }
                    });
                    let owns_keyboard = ui.memory(|memory| memory.has_focus(keyboard_focus_id()));
                    state.list_focus = owns_keyboard;
                    if owns_keyboard {
                        let (up, down, delete, select_all) = ui.input(|input| {
                            (
                                input.key_pressed(egui::Key::ArrowUp),
                                input.key_pressed(egui::Key::ArrowDown),
                                input.key_pressed(egui::Key::Delete),
                                input.key_pressed(egui::Key::A)
                                    && (input.modifiers.mac_cmd
                                        || input.modifiers.command
                                        || input.modifiers.ctrl),
                            )
                        });
                        if select_all {
                            state.select_all();
                        } else if up {
                            state.move_selection(-1);
                        } else if down {
                            state.move_selection(1);
                        } else if delete {
                            state.remove_selected();
                        }
                    }
                });
        });
}

fn draw_sidebar_item(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    id: crate::image_viewer::model::WorkspaceId,
    item: &SidebarItem,
) {
    let is_selected = state.selected_ids.contains(&id);
    let is_primary = state.selected_id == Some(id);
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

    let (name, meta, badge_text, badge_color) = match item {
        SidebarItem::Image(img) => {
            let fname = state.group_label(id).map(str::to_owned).unwrap_or_else(|| {
                std::path::Path::new(&img.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| img.path.clone())
            });
            let meta_str = format!("{}×{} · {}", img.width, img.height, img.info.format);
            let (badge, color) = match &img.midata {
                Some(icu_lib::midata::MiData::FONT(_)) => ("FONT", p.mauve),
                Some(icu_lib::midata::MiData::PATH(_)) => ("SVG", p.green),
                Some(icu_lib::midata::MiData::INDEXED(_)) => ("INDEXED", p.yellow),
                _ => ("IMG", p.accent()),
            };
            (fname, meta_str, badge, color)
        }
        SidebarItem::Glyph(g) => {
            let meta_str = if g.outline_approximate {
                t!("meta_glyph_atlas").to_string()
            } else {
                t!("meta_glyph_cmds", count = g.outline.len()).to_string()
            };
            (g.name.clone(), meta_str, "GLYPH", p.peach)
        }
    };

    let is_animated = matches!(
        item,
        SidebarItem::Image(img) if matches!(img.frames, FrameSource::Animated { .. })
    );
    let is_expanded = matches!(
        item,
        SidebarItem::Image(img) if img.expanded
    );
    let (frame_count, current_frame) = match item {
        SidebarItem::Image(img) => match &img.frames {
            FrameSource::Animated {
                frames, current, ..
            } => (frames.len(), *current),
            _ => (0, 0),
        },
        _ => (0, 0),
    };

    let row_height = if state.context.diff_active && matches!(item, SidebarItem::Image(_)) {
        72.0
    } else {
        56.0
    };
    let desired = egui::vec2(ui.available_width(), row_height);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let arrow_rect = if is_animated {
        Some(egui::Rect::from_center_size(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::vec2(16.0, 16.0),
        ))
    } else {
        None
    };
    let arrow_resp = arrow_rect.and_then(|ar| {
        let r = ui.interact(ar, ui.id().with(("sb_arrow", id)), egui::Sense::click());
        Some((ar, r))
    });

    if ui.is_rect_visible(rect) {
        let fill = if is_primary {
            p.accent_dim()
        } else if is_selected || response.hovered() {
            p.surface1
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(4), fill);
        }

        if let SidebarItem::Glyph(_) = item {
            let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(0), p.peach);
        }

        let indent = if is_animated { 20.0 } else { 0.0 };
        let thumb_size = 40.0;
        let thumb_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + 6.0 + indent,
                rect.center().y - thumb_size / 2.0,
            ),
            egui::vec2(thumb_size, thumb_size),
        );
        match item {
            SidebarItem::Image(image_item) => {
                let (pixels, thumb_w, thumb_h) = image_item.current_pixels();
                ui.painter()
                    .rect_filled(thumb_rect, egui::CornerRadius::same(4), p.surface0);
                ui.painter().rect_stroke(
                    thumb_rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.0, p.surface1),
                    egui::StrokeKind::Inside,
                );
                let pixel_count = thumb_w as usize * thumb_h as usize;
                if pixel_count > 0 && pixels.len() == pixel_count {
                    let tex = ui.ctx().load_texture(
                        format!("sb_thumb_{id:?}"),
                        egui::ColorImage {
                            size: [thumb_w as usize, thumb_h as usize],
                            source_size: egui::vec2(thumb_w as f32, thumb_h as f32),
                            pixels: pixels.to_vec(),
                        },
                        egui::TextureOptions::LINEAR,
                    );
                    let img_aspect = thumb_w as f32 / thumb_h as f32;
                    let inner = thumb_rect.shrink(2.0);
                    let draw_size = if img_aspect >= inner.width() / inner.height() {
                        egui::vec2(inner.width(), inner.width() / img_aspect)
                    } else {
                        egui::vec2(inner.height() * img_aspect, inner.height())
                    };
                    let img_rect = egui::Rect::from_center_size(inner.center(), draw_size);
                    ui.painter().image(
                        tex.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        thumb_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        badge_text,
                        egui::FontId::proportional(9.0),
                        badge_color,
                    );
                }
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

        if let Some((ar, _)) = arrow_resp {
            let arrow = if is_expanded { "▼" } else { "▶" };
            ui.painter().text(
                ar.center(),
                egui::Align2::CENTER_CENTER,
                arrow,
                egui::FontId::proportional(10.0),
                p.subtext0,
            );
        }

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
        let text_x = thumb_rect.right() + 8.0;
        let text_clip_rect = egui::Rect::from_min_max(
            egui::pos2(text_x, rect.top()),
            egui::pos2((badge_rect.left() - 6.0).max(text_x), rect.bottom()),
        );
        let text_painter = ui.painter().with_clip_rect(text_clip_rect);
        text_painter.text(
            egui::pos2(text_x, rect.top() + 13.0),
            egui::Align2::LEFT_CENTER,
            &name,
            egui::FontId::proportional(12.0),
            p.text,
        );
        text_painter.text(
            egui::pos2(text_x, rect.top() + 30.0),
            egui::Align2::LEFT_CENTER,
            &meta,
            egui::FontId::monospace(10.0),
            p.overlay0,
        );

        ui.painter()
            .rect_filled(badge_rect, egui::CornerRadius::same(3), badge_color);
        ui.painter().galley(
            badge_rect.center() - 0.5 * badge_galley.size(),
            badge_galley,
            p.base,
        );
    }

    if let Some((_, ar)) = arrow_resp {
        if ar.clicked() {
            if let Some(SidebarItem::Image(img)) = state.item_mut(id) {
                img.expanded = !img.expanded;
            }
        }
    }

    if response.clicked() {
        ui.memory_mut(|memory| memory.request_focus(keyboard_focus_id()));
        let modifiers = ui.input(|input| input.modifiers);
        if modifiers.shift {
            state.extend_selection(id);
        } else if modifiers.command || modifiers.ctrl {
            state.toggle_selection(id);
        } else {
            state.focus_list(id);
        }
        if matches!(item, SidebarItem::Image(_)) {
            state.font_mode = crate::image_viewer::model::FontMode::Grid;
        }
    }
    if response.hovered() {
        state.hovered_id = Some(id);
    }

    response.context_menu(|ui| {
        if ui.button(t!("ctx_open")).clicked() {
            state.select(id);
            ui.close();
        }
        if ui.button(t!("ctx_info")).clicked() {
            state.context.right_tab = crate::image_viewer::model::RightTab::Info;
            state.select(id);
            ui.close();
        }
        if ui.button(t!("ctx_export")).clicked() {
            state.context.right_tab = crate::image_viewer::model::RightTab::Convert;
            state.select(id);
            ui.close();
        }
        ui.separator();
        if state.is_sequence_group(id) {
            if ui.button(t!("collection_rename")).clicked() {
                state.renaming_group = Some(id);
                state.rename_buffer = state.group_label(id).unwrap_or_default().to_owned();
                ui.close();
            }
            if ui.button(t!("ctx_ungroup")).clicked() {
                state.ungroup(id);
                ui.close();
            }
        }
        if ui.button(t!("ctx_remove")).clicked() {
            state.remove_id(id);
            ui.close();
        }
    });

    if state.renaming_group == Some(id) {
        egui::Window::new(t!("collection_rename"))
            .id(ui.id().with(("rename_group", id)))
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let response = ui.text_edit_singleline(&mut state.rename_buffer);
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    let label = std::mem::take(&mut state.rename_buffer);
                    state.set_group_label(id, label);
                    state.renaming_group = None;
                }
                if ui.button(t!("convert")).clicked() {
                    let label = std::mem::take(&mut state.rename_buffer);
                    state.set_group_label(id, label);
                    state.renaming_group = None;
                }
            });
    }

    if state.context.diff_active {
        if let SidebarItem::Image(_) = item {
            let diff_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - 16.0),
                egui::vec2(rect.width(), 16.0),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(diff_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    draw_diff_selection_buttons(ui, state, id);
                },
            );
        }
    }

    if is_animated && is_expanded {
        draw_frame_child_rows(ui, state, id, frame_count, current_frame, is_selected);
    }
}

fn draw_frame_child_rows(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    parent_id: crate::image_viewer::model::WorkspaceId,
    frame_count: usize,
    current_frame: usize,
    parent_selected: bool,
) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    let row_h = 24.0;
    let indent = 16.0;
    let members = state.frame_snapshots(parent_id).map(|items| {
        items
            .into_iter()
            .map(|(name, image)| (name, image))
            .collect::<Vec<_>>()
    });
    for frame_idx in 0..frame_count {
        let desired = egui::vec2(ui.available_width(), row_h);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        let is_current = parent_selected && frame_idx == current_frame;
        if ui.is_rect_visible(rect) {
            let fill = if is_current {
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

            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.left() + indent + 1.0, rect.top() + 3.0),
                egui::vec2(2.0, rect.height() - 6.0),
            );
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(0), p.peach);

            if let Some((_, frame_item)) = members.as_ref().and_then(|items| items.get(frame_idx)) {
                let (pixels, width, height) = frame_item.current_pixels();
                if width > 0 && height > 0 {
                    let tex = ui.ctx().load_texture(
                        format!("sb_frame_{parent_id:?}_{frame_idx}"),
                        egui::ColorImage {
                            size: [width as usize, height as usize],
                            source_size: egui::vec2(width as f32, height as f32),
                            pixels: pixels.to_vec(),
                        },
                        egui::TextureOptions::LINEAR,
                    );
                    let thumb = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + indent + 6.0, rect.center().y - 8.0),
                        egui::vec2(16.0, 16.0),
                    );
                    ui.painter().image(
                        tex.id(),
                        thumb,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            let label = members
                .as_ref()
                .and_then(|items| items.get(frame_idx))
                .map(|(name, _)| {
                    std::path::Path::new(name)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| name.clone())
                })
                .unwrap_or_else(|| t!("frame_short", index = frame_idx + 1).to_string());
            let color = if is_current { p.text } else { p.subtext0 };
            ui.painter().text(
                egui::pos2(rect.left() + indent + 28.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(11.0),
                color,
            );
        }

        if response.clicked() {
            state.select_frame(parent_id, frame_idx);
            state.font_mode = crate::image_viewer::model::FontMode::Grid;
        }
    }
}

fn draw_diff_selection_buttons(
    ui: &mut egui::Ui,
    state: &mut ViewerState,
    id: crate::image_viewer::model::WorkspaceId,
) {
    ui.horizontal(|ui| {
        let diff1_selected = state.diff_image1_id == Some(id);
        let diff2_selected = state.diff_image2_id == Some(id);
        if crate::image_viewer::ui::widgets::button_opts(
            ui,
            t!("diff1"),
            crate::image_viewer::ui::widgets::ButtonOpts {
                active: diff1_selected,
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            if state.diff_image1_id == Some(id) {
                state.diff_image1_id = None;
            } else {
                state.diff_image1_id = Some(id);
                if state.diff_image2_id == Some(id) {
                    state.diff_image2_id = None;
                }
            }
        }
        if crate::image_viewer::ui::widgets::button_opts(
            ui,
            t!("diff2"),
            crate::image_viewer::ui::widgets::ButtonOpts {
                active: diff2_selected,
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            if state.diff_image2_id == Some(id) {
                state.diff_image2_id = None;
            } else {
                state.diff_image2_id = Some(id);
                if state.diff_image1_id == Some(id) {
                    state.diff_image1_id = None;
                }
            }
        }
    });
}
