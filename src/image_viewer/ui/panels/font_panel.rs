use crate::image_viewer::model::{BakeCharsetTab, FontMode};
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::endecoder::EnDecoder;
use icu_lib::midata::{FontData, MiData};

fn parse_charset_text(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn parse_charset_ranges(input: &str) -> Vec<char> {
    let mut out = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (start, end) = if let Some((a, b)) = part.split_once('-') {
            let s = parse_codepoint(a.trim());
            let e = parse_codepoint(b.trim());
            match (s, e) {
                (Some(s), Some(e)) => (s, e),
                _ => continue,
            }
        } else {
            match parse_codepoint(part) {
                Some(c) => (c, c),
                None => continue,
            }
        };
        if start <= end {
            for cp in start..=end {
                if let Some(ch) = char::from_u32(cp) {
                    out.push(ch);
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn parse_codepoint(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("U+").or_else(|| s.strip_prefix("u+")) {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

fn parse_charset_file(path: &str) -> Vec<char> {
    std::fs::read_to_string(path)
        .map(|s| s.chars().collect())
        .unwrap_or_default()
}

fn collect_charset(state: &crate::image_viewer::model::ViewerState) -> Vec<char> {
    match state.font_bake_charset_tab {
        BakeCharsetTab::Text => parse_charset_text(&state.font_bake_charset_text),
        BakeCharsetTab::Range => parse_charset_ranges(&state.font_bake_charset_ranges),
        BakeCharsetTab::File => state
            .font_bake_charset_file
            .as_deref()
            .map(parse_charset_file)
            .unwrap_or_default(),
    }
}

fn selected_mirx_font<'a>(
    font_data: &'a FontData,
    index: usize,
) -> Option<&'a icu_lib::mirx::Font> {
    match font_data {
        FontData::Mirx(font) => Some(font),
        FontData::MirxBundle(fonts) => fonts.get(index).or_else(|| fonts.first()),
        FontData::FreeType(_) => None,
    }
}

fn show_mirx_metadata(ui: &mut egui::Ui, font: &icu_lib::mirx::Font) {
    ui.label(format!("kind: {:?}", font.chunk_header.kind));
    ui.label(format!("source_size: {}", font.atlas.source_size));
    ui.label(format!("bit_depth: {}", font.atlas.bit_depth));
    ui.label(format!("glyphs: {}", font.atlas.glyph_count));
    ui.label(format!("ascender: {}", font.atlas.ascender));
    ui.label(format!("descender: {}", font.atlas.descender));
    ui.label(format!("line_height: {}", font.atlas.line_height));
}

fn reset_font_caches(state: &mut crate::image_viewer::model::ViewerState) {
    state.font_rendered_preview = None;
    state.font_atlas_cached = None;
    state.font_grid_cached = None;
    state.font_grid_big_cached = None;
    state.selected_glyph = None;
}

pub fn draw_font_panel(ui: &mut egui::Ui, state: &mut crate::image_viewer::model::ViewerState) {
    let ctx = ui.ctx().clone();
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::FONT(font_data)) = &image.midata else {
        return;
    };

    let fg = ctx.global_style().visuals.text_color();
    let bg = ctx.global_style().visuals.panel_fill;
    let text_color = icu_lib::mirx::Color {
        r: fg.r(),
        g: fg.g(),
        b: fg.b(),
        a: fg.a(),
    };

    let tint_image = |img: &icu_lib::image::RgbaImage| -> icu_lib::image::RgbaImage {
        let mut out = img.clone();
        for px in out.pixels_mut() {
            let a = px.0[3] as f32 / 255.0;
            px.0[0] = (bg.r() as f32 * (1.0 - a) + fg.r() as f32 * a) as u8;
            px.0[1] = (bg.g() as f32 * (1.0 - a) + fg.g() as f32 * a) as u8;
            px.0[2] = (bg.b() as f32 * (1.0 - a) + fg.b() as f32 * a) as u8;
            px.0[3] = 255;
        }
        out
    };

    egui::Panel::left("font_left").show(ui, |ui| {
        ui.heading("Font");
        match font_data {
            FontData::Mirx(font) => {
                show_mirx_metadata(ui, font);
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
                    if let Some(path) = super::pick_file(&[("Font", &["ttf", "otf", "ttc", "mirx"])])
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
                                        state.font_mode = FontMode::Rendered;
                                    }
                                }
                                icu_lib::midata::MiData::FONT(icu_lib::midata::FontData::MirxBundle(fonts_b)) => {
                                    if let Some(font_b) = fonts_b.first() {
                                        let img_b = icu_lib::endecoder::mirui::font_render::render_font_atlas(font_b);
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
                                            state.font_mode = FontMode::Rendered;
                                        }
                                    }
                                }
                                icu_lib::midata::MiData::FONT(icu_lib::midata::FontData::FreeType(_)) => {}
                                _ => {}
                            }
                        }
                    }
                }
                if ui.button("Add font file...").clicked() {
                    if let Some(path) = super::pick_file(&[("mirx", &["mirx"])])
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
                    if let Some(path) = super::pick_save_file(&[("mirx", &["mirx"])], "bundle.mirx")
                    {
                        let _ = std::fs::write(&path, merged);
                    }
                }
            }
            FontData::MirxBundle(fonts) => {
                if fonts.is_empty() {
                    ui.label("bundle: 0 fonts");
                    return;
                }
                if state.font_bundle_index >= fonts.len() {
                    state.font_bundle_index = 0;
                }
                ui.label(format!("bundle: {} fonts", fonts.len()));
                if fonts.len() > 1 {
                    let mut next_index = state.font_bundle_index;
                    egui::ComboBox::from_label("font")
                        .selected_text(format!("{} / {}", state.font_bundle_index + 1, fonts.len()))
                        .show_ui(ui, |ui| {
                            for (idx, font) in fonts.iter().enumerate() {
                                ui.selectable_value(
                                    &mut next_index,
                                    idx,
                                    format!("{}: {:?}, {} glyphs", idx + 1, font.chunk_header.kind, font.atlas.glyph_count),
                                );
                            }
                        });
                    if next_index != state.font_bundle_index {
                        state.font_bundle_index = next_index;
                        reset_font_caches(state);
                    }
                }
                if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                    show_mirx_metadata(ui, font);
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
                        if let Some(path) = super::pick_file(&[("Font", &["ttf", "otf", "ttc", "mirx"])])
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
                                            state.font_mode = FontMode::Rendered;
                                        }
                                    }
                                    icu_lib::midata::MiData::FONT(icu_lib::midata::FontData::MirxBundle(fonts_b)) => {
                                        if let Some(font_b) = fonts_b.first() {
                                            let img_b = icu_lib::endecoder::mirui::font_render::render_font_atlas(font_b);
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
                                                state.font_mode = FontMode::Rendered;
                                            }
                                        }
                                    }
                                    icu_lib::midata::MiData::FONT(icu_lib::midata::FontData::FreeType(_)) => {}
                                    _ => {}
                                }
                            }
                        }
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
                    ui.label("bit_depth:");
                    let valid_depths: &[u8] = if state.font_bake_format == "gray" {
                        &[1, 2, 4, 8]
                    } else {
                        &[4, 8]
                    };
                    if !valid_depths.contains(&state.font_bake_bit_depth) {
                        state.font_bake_bit_depth = valid_depths[0];
                    }
                    egui::ComboBox::from_id_salt("bake_bit_depth")
                        .selected_text(format!("{}", state.font_bake_bit_depth))
                        .show_ui(ui, |ui| {
                            for &d in valid_depths {
                                ui.selectable_value(
                                    &mut state.font_bake_bit_depth,
                                    d,
                                    format!("{d}"),
                                );
                            }
                        });
                });

                ui.add_space(4.0);
                crate::image_viewer::ui::widgets::mode_tabs(
                    ui,
                    &mut state.font_bake_charset_tab,
                    &[
                        (BakeCharsetTab::Text, "Text"),
                        (BakeCharsetTab::Range, "Range"),
                        (BakeCharsetTab::File, "File"),
                    ],
                );
                ui.add_space(4.0);

                match state.font_bake_charset_tab {
                    BakeCharsetTab::Text => {
                        ui.text_edit_multiline(&mut state.font_bake_charset_text);
                    }
                    BakeCharsetTab::Range => {
                        ui.text_edit_multiline(&mut state.font_bake_charset_ranges);
                        ui.label(
                            egui::RichText::new("Format: U+XXXX-U+YYYY, one range per line or comma-separated")
                                .size(9.0)
                                .color(ui.style().visuals.weak_text_color()),
                        );
                    }
                    BakeCharsetTab::File => {
                        ui.horizontal(|ui| {
                            if ui.button("Choose charset file…").clicked() {
                                if let Some(path) = super::pick_file(&[("Text", &["txt"])]) {
                                    state.font_bake_charset_file =
                                        Some(path.to_string_lossy().into());
                                }
                            }
                            if let Some(p) = &state.font_bake_charset_file {
                                ui.label(p);
                            }
                        });
                    }
                }

                if ui.button("Bake & Save").clicked() {
                    let kind = if state.font_bake_format == "gray" {
                        icu_lib::mirx::FontChunkKind::Grayscale
                    } else {
                        icu_lib::mirx::FontChunkKind::Sdf
                    };
                    let charset = collect_charset(state);
                    if charset.is_empty() {
                        log::warn!("bake charset is empty");
                        return;
                    }
                    let params = icu_lib::endecoder::mirui::font_bake::FontBakeParams {
                        kind,
                        source_size: state.font_bake_size,
                        bit_depth: state.font_bake_bit_depth as u8,
                        spread: (state.font_bake_size / 4).max(1),
                        charset,
                    };
                    let raw = std::fs::read(&image.path).unwrap_or_default();
                    if let Some(font) = icu_lib::endecoder::mirui::font_bake::bake_font(&raw, &params) {
                        let payload = font.encode();
                        let bytes = icu_lib::mirx::encode_chunk_generic(
                            icu_lib::mirx::chunk_type::FONT,
                            icu_lib::mirx::ChunkEntry::FLAG_CRITICAL,
                            &payload,
                        );
                        if let Some(path) = super::pick_save_file(
                            &[("mirx", &["mirx"])],
                            &format!("{}_{}.mirx", f.family, state.font_bake_format),
                        )
                        {
                            let _ = std::fs::write(&path, bytes);
                        }
                    }
                }
            }
        }
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.horizontal(|ui| {
            crate::image_viewer::ui::widgets::mode_tabs(
                ui,
                &mut state.font_mode,
                &[
                    (FontMode::Atlas, "Atlas"),
                    (FontMode::Rendered, "Rendered"),
                    (FontMode::Grid, "Grid"),
                    (FontMode::Vector, "Vector"),
                ],
            );
        });
        ui.separator();

        match state.font_mode {
            FontMode::Rendered => {
                if state.font_rendered_preview.is_none() {
                    if let Some(font) = selected_mirx_font(font_data, state.font_bundle_index) {
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
            FontMode::Grid => {
                let grid_key = format!("{}_{:?}_{}", image.path, fg, state.font_bundle_index);
                let grid_key_clone = grid_key.clone();
                let need_rebuild = match &state.font_grid_cached {
                    Some((k, _, _)) => k != &grid_key,
                    None => true,
                };
                if need_rebuild {
                    let mut handles: Vec<egui::TextureHandle> = Vec::new();
                    match font_data {
                        FontData::Mirx(font) => {
                            let cell = font.atlas.source_size as usize + 4;
                            for m in font.metrics.iter() {
                                let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                                    font, &ch.to_string(), cell as u32, cell as u32, text_color,
                                );
                                let ci = egui::ColorImage::from_rgba_unmultiplied([cell, cell], img.as_raw());
                                handles.push(ctx.load_texture(format!("glyph_grid_{}", handles.len()), ci, egui::TextureOptions::LINEAR));
                            }
                        }
                        FontData::MirxBundle(fonts) => {
                            if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                                let cell = font.atlas.source_size as usize + 4;
                                for m in font.metrics.iter() {
                                    let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                    let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                                        font, &ch.to_string(), cell as u32, cell as u32, text_color,
                                    );
                                    let ci = egui::ColorImage::from_rgba_unmultiplied([cell, cell], img.as_raw());
                                    handles.push(ctx.load_texture(format!("glyph_grid_{}", handles.len()), ci, egui::TextureOptions::LINEAR));
                                }
                            }
                        }
                        FontData::FreeType(f) => {
                            let cell = 48u32;
                            for g in f.glyphs.iter() {
                                let ch = char::from_u32(g.codepoint).unwrap_or('?');
                                if let Some(img) = icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                                    f, ch, cell, cell, text_color,
                                ) {
                                    let ci = egui::ColorImage::from_rgba_unmultiplied([cell as usize, cell as usize], img.as_raw());
                                    handles.push(ctx.load_texture(format!("ft_grid_{}", handles.len()), ci, egui::TextureOptions::LINEAR));
                                } else {
                                    let empty_img = icu_lib::image::RgbaImage::new(cell, cell);
                                    handles.push(ctx.load_texture("ft_grid_empty", egui::ColorImage::from_rgba_unmultiplied([cell as usize, cell as usize], empty_img.as_raw()), egui::TextureOptions::LINEAR));
                                }
                            }
                        }
                    }
                    let count = handles.len();
                    state.font_grid_cached = Some((grid_key, handles, count));
                    state.font_grid_big_cached = None;
                }

                let handles = state.font_grid_cached.as_ref()
                    .map(|(_, h, _)| h.clone())
                    .unwrap_or_default();

                let cell = match font_data {
                    FontData::Mirx(font) => font.atlas.source_size as f32 + 4.0,
                    FontData::MirxBundle(fonts) => fonts
                        .get(state.font_bundle_index)
                        .or_else(|| fonts.first())
                        .map(|font| font.atlas.source_size as f32 + 4.0)
                        .unwrap_or(48.0),
                    FontData::FreeType(_) => 48.0,
                };
                let cols = 16usize;

                match font_data {
                    FontData::Mirx(font) => {
                        egui::Panel::bottom("glyph_detail").show(ui, |ui| {
                            if let Some(idx) = state.selected_glyph {
                                if let Some(m) = font.metrics.get(idx) {
                                    let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                    ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, m.codepoint));
                                    ui.label(format!("advance: {}  bearing: ({}, {})", m.advance, m.bearing_x, m.bearing_y));
                                    let big_key = format!("{}_{}", grid_key_clone, idx);
                                    let need_big = match &state.font_grid_big_cached {
                                        Some((k, _)) => k != &big_key,
                                        None => true,
                                    };
                                    if need_big {
                                        let big = icu_lib::endecoder::mirui::font_render::render_font_text(
                                            font, &ch.to_string(), 128, 128, text_color,
                                        );
                                        let ci = egui::ColorImage::from_rgba_unmultiplied([128, 128], big.as_raw());
                                        let tex = ctx.load_texture("glyph_big", ci, egui::TextureOptions::LINEAR);
                                        state.font_grid_big_cached = Some((big_key, tex));
                                    }
                                    if let Some((_, tex)) = &state.font_grid_big_cached {
                                        ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                                    }
                                }
                            } else {
                                ui.label("Click a glyph to inspect");
                            }
                        });
                    }
                    FontData::MirxBundle(fonts) => {
                        if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                            egui::Panel::bottom("glyph_detail_bundle").show(ui, |ui| {
                                if let Some(idx) = state.selected_glyph {
                                    if let Some(m) = font.metrics.get(idx) {
                                        let ch = char::from_u32(m.codepoint).unwrap_or('?');
                                        ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, m.codepoint));
                                        ui.label(format!("advance: {}  bearing: ({}, {})", m.advance, m.bearing_x, m.bearing_y));
                                        let big_key = format!("{}_{}", grid_key_clone, idx);
                                        let need_big = match &state.font_grid_big_cached {
                                            Some((k, _)) => k != &big_key,
                                            None => true,
                                        };
                                        if need_big {
                                            let big = icu_lib::endecoder::mirui::font_render::render_font_text(
                                                font, &ch.to_string(), 128, 128, text_color,
                                            );
                                            let ci = egui::ColorImage::from_rgba_unmultiplied([128, 128], big.as_raw());
                                            let tex = ctx.load_texture("glyph_big_bundle", ci, egui::TextureOptions::LINEAR);
                                            state.font_grid_big_cached = Some((big_key, tex));
                                        }
                                        if let Some((_, tex)) = &state.font_grid_big_cached {
                                            ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                                        }
                                    }
                                } else {
                                    ui.label("Click a glyph to inspect");
                                }
                            });
                        }
                    }
                    FontData::FreeType(f) => {
                        egui::Panel::bottom("glyph_detail_ft").show(ui, |ui| {
                            if let Some(idx) = state.selected_glyph {
                                if let Some(g) = f.glyphs.get(idx) {
                                    let ch = char::from_u32(g.codepoint).unwrap_or('?');
                                    ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, g.codepoint));
                                    ui.label(format!("advance: {}  bearing: ({}, {})", g.advance, g.bearing_x, g.bearing_y));
                                    ui.label(format!("bbox: {:?}  outline cmds: {}", g.bbox, g.outline.len()));
                                    let big_key = format!("{}_{}", grid_key_clone, idx);
                                    let need_big = match &state.font_grid_big_cached {
                                        Some((k, _)) => k != &big_key,
                                        None => true,
                                    };
                                    if need_big {
                                        if let Some(img) = icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                                            f, ch, 128, 128, text_color,
                                        ) {
                                            let ci = egui::ColorImage::from_rgba_unmultiplied([128, 128], img.as_raw());
                                            let tex = ctx.load_texture("ft_glyph_big", ci, egui::TextureOptions::LINEAR);
                                            state.font_grid_big_cached = Some((big_key, tex));
                                        }
                                    }
                                    if let Some((_, tex)) = &state.font_grid_big_cached {
                                        ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                                    }
                                }
                            } else {
                                ui.label("Click a glyph to inspect");
                            }
                        });
                    }
                }

                egui::ScrollArea::both().show(ui, |ui| {
                    egui::Grid::new("glyph_grid")
                        .num_columns(cols)
                        .spacing([2.0, 2.0])
                        .show(ui, |ui| {
                            for (i, tex) in handles.iter().enumerate() {
                                let resp = ui.add(egui::Button::image(egui::load::SizedTexture::new(tex.id(), [cell; 2])));
                                if resp.clicked() {
                                    state.selected_glyph = Some(i);
                                }
                                if state.selected_glyph == Some(i) {
                                    ui.painter().rect_stroke(resp.rect, egui::CornerRadius::same(0), egui::Stroke::new(2.0, egui::Color32::CYAN), egui::StrokeKind::Outside);
                                }
                                if (i + 1) % cols == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            }
            FontMode::Atlas | FontMode::Vector => {
                let theme_key = format!(
                    "{:?}_{:?}_{}_{}",
                    fg,
                    bg,
                    image.path,
                    state.font_bundle_index,
                );
                let (image_data, w, h) = if let Some((ref cached_key, _, ref cached_data, cw, ch)) =
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
                            FontData::MirxBundle(fonts) => {
                                if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                                    let atlas_img = icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                                    tint_image(&atlas_img)
                                } else {
                                    icu_lib::image::RgbaImage::new(1, 1)
                                }
                            }
                            FontData::FreeType(_) => {
                                let grid_img = icu_lib::endecoder::mirui::font_render::render_freetype_glyphs(
                                    match font_data { FontData::FreeType(f) => f, _ => unreachable!() },
                                    text_color,
                                );
                                grid_img
                            }
                        };
                        let w = rendered.width();
                        let h = rendered.height();
                        let data: Vec<Color32> = rendered
                            .chunks(4)
                            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        state.font_atlas_cached = Some((theme_key.clone(), image.path.clone(), data.clone(), w, h));
                        (data, w, h)
                    }
                } else {
                    let rendered = match font_data {
                        FontData::Mirx(font) => {
                            let atlas_img =
                                icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                            tint_image(&atlas_img)
                        }
                        FontData::MirxBundle(fonts) => {
                            if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                                let atlas_img = icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                                tint_image(&atlas_img)
                            } else {
                                icu_lib::image::RgbaImage::new(1, 1)
                            }
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
                    state.font_atlas_cached = Some((theme_key.clone(), image.path.clone(), data.clone(), w, h));
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
