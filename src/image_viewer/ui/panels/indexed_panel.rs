use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::MiData;

fn selected_indexed(
    state: &mut crate::image_viewer::model::ViewerState,
) -> Option<icu_lib::midata::IndexedImageData> {
    if state.context.diff_active {
        return None;
    }
    let indexed = match state.current_image.as_ref()?.midata.as_ref()? {
        MiData::INDEXED(indexed) => indexed.clone(),
        _ => return None,
    };

    Some(if state.indexed_dither > 0 {
        if state.indexed_dither != state.indexed_dither_cached {
            state.indexed_dither_cached = state.indexed_dither;
            state.indexed_requantized =
                icu_lib::midata::requantize_indexed(&indexed, state.indexed_dither);
        }
        state.indexed_requantized.clone().unwrap_or(indexed)
    } else {
        state.indexed_requantized = None;
        state.indexed_dither_cached = u32::MAX;
        indexed
    })
}

pub fn draw_indexed_info_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    let Some(_indexed) = selected_indexed(state) else {
        return;
    };

    crate::image_viewer::ui::widgets::section_card(ui, t!("section_display").as_ref(), |ui| {
        let prev_quality = state.indexed_show_quality;
        crate::image_viewer::ui::widgets::toggle_labeled(
            ui,
            t!("toggle_quality_view"),
            &mut state.indexed_show_quality,
        );
        if state.indexed_show_quality != prev_quality {
            state.indexed_hover_palette = None;
        }
    });
}

pub fn draw_indexed_convert_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    let Some(indexed) = selected_indexed(state) else {
        return;
    };

    let mut hovered_palette: Option<u8> = None;

    crate::image_viewer::ui::widgets::section_card(ui, t!("section_dither").as_ref(), |ui| {
        ui.horizontal(|ui| {
            ui.label(t!("label_level"));
            ui.add(egui::Slider::new(&mut state.indexed_dither, 0..=30).text("level"));
        });
    });
    ui.add_space(4.0);
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_palette").as_ref(), |ui| {
        ui.label(t!("palette_hint"));
        let cols = match indexed.bpp {
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 16,
        };
        let grid_resp = egui::Grid::new("palette_grid")
            .num_columns(cols)
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for (i, color) in indexed.palette.iter().enumerate() {
                    let c = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    let selected = state.indexed_hover_palette == Some(i as u8);
                    let btn = ui.add(
                        egui::Button::new(format!("{}", i))
                            .fill(c)
                            .selected(selected),
                    );
                    if btn.hovered() {
                        hovered_palette = Some(i as u8);
                    }
                    if btn.clicked() {
                        let mut picked = egui::Rgba::from_rgb(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                        );
                        egui::color_picker::color_edit_button_rgba(
                            ui,
                            &mut picked,
                            egui::color_picker::Alpha::Opaque,
                        );
                    }
                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
        if !grid_resp.response.hovered() {
            state.indexed_hover_palette = None;
        } else {
            state.indexed_hover_palette = hovered_palette;
        }
    });
    ui.add_space(4.0);
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_export").as_ref(), |ui| {
        if ui.button(t!("btn_export_png")).clicked() {
            let img = indexed.rgba.clone();
            if let Some(path) = super::pick_save_file(&[("PNG", &["png"])], &"indexed.png") {
                let _ = img.save(&path);
            }
        }
        if ui.button(t!("btn_export_lvgl")).clicked() {
            let cf = match indexed.bpp {
                1 => icu_lib::endecoder::ColorFormat::I1,
                2 => icu_lib::endecoder::ColorFormat::I2,
                4 => icu_lib::endecoder::ColorFormat::I4,
                _ => icu_lib::endecoder::ColorFormat::I8,
            };
            let params = icu_lib::EncoderParams::default().with_color_format(cf);
            let midata = icu_lib::midata::MiData::INDEXED(indexed.clone());
            let bytes = icu_lib::endecoder::lvgl::LVGL {}.encode(&midata, params);
            if !bytes.is_empty() {
                if let Some(path) = super::pick_save_file(&[("bin", &["bin"])], &"indexed.bin") {
                    let _ = std::fs::write(&path, bytes);
                }
            }
        }
    });
}

pub fn draw_indexed_canvas(ui: &mut egui::Ui, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(indexed) = selected_indexed(state) else {
        return;
    };

    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.mantle)
        .stroke(egui::Stroke::new(1.0, p.surface0))
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 4,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::image_viewer::ui::widgets::mode_tabs(
                    ui,
                    &mut state.indexed_view_mode,
                    &[
                        (crate::image_viewer::model::IndexedViewMode::RGBA, t!("tab_rgba").as_ref()),
                        (
                            crate::image_viewer::model::IndexedViewMode::IndexMap,
                            t!("tab_index_map").as_ref(),
                        ),
                    ],
                );
            });
        });
    ui.separator();

    let composited = if state.indexed_show_quality {
        let mut stack = icu_lib::postprocess::OverlayStack::new(indexed.rgba.clone());
        stack.push(Box::new(icu_lib::postprocess::QualityOverlay::new(
            &indexed,
            indexed.rgba.clone(),
        )));
        stack.composite().clone()
    } else if let Some(palette_idx) = state.indexed_hover_palette {
        let mut stack = icu_lib::postprocess::OverlayStack::new(indexed.rgba.clone());
        stack.push(Box::new(icu_lib::postprocess::IndexHoverOverlay::new(
            &indexed,
            palette_idx,
        )));
        stack.composite().clone()
    } else {
        indexed.rgba.clone()
    };
    let w = composited.width();
    let h = composited.height();
    let image_data: Vec<Color32> = composited
        .chunks(4)
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    let view_item = crate::image_viewer::model::ImageItem {
        path: image.path.clone(),
        info: image.info.clone(),
        width: w,
        height: h,
        image_data,
        midata: None,
    };
    let mut plotter = ImagePlotter::new("indexed_view")
        .anti_alias(state.context.anti_alias)
        .show_grid(state.context.show_grid);

    match state.indexed_view_mode {
        crate::image_viewer::model::IndexedViewMode::RGBA => {
            plotter.show(ui, &Some(view_item));
        }
        crate::image_viewer::model::IndexedViewMode::IndexMap => {
            let palette_count = indexed.palette.len().max(1) as u32;
            let max_index = (1u32 << indexed.bpp) - 1;
            let (iw, ih) = (indexed.width, indexed.height);
            let map_data: Vec<Color32> = indexed
                .indexes
                .iter()
                .map(|&idx| {
                    let normalized = if max_index > 0 {
                        idx as u32 * 255 / max_index
                    } else {
                        0
                    };
                    let c = normalized as u8;
                    let pal_color = indexed
                        .palette
                        .get(idx as usize)
                        .copied()
                        .unwrap_or([0, 0, 0, 255]);
                    let pal_fade = 0.4;
                    let r = (c as f32 * pal_fade + pal_color[0] as f32 * (1.0 - pal_fade)) as u8;
                    let g = (c as f32 * pal_fade + pal_color[1] as f32 * (1.0 - pal_fade)) as u8;
                    let b = (c as f32 * pal_fade + pal_color[2] as f32 * (1.0 - pal_fade)) as u8;
                    Color32::from_rgb(r, g, b)
                })
                .collect();
            let map_item = crate::image_viewer::model::ImageItem {
                path: image.path.clone(),
                info: image.info.clone(),
                width: iw,
                height: ih,
                image_data: map_data,
                midata: None,
            };
            let _ = palette_count;
            plotter.show(ui, &Some(map_item));
        }
    }
}
