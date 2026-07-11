use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::{FontData, MiData};

pub fn draw_font_panel(ctx: &egui::Context, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::FONT(font_data)) = &image.midata else {
        return;
    };

    let fg = ctx.style().visuals.text_color();
    let bg = ctx.style().visuals.panel_fill;
    let text_color = icu_lib::mirx::Color {
        r: fg.r(),
        g: fg.g(),
        b: fg.b(),
        a: fg.a(),
    };

    let tint_image = |img: &icu_lib::image::RgbaImage| -> icu_lib::image::RgbaImage {
        let mut out = img.clone();
        for px in out.pixels_mut() {
            let v = px.0[0] as f32 / 255.0;
            px.0[0] = (bg.r() as f32 * (1.0 - v) + fg.r() as f32 * v) as u8;
            px.0[1] = (bg.g() as f32 * (1.0 - v) + fg.g() as f32 * v) as u8;
            px.0[2] = (bg.b() as f32 * (1.0 - v) + fg.b() as f32 * v) as u8;
            px.0[3] = 255;
        }
        out
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
                        text_color,
                    );
                    state.font_rendered_preview = Some(img);
                }
                ui.separator();
                ui.label("Glyph diff:");
                if ui.button("Select font to diff...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Font", &["ttf", "otf", "ttc", "mirx"])
                        .pick_file()
                    {
                        state.font_diff_path = Some(path.to_string_lossy().into());
                    }
                }
                if let Some(diff_path) = &state.font_diff_path {
                    ui.label(format!("vs: {}", diff_path));
                    if ui.button("Render Diff").clicked() {
                        let img_a = icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                        let raw_b = std::fs::read(diff_path).unwrap_or_default();
                        let ed = icu_lib::endecoder::mirui::Mirx;
                        if ed.can_decode(&raw_b) {
                            match ed.decode(raw_b) {
                                icu_lib::midata::MiData::FONT(icu_lib::midata::FontData::Mirx(font_b)) => {
                                    let img_b = icu_lib::endecoder::mirui::font_render::render_font_atlas(&font_b);
                                    let (wa, ha) = img_a.dimensions();
                                    let (wb, hb) = img_b.dimensions();
                                    let (w, h) = (wa.max(wb), ha.max(hb));
                                    let mut canvas_a = icu_lib::image::RgbaImage::new(w, h);
                                    icu_lib::image::imageops::overlay(&mut canvas_a, &img_a, 0, 0);
                                    let mut canvas_b = icu_lib::image::RgbaImage::new(w, h);
                                    icu_lib::image::imageops::overlay(&mut canvas_b, &img_b, 0, 0);
                                    let dr = icu_lib::endecoder::utils::diff::diff_image(
                                        &icu_lib::midata::MiData::RGBA(canvas_a.clone()),
                                        &icu_lib::midata::MiData::RGBA(canvas_b),
                                    );
                                    if let Some(dr) = dr {
                                        let mut stack = icu_lib::postprocess::OverlayStack::new(canvas_a);
                                        stack.push(Box::new(icu_lib::postprocess::DiffOverlay::new(dr, 1.0, 0.5)));
                                        let composited = stack.composite().clone();
                                        state.font_rendered_preview = Some(composited);
                                        state.font_view_mode = "rendered".into();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
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
        ui.horizontal(|ui| {
            ui.radio_value(&mut state.font_view_mode, "atlas".into(), "Atlas");
            ui.radio_value(&mut state.font_view_mode, "rendered".into(), "Rendered");
            ui.radio_value(&mut state.font_view_mode, "grid".into(), "Glyph Grid");
        });
        ui.separator();

        match state.font_view_mode.as_str() {
            "rendered" => {
                if state.font_rendered_preview.is_none() {
                    if let FontData::Mirx(font) = font_data {
                        let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                            font,
                            &state.font_preview_text,
                            400,
                            64,
                            text_color,
                        );
                        state.font_rendered_preview = Some(img);
                    } else if let FontData::FreeType(_) = font_data {
                    }
                }
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
                    ui.label("Rendering not available for this font type");
                }
            }
            "grid" => {
                match font_data {
                    FontData::Mirx(font) => {
                        let cell = font.atlas.source_size as usize + 4;
                        let cols = 16usize;
                        egui::TopBottomPanel::bottom("glyph_detail").show_inside(ui, |ui| {
                            if let Some(idx) = state.font_selected_glyph {
                                if let Some(m) = font.metrics.get(idx) {
                                    let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                    ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, m.codepoint));
                                    ui.label(format!("advance: {}", m.advance));
                                    ui.label(format!("bearing: ({}, {})", m.bearing_x, m.bearing_y));
                                    let big = icu_lib::endecoder::mirui::font_render::render_font_text(
                                        font, &ch.to_string(), 128, 128, text_color,
                                    );
                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                        [128, 128], big.as_raw(),
                                    );
                                    let tex = ui.ctx().load_texture(
                                        format!("glyph_big_{}", idx),
                                        color_image, egui::TextureOptions::LINEAR,
                                    );
                                    ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                                }
                            } else {
                                ui.label("Click a glyph to inspect");
                            }
                        });
                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("glyph_grid")
                                .num_columns(cols)
                                .spacing([2.0, 2.0])
                                .show(ui, |ui| {
                                    for (i, m) in font.metrics.iter().enumerate() {
                                        let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                        let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                                            font, &ch.to_string(), cell as u32, cell as u32, text_color,
                                        );
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                            [cell, cell], img.as_raw(),
                                        );
                                        let tex_id = ui.ctx().load_texture(
                                            format!("glyph_{}", i),
                                            color_image, egui::TextureOptions::LINEAR,
                                        ).id();
                                        let resp = ui.add(egui::Button::image(egui::load::SizedTexture::new(tex_id, [cell as f32; 2])));
                                        if resp.clicked() {
                                            state.font_selected_glyph = Some(i);
                                        }
                                        if state.font_selected_glyph == Some(i) {
                                            ui.painter().rect_stroke(resp.rect, 0.0, egui::Stroke::new(2.0, egui::Color32::CYAN), egui::StrokeKind::Outside);
                                        }
                                        if (i + 1) % cols == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                    }
                    FontData::FreeType(f) => {
                        let cell = 48u32;
                        let cols = 16usize;
                        egui::TopBottomPanel::bottom("glyph_detail_ft").show_inside(ui, |ui| {
                            if let Some(idx) = state.font_selected_glyph {
                                if let Some(g) = f.glyphs.get(idx) {
                                    let ch = char::from_u32(g.codepoint).unwrap_or('?');
                                    ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, g.codepoint));
                                    ui.label(format!("advance: {}", g.advance));
                                    ui.label(format!("bearing: ({}, {})", g.bearing_x, g.bearing_y));
                                    ui.label(format!("bbox: {:?}", g.bbox));
                                    ui.label(format!("outline cmds: {}", g.outline.len()));
                                    if let Some(img) = icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                                        f, ch, 128, 128, text_color,
                                    ) {
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                            [128, 128], img.as_raw(),
                                        );
                                        let tex = ui.ctx().load_texture(
                                            format!("ft_glyph_big_{}", idx),
                                            color_image, egui::TextureOptions::LINEAR,
                                        );
                                        ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                                    }
                                }
                            } else {
                                ui.label("Click a glyph to inspect");
                            }
                        });
                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("glyph_grid_ft")
                                .num_columns(cols)
                                .spacing([2.0, 2.0])
                                .show(ui, |ui| {
                                    for (i, g) in f.glyphs.iter().enumerate() {
                                        let ch = char::from_u32(g.codepoint).unwrap_or('?');
                                        if let Some(img) = icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                                            f, ch, cell, cell, text_color,
                                        ) {
                                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                                [cell as usize, cell as usize],
                                                img.as_raw(),
                                            );
                                            let tex_id = ui.ctx().load_texture(
                                                format!("ft_glyph_{}", i),
                                                color_image, egui::TextureOptions::LINEAR,
                                            ).id();
                                            let resp = ui.add(egui::Button::image(egui::load::SizedTexture::new(tex_id, [cell as f32; 2])));
                                            if resp.clicked() {
                                                state.font_selected_glyph = Some(i);
                                            }
                                            if state.font_selected_glyph == Some(i) {
                                                ui.painter().rect_stroke(resp.rect, 0.0, egui::Stroke::new(2.0, egui::Color32::CYAN), egui::StrokeKind::Outside);
                                            }
                                        } else {
                                            ui.label(format!("{}", ch));
                                        }
                                        if (i + 1) % cols == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                    }
                }
            }
            _ => {
                let theme_key = format!(
                    "{:?}_{:?}",
                    fg,
                    bg
                );
                let (image_data, w, h) = if let Some((ref cached_key, ref cached_data, cw, ch)) =
                    state.font_atlas_cached
                {
                    if *cached_key == theme_key {
                        (cached_data.clone(), cw, ch)
                    } else {
                        let rendered = match font_data {
                            FontData::Mirx(font) => {
                                let atlas_img =
                                    icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                                tint_image(&atlas_img)
                            }
                            FontData::FreeType(f) => {
                                icu_lib::endecoder::mirui::font_render::render_freetype_glyphs(
                                    f, text_color,
                                )
                            }
                        };
                        let w = rendered.width();
                        let h = rendered.height();
                        let data: Vec<Color32> = rendered
                            .chunks(4)
                            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        state.font_atlas_cached = Some((theme_key, data.clone(), w, h));
                        (data, w, h)
                    }
                } else {
                    let rendered = match font_data {
                        FontData::Mirx(font) => {
                            let atlas_img =
                                icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                            tint_image(&atlas_img)
                        }
                        FontData::FreeType(f) => {
                            icu_lib::endecoder::mirui::font_render::render_freetype_glyphs(
                                f, text_color,
                            )
                        }
                    };
                    let w = rendered.width();
                    let h = rendered.height();
                    let data: Vec<Color32> = rendered
                        .chunks(4)
                        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                        .collect();
                    state.font_atlas_cached = Some((theme_key, data.clone(), w, h));
                    (data, w, h)
                };
                let tint_item = crate::image_viewer::model::ImageItem {
                    path: image.path.clone(),
                    info: image.info.clone(),
                    width: w,
                    height: h,
                    image_data,
                    midata: None,
                };
                let mut plotter = ImagePlotter::new("font_atlas")
                    .anti_alias(state.context.anti_alias)
                    .show_grid(state.context.show_grid);
                plotter.show(ui, &Some(tint_item));
            }
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
        let highlight = if let Some(idx) = state.path_selected_op {
            if let Some(op) = scene_data.scene.ops.get(idx) {
                op_center(op)
            } else {
                None
            }
        } else {
            None
        };
        let mut plotter = ImagePlotter::new("path_preview")
            .anti_alias(state.context.anti_alias)
            .show_grid(state.context.show_grid)
            .highlight(highlight);
        plotter.show(ui, &Some(image.clone()));
    });
}

