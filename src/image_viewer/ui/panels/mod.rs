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
                ui.separator();
                ui.label("Merge fonts:");
                if ui.button("Add font file...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("mirx", &["mirx"])
                        .pick_file()
                    {
                        state.merge_font_paths.push(path.to_string_lossy().into());
                    }
                }
                for (i, p) in state.merge_font_paths.clone().iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}", p));
                        if ui.button("×").clicked() {
                            state.merge_font_paths.remove(i);
                        }
                    });
                }
                if state.merge_font_paths.len() >= 2 && ui.button("Merge & Save").clicked() {
                    let inputs: Vec<Vec<u8>> = state.merge_font_paths
                        .iter()
                        .filter_map(|p| std::fs::read(p).ok())
                        .collect();
                    let merged = icu_lib::endecoder::mirui::font_bake::merge_font_chunks(&inputs);
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("mirx", &["mirx"])
                        .set_file_name("bundle.mirx")
                        .save_file()
                    {
                        let _ = std::fs::write(&path, merged);
                    }
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
                ui.separator();
                ui.label("Bake to mirx:");
                ui.horizontal(|ui| {
                    ui.label("size:");
                    ui.add(egui::DragValue::new(&mut state.font_bake_size).range(8..=64));
                    ui.label("format:");
                    egui::ComboBox::from_label("")
                        .selected_text(&state.font_bake_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.font_bake_format, "sdf".into(), "sdf");
                            ui.selectable_value(&mut state.font_bake_format, "gray".into(), "gray");
                        });
                });
                if ui.button("Bake & Save").clicked() {
                    let kind = if state.font_bake_format == "gray" {
                        icu_lib::mirx::FontChunkKind::Grayscale
                    } else {
                        icu_lib::mirx::FontChunkKind::Sdf
                    };
                    let charset: String = (0x20u32..=0x7Eu32)
                        .filter_map(char::from_u32)
                        .collect();
                    let params = icu_lib::endecoder::mirui::font_bake::FontBakeParams {
                        kind,
                        source_size: state.font_bake_size,
                        bit_depth: if kind == icu_lib::mirx::FontChunkKind::Sdf { 4 } else { 4 },
                        spread: (state.font_bake_size / 4).max(1),
                        charset: charset.chars().collect(),
                    };
                    let raw = std::fs::read(&image.path).unwrap_or_default();
                    if let Some(font) = icu_lib::endecoder::mirui::font_bake::bake_font(&raw, &params) {
                        let payload = font.encode();
                        let bytes = icu_lib::mirx::encode_chunk_generic(
                            icu_lib::mirx::chunk_type::FONT,
                            icu_lib::mirx::ChunkEntry::FLAG_CRITICAL,
                            &payload,
                        );
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("mirx", &["mirx"])
                            .set_file_name(format!("{}_{}.mirx", f.family, state.font_bake_format))
                            .save_file()
                        {
                            let _ = std::fs::write(&path, bytes);
                        }
                    }
                }
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
    let scene_data = scene_data.clone();

    egui::SidePanel::left("path_left").show(ctx, |ui| {
        ui.heading("Scene");
        ui.label(format!("ops: {}", scene_data.scene.ops.len()));
        ui.separator();
        if ui.button("Export PNG").clicked() {
            let (w, h) = icu_lib::endecoder::mirui::scene_render::scene_dimensions(&scene_data.scene)
                .unwrap_or((256, 256));
            let img = icu_lib::endecoder::mirui::scene_render::render_scene(&scene_data.scene, w, h);
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .set_file_name("scene.png")
                .save_file()
            {
                let _ = img.save(&path);
            }
        }
        if ui.button("Export SVG").clicked() {
            let svg = icu_lib::endecoder::svg::export::scene_to_svg(&scene_data.scene, 0, 0);
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SVG", &["svg"])
                .set_file_name("scene.svg")
                .save_file()
            {
                let _ = std::fs::write(&path, svg);
            }
        }
        if ui.button("Export mirx").clicked() {
            let payload = scene_data.scene.encode().unwrap_or_default();
            let bytes = icu_lib::mirx::encode_chunk_generic(
                icu_lib::mirx::chunk_type::VECTOR,
                icu_lib::mirx::ChunkEntry::FLAG_CRITICAL,
                &payload,
            );
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("mirx", &["mirx"])
                .set_file_name("scene.mirx")
                .save_file()
            {
                let _ = std::fs::write(&path, bytes);
            }
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, op) in scene_data.scene.ops.iter().enumerate() {
                let label = op_label(op);
                if ui
                    .selectable_label(state.path_selected_op == Some(i), format!("{}. {}", i, label))
                    .clicked()
                {
                    state.path_selected_op = Some(i);
                }
            }
        });
    });

    egui::SidePanel::right("path_right").show(ctx, |ui| {
        if let Some(idx) = state.path_selected_op {
            if let Some(op) = scene_data.scene.ops.get(idx) {
                ui.heading(format!("Op #{}: {}", idx, op_label(op)));
                ui.separator();
                op_inspector(ui, op);
            }
        }
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut plotter = ImagePlotter::new("path_preview")
            .anti_alias(state.context.anti_alias)
            .show_grid(state.context.show_grid);
        plotter.show(ui, &Some(image.clone()));
    });
}

