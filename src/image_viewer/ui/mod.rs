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
pub use viewer::{draw_central_panel, draw_image_info};

use crate::image_viewer::model::{RightTab, ViewerState};
use eframe::egui;

pub fn draw_right_panel_container(ui: &mut egui::Ui, state: &mut ViewerState) {
    let frame = theme::side_panel_frame(ui.ctx());
    egui::Panel::right("RightPanel")
        .exact_size(300.0)
        .resizable(true)
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let tabs = [(RightTab::Info, "Info"), (RightTab::Convert, "Convert"), (RightTab::Diff, "Diff")];
                widgets::mode_tabs(ui, &mut state.context.right_tab, &tabs);
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match state.context.right_tab {
                    RightTab::Info => {
                        draw_info_tab(ui, state);
                    }
                    RightTab::Convert => {
                        convert_panel::draw_convert_options(ui, state);
                    }
                    RightTab::Diff => {
                        diff_panel::draw_diff_panel_contents(ui, state);
                    }
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
