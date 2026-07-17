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

pub fn draw_right_panel_container(ui: &mut egui::Ui, state: &mut ViewerState) {
    let frame = theme::side_panel_frame(ui.ctx());
    egui::Panel::right("RightPanel")
        .exact_size(300.0)
        .resizable(false)
        .frame(frame)
        .show(ui, |ui| {
            let p = theme::tokens::palette(ui.ctx());
            egui::Frame::new()
                .fill(p.surface0)
                .stroke(egui::Stroke::new(1.0, p.surface1))
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 4,
                    bottom: 4,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let prev_tab = state.context.right_tab;
                        let tabs = [
                            (RightTab::Info, "Info"),
                            (RightTab::Convert, "Convert"),
                            (RightTab::Diff, "Diff"),
                        ];
                        widgets::mode_tabs(ui, &mut state.context.right_tab, &tabs);
                        if prev_tab == RightTab::Diff && state.context.right_tab != RightTab::Diff {
                            state.context.diff_active = false;
                        }
                    });
                });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.allocate_space(egui::vec2(12.0, 0.0));
                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| match state.context.right_tab {
                            RightTab::Info => draw_info_tab(ui, state),
                            RightTab::Convert => convert_panel::draw_convert_options(ui, state),
                            RightTab::Diff => diff_panel::draw_diff_panel_contents(ui, state),
                        });
                });
        });
}

fn draw_info_tab(ui: &mut egui::Ui, state: &mut ViewerState) {
    use crate::image_viewer::model::SidebarItem;
    let idx = match state.selected_index {
        Some(i) => i,
        None => {
            ui.label("No file selected");
            return;
        }
    };
    let item = match state.items.get(idx) {
        Some(it) => it,
        None => {
            ui.label("No file selected");
            return;
        }
    };

    match item {
        SidebarItem::Image(img) => {
            widgets::section_card(ui, "File Info", |ui| {
                widgets::info_row(ui, "Name", &img.path);
                widgets::info_row(ui, "Width", &img.width.to_string());
                widgets::info_row(ui, "Height", &img.height.to_string());
                widgets::info_row(ui, "Format", &img.info.format);
                widgets::info_row(ui, "Size", &format!("{} bytes", img.info.data_size));
            });

            if let Some(icu_lib::midata::MiData::PATH(scene_data)) = &img.midata {
                ui.add_space(8.0);
                widgets::section_card(ui, "Scene", |ui| {
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
                        widgets::section_card(ui, "Font Metadata", |ui| {
                            widgets::info_row(ui, "Kind", &format!("{:?}", font.chunk_header.kind));
                            widgets::info_row(
                                ui,
                                "Source Size",
                                &font.atlas.source_size.to_string(),
                            );
                            widgets::info_row(ui, "Bit Depth", &font.atlas.bit_depth.to_string());
                            widgets::info_row(ui, "Glyphs", &font.atlas.glyph_count.to_string());
                            widgets::info_row(ui, "Ascender", &font.atlas.ascender.to_string());
                            widgets::info_row(ui, "Descender", &font.atlas.descender.to_string());
                            widgets::info_row(
                                ui,
                                "Line Height",
                                &font.atlas.line_height.to_string(),
                            );
                        });
                    }
                    icu_lib::midata::FontData::MirxBundle(fonts) => {
                        widgets::section_card(ui, "Font Bundle", |ui| {
                            widgets::info_row(ui, "Fonts", &fonts.len().to_string());
                        });
                    }
                    icu_lib::midata::FontData::FreeType(f) => {
                        widgets::section_card(ui, "FreeType Metadata", |ui| {
                            widgets::info_row(ui, "Family", &f.family);
                            widgets::info_row(ui, "Style", &f.style);
                            widgets::info_row(ui, "Units/em", &f.units_per_em.to_string());
                            widgets::info_row(ui, "Ascender", &f.ascender.to_string());
                            widgets::info_row(ui, "Descender", &f.descender.to_string());
                            widgets::info_row(ui, "Line Height", &f.line_height.to_string());
                            widgets::info_row(
                                ui,
                                "Glyphs",
                                &format!("{} / {}", f.glyphs.len(), f.glyph_count),
                            );
                        });
                    }
                }
            }

            if let Some(icu_lib::midata::MiData::INDEXED(indexed)) = &img.midata {
                ui.add_space(8.0);
                widgets::section_card(ui, "Indexed Info", |ui| {
                    widgets::info_row(ui, "BPP", &indexed.bpp.to_string());
                    widgets::info_row(ui, "Palette", &indexed.palette.len().to_string());
                    widgets::info_row(ui, "Size", &format!("{}×{}", indexed.width, indexed.height));
                });
            }

            if !img.info.other_info.is_null() {
                ui.add_space(8.0);
                widgets::section_card(ui, "Metadata", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            crate::image_viewer::ui::viewer::ui_tree_view(ui, &img.info.other_info);
                        });
                });
            }
        }
        SidebarItem::Glyph(g) => {
            widgets::section_card(ui, "Glyph Properties", |ui| {
                widgets::info_row(ui, "Codepoint", &format!("U+{:04X}", g.codepoint));
                widgets::info_row(ui, "Character", &g.char_repr);
                widgets::info_row(ui, "Advance", &format!("{}px", g.advance));
                widgets::info_row(ui, "Bearing", &format!("{:?}", g.bearing));
                widgets::info_row(ui, "BBox", &format!("{:?}", g.bbox));
                widgets::info_row(ui, "Outline cmds", &g.outline.len().to_string());
                widgets::info_row(
                    ui,
                    "Source",
                    if g.outline_approximate {
                        "atlas (approximate)"
                    } else {
                        "FreeType (true vector)"
                    },
                );
            });
        }
    }
}