fn op_center(op: &icu_lib::mirx::SceneOp) -> Option<[u32; 2]> {
    use icu_lib::mirx::SceneOp;
    match op {
        SceneOp::FillPath { path, .. } | SceneOp::StrokePath { path, .. } => {
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for cmd in &path.cmds {
                let p = match cmd {
                    icu_lib::mirx::PathCmd::MoveTo(p) | icu_lib::mirx::PathCmd::LineTo(p) => *p,
                    icu_lib::mirx::PathCmd::QuadTo { end, .. } => *end,
                    icu_lib::mirx::PathCmd::CubicTo { end, .. } => *end,
                    icu_lib::mirx::PathCmd::Close => continue,
                };
                let x = p.x.to_int();
                let y = p.y.to_int();
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            if min_x <= max_x && min_y <= max_y {
                Some([((min_x + max_x) / 2) as u32, ((min_y + max_y) / 2) as u32])
            } else {
                None
            }
        }
        SceneOp::FillRect { area, .. } | SceneOp::Border { area, .. } => {
            let cx = (area.x.to_int() + area.w.to_int()) / 2;
            let cy = (area.y.to_int() + area.h.to_int()) / 2;
            Some([cx as u32, cy as u32])
        }
        SceneOp::Line { p1, p2, .. } => {
            let cx = (p1.x.to_int() + p2.x.to_int()) / 2;
            let cy = (p1.y.to_int() + p2.y.to_int()) / 2;
            Some([cx as u32, cy as u32])
        }
        SceneOp::Arc { center, .. } => Some([center.x.to_int() as u32, center.y.to_int() as u32]),
        _ => None,
    }
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

    egui::SidePanel::left("indexed_left").show(ctx, |ui| {
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
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .set_file_name("indexed.png")
                .save_file()
            {
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
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("bin", &["bin"])
                    .set_file_name("indexed.bin")
                    .save_file()
                {
                    let _ = std::fs::write(&path, bytes);
                }
            }
        }
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
