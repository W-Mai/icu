pub mod convert_panel;
pub mod diff_panel;
pub mod image_list;
pub mod layout;
pub mod panels;
pub mod theme;
pub mod viewer;
#[allow(dead_code)]
pub mod widgets;

pub use image_list::draw_left_panel;
pub use layout::{draw_bottom_panel, draw_top_panel};
pub use viewer::draw_central_panel;

use crate::image_viewer::model::{RightTab, ViewerState};
use eframe::egui;
use std::time::Duration;

pub fn draw_right_panel_container(ui: &mut egui::Ui, state: &mut ViewerState) {
    let frame = theme::side_panel_frame(ui.ctx());
    egui::Panel::right("RightPanel")
        .resizable(true)
        .frame(frame)
        .show(ui, |ui| {
            if ui.rect_contains_pointer(ui.max_rect())
                && ui.input(|input| input.pointer.any_pressed())
            {
                state.blur_list();
            }
            let p = theme::tokens::palette(ui.ctx());
            egui::Frame::new()
                .fill(p.mantle)
                .stroke(egui::Stroke::NONE)
                .inner_margin(egui::Margin::same(0))
                .show(ui, |ui| {
                    let prev_tab = state.context.right_tab;
                    let info_label = t!("tab_info").to_string();
                    let convert_label = t!("tab_convert").to_string();
                    let diff_label = t!("tab_diff").to_string();
                    let tabs = [
                        (RightTab::Info, info_label.as_str()),
                        (RightTab::Convert, convert_label.as_str()),
                        (RightTab::Diff, diff_label.as_str()),
                    ];
                    widgets::mode_tabs(ui, &mut state.context.right_tab, &tabs);
                    match (prev_tab, state.context.right_tab) {
                        (RightTab::Diff, new) if new != RightTab::Diff => {
                            state.context.diff_active = false;
                        }
                        (_, RightTab::Diff) => {
                            state.context.diff_active = true;
                        }
                        _ => {}
                    }
                });
            ui.painter().line_segment(
                [
                    egui::pos2(ui.min_rect().left(), ui.min_rect().bottom()),
                    egui::pos2(ui.max_rect().right(), ui.min_rect().bottom()),
                ],
                egui::Stroke::new(1.0, p.surface1),
            );

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| match state.context.right_tab {
                            RightTab::Info => draw_info_tab(ui, state),
                            RightTab::Convert => draw_convert_tab(ui, state),
                            RightTab::Diff => diff_panel::draw_diff_panel_contents(ui, state),
                        });
                });
        });
}

fn draw_convert_tab(ui: &mut egui::Ui, state: &mut ViewerState) {
    match viewer::get_content_type(state) {
        viewer::ContentType::Rgba => convert_panel::draw_convert_options(ui, state),
        viewer::ContentType::Font => panels::font_panel::draw_font_convert_section(ui, state),
        viewer::ContentType::Path => panels::path_panel::draw_path_export_section(ui, state),
        viewer::ContentType::Indexed => {
            panels::indexed_panel::draw_indexed_convert_section(ui, state)
        }
        viewer::ContentType::Glyph => panels::font_panel::draw_glyph_convert_section(ui, state),
    }
}

