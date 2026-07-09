use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::midata::{FontData, MiData};

pub fn draw_font_panel(ctx: &egui::Context, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::FONT(font_data)) = &image.midata else {
        return;
    };

    egui::SidePanel::left("font_left").show(ctx, |ui| {
        ui.heading("Font");
        match font_data {
            FontData::Mirx(font) => {
                ui.label(format!("kind: {:?}", font.chunk_header.kind));
                ui.label(format!("source_size: {}", font.atlas.source_size));
                ui.label(format!("bit_depth: {}", font.atlas.bit_depth));
                ui.label(format!("glyphs: {}", font.atlas.glyph_count));
                ui.label(format!("ascender: {}", font.atlas.ascender));
                ui.label(format!("descender: {}", font.atlas.descender));
                ui.label(format!("line_height: {}", font.atlas.line_height));
                ui.separator();
                ui.label("Preview text:");
                ui.text_edit_singleline(&mut state.font_preview_text);
                if ui.button("Render").clicked() {
                    let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                        font,
                        &state.font_preview_text,
                        400,
                        64,
                    );
                    state.font_rendered_preview = Some(img);
                }
            }
            FontData::FreeType(f) => {
                ui.label(format!("family: {}", f.family));
                ui.label(format!("style: {}", f.style));
                ui.label(format!("units_per_em: {}", f.units_per_em));
                ui.label(format!("ascender: {}", f.ascender));
                ui.label(format!("descender: {}", f.descender));
                ui.label(format!("line_height: {}", f.line_height));
                ui.label(format!("glyphs: {} / {}", f.glyphs.len(), f.glyph_count));
            }
        }
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(preview) = &state.font_rendered_preview {
            let w = preview.width();
            let h = preview.height();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                preview.as_raw(),
            );
            let texture = ui
                .ctx()
                .load_texture("font_rendered", color_image, egui::TextureOptions::LINEAR);
            ui.image(egui::load::SizedTexture::new(
                texture.id(),
                [w as f32, h as f32],
            ));
        } else {
            let mut plotter = ImagePlotter::new("font_atlas")
                .anti_alias(state.context.anti_alias)
                .show_grid(state.context.show_grid);
            plotter.show(ui, &Some(image.clone()));
        }
    });
}

pub fn draw_path_panel(ctx: &egui::Context, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::PATH(scene_data)) = &image.midata else {
        return;
    };

    egui::SidePanel::left("path_left").show(ctx, |ui| {
        ui.heading("Scene");
        ui.label(format!("ops: {}", scene_data.scene.ops.len()));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, op) in scene_data.scene.ops.iter().enumerate() {
                let label = match op {
                    icu_lib::mirx::SceneOp::GroupBegin { .. } => "GroupBegin",
                    icu_lib::mirx::SceneOp::GroupEnd => "GroupEnd",
                    icu_lib::mirx::SceneOp::FillPath { .. } => "FillPath",
                    icu_lib::mirx::SceneOp::StrokePath { .. } => "StrokePath",
                    icu_lib::mirx::SceneOp::FillRect { .. } => "FillRect",
                    icu_lib::mirx::SceneOp::Border { .. } => "Border",
                    icu_lib::mirx::SceneOp::Line { .. } => "Line",
                    icu_lib::mirx::SceneOp::Arc { .. } => "Arc",
                    icu_lib::mirx::SceneOp::Label { .. } => "Label",
                    icu_lib::mirx::SceneOp::Blit { .. } => "Blit",
                };
                if ui
                    .selectable_label(state.path_selected_op == Some(i), format!("{}. {}", i, label))
                    .clicked()
                {
                    state.path_selected_op = Some(i);
                }
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut plotter = ImagePlotter::new("path_preview")
            .anti_alias(state.context.anti_alias)
            .show_grid(state.context.show_grid);
        plotter.show(ui, &Some(image.clone()));
    });
}

pub fn draw_indexed_panel(ctx: &egui::Context, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::INDEXED(indexed)) = &image.midata else {
        return;
    };
    let indexed = indexed.clone();

    egui::SidePanel::left("indexed_left").show(ctx, |ui| {
        ui.heading("Indexed");
        ui.label(format!("bpp: {}", indexed.bpp));
        ui.label(format!("palette: {}", indexed.palette.len()));
        ui.label(format!("size: {}x{}", indexed.width, indexed.height));
        ui.separator();
        ui.label("Hover a palette entry:");
        let cols = match indexed.bpp {
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 16,
        };
        egui::Grid::new("palette_grid")
            .num_columns(cols)
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for (i, color) in indexed.palette.iter().enumerate() {
                    let c = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    let selected = state.indexed_hover_palette == Some(i as u8);
                    let resp = ui.add(
                        egui::Button::new(format!("{}", i))
                            .fill(c)
                            .selected(selected),
                    );
                    if resp.hovered() {
                        state.indexed_hover_palette = Some(i as u8);
                    }
                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(palette_idx) = state.indexed_hover_palette {
            let mut stack = icu_lib::postprocess::OverlayStack::new(indexed.rgba.clone());
            stack.push(Box::new(icu_lib::postprocess::IndexHoverOverlay::new(
                &indexed,
                palette_idx,
            )));
            let composited = stack.composite().clone();
            let w = composited.width();
            let h = composited.height();
            let image_data: Vec<Color32> = composited
                .chunks(4)
                .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();
            let hover_item = crate::image_viewer::model::ImageItem {
                path: image.path.clone(),
                info: image.info.clone(),
                width: w,
                height: h,
                image_data,
                midata: None,
            };
            let mut plotter = ImagePlotter::new("indexed_hover")
                .anti_alias(state.context.anti_alias)
                .show_grid(state.context.show_grid);
            plotter.show(ui, &Some(hover_item));
        } else {
            let mut plotter = ImagePlotter::new("indexed_preview")
                .anti_alias(state.context.anti_alias)
                .show_grid(state.context.show_grid);
            plotter.show(ui, &Some(image.clone()));
        }
    });
}
