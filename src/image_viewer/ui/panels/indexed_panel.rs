use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::MiData;

pub fn draw_indexed_panel(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    if state.context.image_diff {
        return;
    }
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::INDEXED(indexed)) = &image.midata else {
        return;
    };
    let indexed = indexed.clone();

    let indexed = if state.indexed_dither > 0 {
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
    };

    let mut hovered_palette: Option<u8> = None;

    egui::Panel::left("indexed_left").show(ui, |ui| {
        ui.heading("Indexed");
        ui.label(format!("bpp: {}", indexed.bpp));
        ui.label(format!("palette: {}", indexed.palette.len()));
        ui.label(format!("size: {}x{}", indexed.width, indexed.height));
        ui.separator();
        let prev_quality = state.indexed_show_quality;
        ui.checkbox(&mut state.indexed_show_quality, "Quality view");
        if state.indexed_show_quality != prev_quality {
            state.indexed_hover_palette = None;
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Dither:");
            ui.add(egui::Slider::new(&mut state.indexed_dither, 0..=30).text("level"));
        });
        ui.separator();
        ui.label("Palette (hover to highlight, click to edit):");
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
        ui.separator();
        if ui.button("Export PNG").clicked() {
            let img = indexed.rgba.clone();
            if let Some(path) = super::pick_save_file(&[("PNG", &["png"])], &"indexed.png") {
                let _ = img.save(&path);
            }
        }
        if ui.button("Export LVGL").clicked() {
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

    egui::CentralPanel::default().show(ui, |ui| {
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
        plotter.show(ui, &Some(view_item));
    });
}