fn draw_info_tab(ui: &mut egui::Ui, state: &mut ViewerState) {
    use crate::image_viewer::model::SidebarItem;
    let item = match state.selected_item().cloned() {
        Some(item) => item,
        None => {
            ui.label(t!("no_file_selected"));
            return;
        }
    };

    match &item {
        SidebarItem::Image(img) => {
            widgets::section_card(ui, t!("section_file_info").as_ref(), |ui| {
                widgets::info_row(ui, t!("name").as_ref(), &img.path);
                widgets::info_row(ui, t!("width").as_ref(), &img.width.to_string());
                widgets::info_row(ui, t!("height").as_ref(), &img.height.to_string());
                widgets::info_row(ui, t!("format").as_ref(), &img.info.format);
                widgets::info_row(
                    ui,
                    t!("size").as_ref(),
                    &format!("{} bytes", img.info.data_size),
                );
                if img.frame_count() > 1 {
                    let total = img
                        .total_duration()
                        .map(|d| format_duration(d))
                        .unwrap_or_else(|| "0s".to_string());
                    widgets::info_row(
                        ui,
                        t!("info_frames").as_ref(),
                        &format!("{} · {}", img.frame_count(), total),
                    );
                    let mut autoplay = img.autoplay();
                    if widgets::toggle_labeled(ui, t!("info_autoplay"), &mut autoplay).changed() {
                        if let Some(current) = state.current_image_mut() {
                            current.set_autoplay(autoplay);
                        }
                    }
                }
            });

            if let Some(icu_lib::midata::MiData::PATH(scene_data)) = &img.midata {
                ui.add_space(8.0);
                widgets::section_card(ui, t!("section_scene").as_ref(), |ui| {
                    ui.label(format!("ops: {}", scene_data.scene.ops.len()));
                });
                if let Some(idx) = state.selected_op {
                    if let Some(op) = scene_data.scene.ops.get(idx) {
                        ui.add_space(8.0);
                        widgets::section_card(
                            ui,
                            &format!("Op #{}: {}", idx, panels::path_panel::op_label(op)),
                            |ui| {
                                panels::path_panel::op_inspector(ui, op);
                            },
                        );
                    }
                }
            }

            if let Some(icu_lib::midata::MiData::FONT(font_data)) = &img.midata {
                ui.add_space(8.0);
                match font_data {
                    icu_lib::midata::FontData::Mirx(font) => {
                        widgets::section_card(ui, t!("section_font_metadata").as_ref(), |ui| {
                            widgets::info_row(
                                ui,
                                t!("kind").as_ref(),
                                &format!("{:?}", font.chunk_header.kind),
                            );
                            widgets::info_row(
                                ui,
                                t!("source_size").as_ref(),
                                &font.atlas.source_size.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("bit_depth").as_ref(),
                                &font.atlas.bit_depth.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("glyphs").as_ref(),
                                &font.atlas.glyph_count.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("ascender").as_ref(),
                                &font.atlas.ascender.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("descender").as_ref(),
                                &font.atlas.descender.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("line_height").as_ref(),
                                &font.atlas.line_height.to_string(),
                            );
                        });
                    }
                    icu_lib::midata::FontData::MirxBundle(fonts) => {
                        widgets::section_card(ui, t!("section_font_bundle").as_ref(), |ui| {
                            widgets::info_row(ui, t!("fonts").as_ref(), &fonts.len().to_string());
                        });
                    }
                    icu_lib::midata::FontData::FreeType(f) => {
                        widgets::section_card(ui, t!("section_freetype_metadata").as_ref(), |ui| {
                            widgets::info_row(ui, t!("family").as_ref(), &f.family);
                            widgets::info_row(ui, t!("style").as_ref(), &f.style);
                            widgets::info_row(
                                ui,
                                t!("units_per_em").as_ref(),
                                &f.units_per_em.to_string(),
                            );
                            widgets::info_row(ui, t!("ascender").as_ref(), &f.ascender.to_string());
                            widgets::info_row(
                                ui,
                                t!("descender").as_ref(),
                                &f.descender.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("line_height").as_ref(),
                                &f.line_height.to_string(),
                            );
                            widgets::info_row(
                                ui,
                                t!("glyphs").as_ref(),
                                &format!("{} / {}", f.glyphs.len(), f.glyph_count),
                            );
                        });
                    }
                }
                ui.add_space(8.0);
                panels::font_panel::draw_font_info_section(ui, state);
            }

            if let Some(icu_lib::midata::MiData::INDEXED(indexed)) = &img.midata {
                ui.add_space(8.0);
                widgets::section_card(ui, t!("section_indexed_info").as_ref(), |ui| {
                    widgets::info_row(ui, t!("bpp").as_ref(), &indexed.bpp.to_string());
                    widgets::info_row(
                        ui,
                        t!("palette").as_ref(),
                        &indexed.palette.len().to_string(),
                    );
                    widgets::info_row(
                        ui,
                        t!("size").as_ref(),
                        &format!("{}×{}", indexed.width, indexed.height),
                    );
                });
                ui.add_space(8.0);
                panels::indexed_panel::draw_indexed_info_section(ui, state);
            }

            if !img.info.other_info.is_null() {
                ui.add_space(8.0);
                widgets::section_card(ui, t!("section_metadata").as_ref(), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            crate::image_viewer::ui::viewer::ui_tree_view(ui, &img.info.other_info);
                        });
                });
            }
        }
        SidebarItem::Glyph(g) => {
            widgets::section_card(ui, t!("section_glyph_properties").as_ref(), |ui| {
                let source_atlas = t!("source_atlas_approximate").to_string();
                let source_freetype = t!("source_freetype_true_vector").to_string();
                widgets::info_row(
                    ui,
                    t!("codepoint").as_ref(),
                    &format!("U+{:04X}", g.codepoint),
                );
                widgets::info_row(ui, t!("character").as_ref(), &g.char_repr);
                widgets::info_row(ui, t!("advance").as_ref(), &format!("{}px", g.advance));
                widgets::info_row(ui, t!("bearing").as_ref(), &format!("{:?}", g.bearing));
                widgets::info_row(ui, t!("bbox").as_ref(), &format!("{:?}", g.bbox));
                widgets::info_row(
                    ui,
                    t!("outline_cmds").as_ref(),
                    &g.outline.len().to_string(),
                );
                widgets::info_row(
                    ui,
                    t!("source").as_ref(),
                    if g.outline_approximate {
                        source_atlas.as_str()
                    } else {
                        source_freetype.as_str()
                    },
                );
            });
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis >= 1000 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", millis)
    }
}