fn op_label(op: &icu_lib::mirx::SceneOp) -> &'static str {
    match op {
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
        icu_lib::mirx::SceneOp::PushClip { .. } => "PushClip",
        icu_lib::mirx::SceneOp::PopClip => "PopClip",
    }
}

fn op_inspector(ui: &mut egui::Ui, op: &icu_lib::mirx::SceneOp) {
    use icu_lib::mirx::SceneOp;
    match op {
        SceneOp::FillPath { paint, opa, fill_rule, .. } => {
            ui.label(format!("paint: {:?}", paint));
            ui.label(format!("opa: {}", opa));
            ui.label(format!("fill_rule: {:?}", fill_rule));
        }
        SceneOp::StrokePath { paint, width, opa, line_cap, line_join, miter_limit, dash, .. } => {
            ui.label(format!("paint: {:?}", paint));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
            ui.label(format!("cap: {:?}", line_cap));
            ui.label(format!("join: {:?}", line_join));
            ui.label(format!("miter_limit: {}", miter_limit.to_f32()));
            if !dash.is_empty() {
                let s: Vec<String> = dash.iter().map(|d| d.to_f32().to_string()).collect();
                ui.label(format!("dash: [{}]", s.join(", ")));
            }
        }
        SceneOp::FillRect { area, color, radius, opa, .. } => {
            ui.label(format!("area: ({},{},{},{})", area.x.to_f32(), area.y.to_f32(), area.w.to_f32(), area.h.to_f32()));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Border { area, color, width, radius, opa, .. } => {
            ui.label(format!("area: ({},{},{},{})", area.x.to_f32(), area.y.to_f32(), area.w.to_f32(), area.h.to_f32()));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Line { p1, p2, color, width, opa, .. } => {
            ui.label(format!("p1: ({},{})", p1.x.to_f32(), p1.y.to_f32()));
            ui.label(format!("p2: ({},{})", p2.x.to_f32(), p2.y.to_f32()));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Arc { center, radius, start_angle, end_angle, color, width, opa, .. } => {
            ui.label(format!("center: ({},{})", center.x.to_f32(), center.y.to_f32()));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!("angles: {}° - {}°", start_angle.to_f32(), end_angle.to_f32()));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::GroupBegin { transform, opacity, .. } => {
            if let Some(t) = transform {
                ui.label(format!("transform: [{},{},{}/{},{},{}]", t.m00.to_f32(), t.m01.to_f32(), t.tx.to_f32(), t.m10.to_f32(), t.m11.to_f32(), t.ty.to_f32()));
            } else {
                ui.label("transform: identity");
            }
            ui.label(format!("opacity: {:?}", opacity));
        }
        SceneOp::Label { text, color, opa, .. } => {
            ui.label(format!("text: {:?}", text));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::PushClip { fill_rule, .. } => {
            ui.label(format!("fill_rule: {:?}", fill_rule));
        }
        _ => {}
    }
}

pub fn draw_indexed_panel(ctx: &egui::Context, state: &mut crate::image_viewer::model::ViewerState) {
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

    egui::SidePanel::left("indexed_left").show(ctx, |ui| {
        ui.heading("Indexed");
        ui.label(format!("bpp: {}", indexed.bpp));
        ui.label(format!("palette: {}", indexed.palette.len()));
        ui.label(format!("size: {}x{}", indexed.width, indexed.height));
        ui.separator();
        ui.checkbox(&mut state.indexed_show_quality, "Quality view");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Dither:");
            ui.add(egui::Slider::new(&mut state.indexed_dither, 0..=30).text("level"));
        });
        ui.separator();
        ui.label("Palette (click to edit):");
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
                    let btn = ui.add(
                        egui::Button::new(format!("{}", i))
                            .fill(c)
                            .selected(selected),
                    );
                    if btn.hovered() {
                        state.indexed_hover_palette = Some(i as u8);
                    }
                    if btn.clicked() {
                        let mut picked = egui::Rgba::from_rgb(color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0);
                        egui::color_picker::color_edit_button_rgba(ui, &mut picked, egui::color_picker::Alpha::Opaque);
                        let mut idx = indexed.clone();
                        idx.palette[i] = [(picked.r() * 255.0) as u8, (picked.g() * 255.0) as u8, (picked.b() * 255.0) as u8, (picked.a() * 255.0) as u8];
                        for px in 0..idx.indexes.len() {
                            if idx.indexes[px] == i as u8 {
                                let p = idx.palette[i];
                                let rgba = idx.rgba.get_pixel_mut(px as u32 % idx.width, px as u32 / idx.width);
                                rgba.0 = [p[0], p[1], p[2], p[3]];
                            }
                        }
                    }
                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
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
