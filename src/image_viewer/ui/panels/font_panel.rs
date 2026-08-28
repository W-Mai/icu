use crate::image_viewer::model::{
    BakeCharsetTab, CanvasViewCommand, FontMode, GlyphCanvasView, GlyphDiffResult,
    GlyphTextureCache,
};
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::Color32;
use icu_lib::midata::{FontData, MiData};
use icu_lib::mirx;

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

fn reset_font_caches(state: &mut crate::image_viewer::model::ViewerState) {
    state.font_rendered_preview = None;
    state.font_atlas_cached = None;
    state.font_grid_cached = None;
    state.font_grid_big_cached = None;
    state.selected_glyph = None;
}

fn selected_opened_glyph(
    state: &crate::image_viewer::model::ViewerState,
) -> Option<crate::image_viewer::model::OpenedGlyph> {
    use crate::image_viewer::model::SidebarItem;
    match state.selected_item() {
        Some(SidebarItem::Glyph(g)) => Some(g.clone()),
        _ => None,
    }
}

pub fn draw_glyph_canvas(ui: &mut egui::Ui, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(mut glyph) = selected_opened_glyph(state) else {
        return;
    };

    let (delete_pressed, undo_pressed, redo_pressed) = ui.input(|input| {
        (
            input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
            input.modifiers.command && input.key_pressed(egui::Key::Z) && !input.modifiers.shift,
            (input.modifiers.command && input.modifiers.shift && input.key_pressed(egui::Key::Z))
                || (input.modifiers.command && input.key_pressed(egui::Key::Y)),
        )
    });
    if delete_pressed && let Some(node) = glyph.editor.selected_node {
        let before = glyph.outline.clone();
        if crate::image_viewer::model::delete_glyph_node(&mut glyph.outline, node) {
            glyph.editor.record(before);
            glyph.editor.selected_node = None;
        }
    }
    if undo_pressed && let Some(outline) = glyph.editor.undo(&glyph.outline) {
        glyph.outline = outline;
        glyph.editor.selected_node = None;
    }
    if redo_pressed && let Some(outline) = glyph.editor.redo(&glyph.outline) {
        glyph.outline = outline;
        glyph.editor.selected_node = None;
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} · U+{:04X} · {} path cmds",
                glyph.char_repr,
                glyph.codepoint,
                glyph.outline.len()
            ))
            .size(11.0)
            .color(ui.style().visuals.weak_text_color()),
        );
        ui.separator();
        let selected = glyph.editor.selected_node;
        let can_add = selected.is_some_and(|node| {
            crate::image_viewer::model::can_add_glyph_node(&glyph.outline, node)
        });
        let can_delete = selected.is_some_and(|node| {
            crate::image_viewer::model::can_delete_glyph_node(&glyph.outline, node)
        });
        if ui
            .add_enabled(can_add, egui::Button::new("+"))
            .on_hover_text(t!("add_node"))
            .clicked()
            && let Some(node) = selected
        {
            let before = glyph.outline.clone();
            if crate::image_viewer::model::add_glyph_node(&mut glyph.outline, node) {
                glyph.editor.record(before);
                glyph.editor.selected_node = None;
            }
        }
        if ui
            .add_enabled(can_delete, egui::Button::new("−"))
            .on_hover_text(t!("delete_node"))
            .clicked()
            && let Some(node) = selected
        {
            let before = glyph.outline.clone();
            if crate::image_viewer::model::delete_glyph_node(&mut glyph.outline, node) {
                glyph.editor.record(before);
                glyph.editor.selected_node = None;
            }
        }
        if ui
            .add_enabled(glyph.editor.can_undo(), egui::Button::new("↶"))
            .on_hover_text(t!("undo"))
            .clicked()
            && let Some(outline) = glyph.editor.undo(&glyph.outline)
        {
            glyph.outline = outline;
            glyph.editor.selected_node = None;
        }
        if ui
            .add_enabled(glyph.editor.can_redo(), egui::Button::new("↷"))
            .on_hover_text(t!("redo"))
            .clicked()
            && let Some(outline) = glyph.editor.redo(&glyph.outline)
        {
            glyph.outline = outline;
            glyph.editor.selected_node = None;
        }
    });
    ui.separator();

    if let Some((min_x, min_y, max_x, max_y)) = glyph_outline_bounds(&glyph.outline) {
        glyph.bbox = (
            min_x.round() as i16,
            min_y.round() as i16,
            max_x.round() as i16,
            max_y.round() as i16,
        );
    }
    draw_glyph_vector_view(
        ui,
        glyph.codepoint,
        glyph.advance,
        glyph.bearing.0,
        glyph.bearing.1,
        glyph.bbox,
        &mut glyph.outline,
        glyph.outline_approximate,
        &mut state.glyph_canvas_view,
        Some(&mut glyph.editor),
    );
    if let Some(crate::image_viewer::model::SidebarItem::Glyph(selected)) =
        state.selected_item_mut()
    {
        *selected = glyph;
    }
}

fn font_text_color(ctx: &egui::Context) -> icu_lib::mirx::Color {
    let fg = ctx.global_style().visuals.text_color();
    icu_lib::mirx::Color {
        r: fg.r(),
        g: fg.g(),
        b: fg.b(),
        a: fg.a(),
    }
}

pub fn draw_font_info_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    let ctx = ui.ctx().clone();
    let Some(image) = state.current_image().cloned() else {
        return;
    };
    let Some(MiData::FONT(font_data)) = &image.midata else {
        return;
    };
    let fg = ctx.global_style().visuals.text_color();
    let text_color = icu_lib::mirx::Color {
        r: fg.r(),
        g: fg.g(),
        b: fg.b(),
        a: fg.a(),
    };
    let grid_key = format!("{}_{:?}_{}", image.path, fg, state.font_bundle_index);

    match font_data {
        FontData::Mirx(font) => {
            draw_font_preview_section(ui, state, font, font_text_color(&ctx));
            draw_selected_glyph_section(ui, state, font_data, text_color, &grid_key);
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
                                format!(
                                    "{}: {:?}, {} glyphs",
                                    idx + 1,
                                    font.chunk_header.kind,
                                    font.atlas.glyph_count
                                ),
                            );
                        }
                    });
                if next_index != state.font_bundle_index {
                    state.font_bundle_index = next_index;
                    reset_font_caches(state);
                }
            }
            if let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                ui.add_space(4.0);
                draw_font_preview_section(ui, state, font, font_text_color(&ctx));
                draw_selected_glyph_section(ui, state, font_data, text_color, &grid_key);
            }
        }
        FontData::FreeType(font) => {
            draw_freetype_preview_section(ui, state, font, font_text_color(&ctx));
            draw_selected_glyph_section(ui, state, font_data, text_color, &grid_key);
        }
    }
}

pub fn draw_glyph_convert_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    let Some(glyph) = selected_opened_glyph(state) else {
        return;
    };

    crate::image_viewer::ui::widgets::section_card(ui, t!("section_export").as_ref(), |ui| {
        egui::ComboBox::from_id_salt("glyph_output_format")
            .selected_text(state.glyph_convert_format.clone())
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for format in [
                    "PNG", "JPEG", "BMP", "GIF", "TIFF", "WEBP", "ICO", "LVGL", "MIRX", "SVG",
                ] {
                    ui.selectable_value(
                        &mut state.glyph_convert_format,
                        format.to_string(),
                        format,
                    );
                }
            });

        if state.glyph_convert_format == "MIRX" {
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("glyph_mirx_export_kind")
                .selected_text(match state.context.mirx_export_kind.as_str() {
                    "flat" => t!("mirx_kind_img_flat").to_string(),
                    _ => t!("mirx_kind_scene").to_string(),
                })
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    let scene_label = t!("mirx_kind_scene").to_string();
                    let flat_label = t!("mirx_kind_img_flat").to_string();
                    ui.selectable_value(
                        &mut state.context.mirx_export_kind,
                        "scene".to_string(),
                        &scene_label,
                    );
                    ui.selectable_value(
                        &mut state.context.mirx_export_kind,
                        "flat".to_string(),
                        &flat_label,
                    );
                });
        }

        ui.add_space(12.0);

        if !glyph.outline.is_empty() {
            if crate::image_viewer::ui::widgets::primary_action_button(ui, t!("convert")).clicked()
            {
                match state.glyph_convert_format.as_str() {
                    "SVG" => {
                        let svg = glyph_outline_to_svg(&glyph.outline);
                        if let Some(path) = super::pick_save_file(
                            &[("SVG", &["svg"])],
                            &format!("U+{:04X}.svg", glyph.codepoint),
                        ) {
                            let _ = std::fs::write(&path, svg);
                        }
                    }
                    "MIRX" if state.context.mirx_export_kind == "scene" => {
                        let scene = icu_lib::mirx::Scene {
                            ops: vec![icu_lib::mirx::SceneOp::FillPath {
                                path: icu_lib::mirx::Path {
                                    cmds: glyph.outline.clone(),
                                },
                                transform: icu_lib::mirx::Transform::IDENTITY,
                                paint: icu_lib::mirx::Paint::Color(icu_lib::mirx::Color {
                                    r: 255,
                                    g: 255,
                                    b: 255,
                                    a: 255,
                                }),
                                opa: 255,
                                fill_rule: icu_lib::mirx::FillRule::NonZero,
                            }],
                        };
                        let payload = scene.encode().unwrap_or_default();
                        let bytes = icu_lib::mirx::encode_chunk_generic(
                            icu_lib::mirx::chunk_type::VECTOR,
                            icu_lib::mirx::ChunkEntry::FLAG_CRITICAL,
                            &payload,
                        );
                        if let Some(path) = super::pick_save_file(
                            &[("mirx", &["mirx"])],
                            &format!("U+{:04X}.mirx", glyph.codepoint),
                        ) {
                            let _ = std::fs::write(&path, bytes);
                        }
                    }
                    _ => {
                        let img = render_glyph_outline_image(&glyph.outline);
                        let width = img.width();
                        let height = img.height();
                        let image_item = crate::image_viewer::model::ImageItem {
                            path: format!("glyph U+{:04X}", glyph.codepoint),
                            info: icu_lib::endecoder::ImageInfo {
                                width,
                                height,
                                data_size: 0,
                                format: "rgba".to_string(),
                                other_info: serde_json::Value::Null,
                            },
                            width,
                            height,
                            frames: crate::image_viewer::model::FrameSource::single(
                                img.chunks(4)
                                    .map(|pixel| {
                                        Color32::from_rgba_unmultiplied(
                                            pixel[0], pixel[1], pixel[2], pixel[3],
                                        )
                                    })
                                    .collect::<Vec<Color32>>(),
                                width,
                                height,
                            ),
                            midata: Some(MiData::RGBA(img)),
                            expanded: false,
                        };
                        let mut params = state.context.convert_params.clone();
                        params.output_format = match state.glyph_convert_format.as_str() {
                            "PNG" => crate::image_viewer::model::ImageFormat::PNG,
                            "JPEG" => crate::image_viewer::model::ImageFormat::JPEG,
                            "BMP" => crate::image_viewer::model::ImageFormat::BMP,
                            "GIF" => crate::image_viewer::model::ImageFormat::GIF,
                            "TIFF" => crate::image_viewer::model::ImageFormat::TIFF,
                            "WEBP" => crate::image_viewer::model::ImageFormat::WEBP,
                            "ICO" => crate::image_viewer::model::ImageFormat::ICO,
                            "MIRX" => crate::image_viewer::model::ImageFormat::MIRX,
                            _ => crate::image_viewer::model::ImageFormat::LVGL,
                        };
                        crate::image_viewer::utils::save_images(&[image_item], &params);
                    }
                }
            }
        } else {
            ui.label(t!("rendering_not_available"));
        }
    });
}

pub fn draw_font_convert_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    let Some(image) = state.current_image().cloned() else {
        return;
    };
    let Some(MiData::FONT(font_data)) = &image.midata else {
        return;
    };

    match font_data {
        FontData::Mirx(_font) => {
            draw_merge_fonts_section(ui, state);
        }
        FontData::MirxBundle(fonts) => {
            if let Some(_font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) {
                draw_merge_fonts_section(ui, state);
            }
        }
        FontData::FreeType(f) => {
            draw_font_bake_section(ui, state, &image, f);
        }
    }
}

fn draw_font_preview_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
    font: &icu_lib::mirx::Font,
    text_color: icu_lib::mirx::Color,
) {
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_preview").as_ref(), |ui| {
        ui.text_edit_singleline(&mut state.font_preview_text);
        if ui.button(t!("btn_render")).clicked() {
            let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                font,
                &state.font_preview_text,
                400,
                64,
                text_color,
            );
            state.font_rendered_preview = Some(img);
        }
    });
}

fn draw_freetype_preview_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
    font: &icu_lib::midata::FreeTypeFontData,
    text_color: icu_lib::mirx::Color,
) {
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_preview").as_ref(), |ui| {
        ui.text_edit_singleline(&mut state.font_preview_text);
        if ui.button(t!("btn_render")).clicked() {
            let img = icu_lib::endecoder::mirui::font_render::render_freetype_text(
                font,
                &state.font_preview_text,
                400,
                64,
                text_color,
            );
            state.font_rendered_preview = Some(img);
        }
    });
}

fn draw_selected_glyph_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
    font_data: &FontData,
    text_color: icu_lib::mirx::Color,
    grid_key: &str,
) {
    let Some(idx) = state.selected_glyph else {
        return;
    };

    match font_data {
        FontData::Mirx(font) => {
            let Some(m) = font.metrics.get(idx) else {
                return;
            };
            let ch = char::from_u32(m.codepoint).unwrap_or('?');
            crate::image_viewer::ui::widgets::section_card(ui, "Selected Glyph", |ui| {
                ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, m.codepoint));
                ui.label(format!(
                    "advance: {}  bearing: ({}, {})",
                    m.advance, m.bearing_x, m.bearing_y
                ));
                ui.label("bbox: n/a  outline cmds: 0");
                let big_key = format!("{}_{}", grid_key, idx);
                let need_big = match &state.font_grid_big_cached {
                    Some((k, _)) => k != &big_key,
                    None => true,
                };
                if need_big {
                    let big = icu_lib::endecoder::mirui::font_render::render_font_text(
                        font,
                        &ch.to_string(),
                        128,
                        128,
                        text_color,
                    );
                    let ci = egui::ColorImage::from_rgba_unmultiplied([128, 128], big.as_raw());
                    let tex = ui
                        .ctx()
                        .load_texture("glyph_big", ci, egui::TextureOptions::LINEAR);
                    state.font_grid_big_cached = Some((big_key, tex));
                }
                if let Some((_, tex)) = &state.font_grid_big_cached {
                    ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                }
            });
        }
        FontData::MirxBundle(fonts) => {
            let Some(font) = fonts.get(state.font_bundle_index).or_else(|| fonts.first()) else {
                return;
            };
            let Some(m) = font.metrics.get(idx) else {
                return;
            };
            let ch = char::from_u32(m.codepoint).unwrap_or('?');
            crate::image_viewer::ui::widgets::section_card(ui, "Selected Glyph", |ui| {
                ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, m.codepoint));
                ui.label(format!(
                    "advance: {}  bearing: ({}, {})",
                    m.advance, m.bearing_x, m.bearing_y
                ));
                ui.label("bbox: n/a  outline cmds: 0");
                let big_key = format!("{}_{}", grid_key, idx);
                let need_big = match &state.font_grid_big_cached {
                    Some((k, _)) => k != &big_key,
                    None => true,
                };
                if need_big {
                    let big = icu_lib::endecoder::mirui::font_render::render_font_text(
                        font,
                        &ch.to_string(),
                        128,
                        128,
                        text_color,
                    );
                    let ci = egui::ColorImage::from_rgba_unmultiplied([128, 128], big.as_raw());
                    let tex =
                        ui.ctx()
                            .load_texture("glyph_big_bundle", ci, egui::TextureOptions::LINEAR);
                    state.font_grid_big_cached = Some((big_key, tex));
                }
                if let Some((_, tex)) = &state.font_grid_big_cached {
                    ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                }
            });
        }
        FontData::FreeType(f) => {
            let Some(g) = f.glyphs.get(idx) else {
                return;
            };
            let ch = char::from_u32(g.codepoint).unwrap_or('?');
            crate::image_viewer::ui::widgets::section_card(ui, "Selected Glyph", |ui| {
                ui.heading(format!("Glyph #{}: '{}' (U+{:04X})", idx, ch, g.codepoint));
                ui.label(format!(
                    "advance: {}  bearing: ({}, {})",
                    g.advance, g.bearing_x, g.bearing_y
                ));
                ui.label(format!(
                    "bbox: {:?}  outline cmds: {}",
                    g.bbox,
                    g.outline.len()
                ));
                let big_key = format!("{}_{}", grid_key, idx);
                let need_big = match &state.font_grid_big_cached {
                    Some((k, _)) => k != &big_key,
                    None => true,
                };
                if need_big {
                    if let Some(img) =
                        icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                            f, ch, 128, 128, text_color,
                        )
                    {
                        let padded = pad_image_to_cell(&img, 128);
                        let ci =
                            egui::ColorImage::from_rgba_unmultiplied([128, 128], padded.as_raw());
                        let tex =
                            ui.ctx()
                                .load_texture("ft_glyph_big", ci, egui::TextureOptions::LINEAR);
                        state.font_grid_big_cached = Some((big_key, tex));
                    }
                }
                if let Some((_, tex)) = &state.font_grid_big_cached {
                    ui.image(egui::load::SizedTexture::new(tex.id(), [128.0, 128.0]));
                }
            });
        }
    }
}

pub fn build_selected_glyph_diff_result(
    state: &crate::image_viewer::model::ViewerState,
) -> Option<GlyphDiffResult> {
    use crate::image_viewer::model::SidebarItem;
    let font_data_a = state
        .diff_image1_id
        .and_then(|id| state.item(id))
        .and_then(|item| match item {
            SidebarItem::Image(img) if matches!(img.midata, Some(MiData::FONT(_))) => {
                match &img.midata {
                    Some(MiData::FONT(fd)) => Some(fd.clone()),
                    _ => None,
                }
            }
            _ => None,
        })?;
    let font_data_b = state
        .diff_image2_id
        .and_then(|id| state.item(id))
        .and_then(|item| match item {
            SidebarItem::Image(img) if matches!(img.midata, Some(MiData::FONT(_))) => {
                match &img.midata {
                    Some(MiData::FONT(fd)) => Some(fd.clone()),
                    _ => None,
                }
            }
            _ => None,
        })?;
    let text_color = icu_lib::mirx::Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    let ch = state.glyph_diff_char.chars().next().unwrap_or('A');
    let cell = diff_cell_size(&font_data_a, &font_data_b, state.font_bundle_index);
    let raw_a = render_source_glyph(&font_data_a, state.font_bundle_index, ch, cell, text_color)?;
    let raw_b = render_source_glyph(&font_data_b, 0, ch, cell, text_color)?;
    let w = raw_a.width().max(raw_b.width());
    let h = raw_a.height().max(raw_b.height());
    let img_a = paste_to_canvas(&raw_a, w, h);
    let img_b = paste_to_canvas(&raw_b, w, h);
    build_glyph_diff_result(ch as u32, ch, img_a, img_b)
}

fn paste_to_canvas(src: &icu_lib::image::RgbaImage, w: u32, h: u32) -> icu_lib::image::RgbaImage {
    let mut canvas = icu_lib::image::RgbaImage::new(w, h);
    let dx = (w - src.width()) / 2;
    let dy = (h - src.height()) / 2;
    for y in 0..src.height() {
        for x in 0..src.width() {
            let px = *src.get_pixel(x, y);
            canvas.put_pixel(dx + x, dy + y, px);
        }
    }
    canvas
}

fn pad_image_to_cell(src: &icu_lib::image::RgbaImage, cell: u32) -> icu_lib::image::RgbaImage {
    let sw = src.width();
    let sh = src.height();
    if sw == cell && sh == cell {
        return src.clone();
    }
    let mut canvas = icu_lib::image::RgbaImage::new(cell, cell);
    let dw = sw.min(cell);
    let dh = sh.min(cell);
    let dx = (cell - dw) / 2;
    let dy = (cell - dh) / 2;
    for y in 0..dh {
        for x in 0..dw {
            canvas.put_pixel(dx + x, dy + y, *src.get_pixel(x, y));
        }
    }
    canvas
}

fn render_source_glyph(
    font_data: &FontData,
    bundle_index: usize,
    ch: char,
    cell: u32,
    text_color: icu_lib::mirx::Color,
) -> Option<icu_lib::image::RgbaImage> {
    match font_data {
        FontData::Mirx(font) => Some(icu_lib::endecoder::mirui::font_render::render_font_text(
            font,
            &ch.to_string(),
            cell,
            cell,
            text_color,
        )),
        FontData::MirxBundle(_fonts) => selected_mirx_font(font_data, bundle_index).map(|font| {
            icu_lib::endecoder::mirui::font_render::render_font_text(
                font,
                &ch.to_string(),
                cell,
                cell,
                text_color,
            )
        }),
        FontData::FreeType(font) => {
            icu_lib::endecoder::mirui::font_render::render_freetype_glyph_at(
                font, ch, cell, cell, text_color,
            )
            .or_else(|| Some(icu_lib::image::RgbaImage::new(cell, cell)))
        }
    }
}

fn glyph_count(font_data: &FontData, bundle_index: usize) -> usize {
    match font_data {
        FontData::Mirx(font) => font.metrics.len(),
        FontData::MirxBundle(fonts) => fonts
            .get(bundle_index)
            .or_else(|| fonts.first())
            .map(|font| font.metrics.len())
            .unwrap_or(0),
        FontData::FreeType(font) => font.glyphs.len(),
    }
}

fn glyph_codepoint(font_data: &FontData, bundle_index: usize, index: usize) -> Option<u32> {
    match font_data {
        FontData::FreeType(font) => font.glyphs.get(index).map(|g| g.codepoint),
        FontData::Mirx(font) => font.metrics.get(index).map(|m| m.codepoint),
        FontData::MirxBundle(fonts) => fonts
            .get(bundle_index)
            .or_else(|| fonts.first())
            .and_then(|font| font.metrics.get(index))
            .map(|m| m.codepoint),
    }
}

fn render_glyph_grid_texture(
    ctx: &egui::Context,
    font_data: &FontData,
    bundle_index: usize,
    glyph_index: usize,
    cell: u32,
    text_color: icu_lib::mirx::Color,
) -> Option<egui::TextureHandle> {
    let ch = char::from_u32(glyph_codepoint(font_data, bundle_index, glyph_index)?).unwrap_or('?');
    let img = render_source_glyph(font_data, bundle_index, ch, cell, text_color)?;
    let padded = pad_image_to_cell(&img, cell);
    let ci =
        egui::ColorImage::from_rgba_unmultiplied([cell as usize, cell as usize], padded.as_raw());
    Some(ctx.load_texture(
        format!("glyph_grid_{bundle_index}_{glyph_index}"),
        ci,
        egui::TextureOptions::LINEAR,
    ))
}

fn diff_cell_size(left: &FontData, right: &FontData, bundle_index: usize) -> u32 {
    let left_cell = preferred_glyph_cell(left, bundle_index).max(1);
    let right_cell = preferred_glyph_cell(right, 0).max(1);
    left_cell.max(right_cell).saturating_mul(2).max(1)
}

fn preferred_glyph_cell(font_data: &FontData, bundle_index: usize) -> u32 {
    match font_data {
        FontData::Mirx(font) => font.atlas.source_size as u32,
        FontData::MirxBundle(_) => selected_mirx_font(font_data, bundle_index)
            .map(|font| font.atlas.source_size as u32)
            .unwrap_or(0),
        FontData::FreeType(_) => 64,
    }
}

fn build_glyph_diff_result(
    codepoint: u32,
    ch: char,
    img_a: icu_lib::image::RgbaImage,
    img_b: icu_lib::image::RgbaImage,
) -> Option<GlyphDiffResult> {
    let dr = icu_lib::endecoder::utils::diff::diff_image(
        &MiData::RGBA(img_a.clone()),
        &MiData::RGBA(img_b.clone()),
    )
    .unwrap_or_else(|| {
        let (w, h) = (img_a.width(), img_a.height());
        icu_lib::endecoder::utils::diff::ImageDiffResult::new((w, h), Vec::new(), 0.0, 0.0)
    });
    Some(GlyphDiffResult {
        codepoint,
        char_repr: ch.to_string(),
        img_a: img_a.clone(),
        img_b: img_b.clone(),
        diff: dr,
    })
}

fn glyph_outline_to_svg(outline: &[icu_lib::mirx::PathCmd]) -> String {
    let Some((min_x, min_y, max_x, max_y)) = glyph_outline_bounds(outline) else {
        return "<svg viewBox=\"0 0 1 1\"><path d=\"\" fill=\"black\"/></svg>".to_string();
    };
    let mut d = String::new();
    for cmd in outline {
        match cmd {
            icu_lib::mirx::PathCmd::MoveTo(p) => {
                push_svg_cmd(&mut d, 'M', &[p.x.to_f32(), p.y.to_f32()]);
            }
            icu_lib::mirx::PathCmd::LineTo(p) => {
                push_svg_cmd(&mut d, 'L', &[p.x.to_f32(), p.y.to_f32()]);
            }
            icu_lib::mirx::PathCmd::QuadTo { ctrl, end } => {
                push_svg_cmd(
                    &mut d,
                    'Q',
                    &[
                        ctrl.x.to_f32(),
                        ctrl.y.to_f32(),
                        end.x.to_f32(),
                        end.y.to_f32(),
                    ],
                );
            }
            icu_lib::mirx::PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                push_svg_cmd(
                    &mut d,
                    'C',
                    &[
                        ctrl1.x.to_f32(),
                        ctrl1.y.to_f32(),
                        ctrl2.x.to_f32(),
                        ctrl2.y.to_f32(),
                        end.x.to_f32(),
                        end.y.to_f32(),
                    ],
                );
            }
            icu_lib::mirx::PathCmd::Close => {
                if !d.is_empty() {
                    d.push(' ');
                }
                d.push('Z');
            }
        }
    }
    format!(
        "<svg viewBox=\"{} {} {} {}\"><path d=\"{}\" fill=\"black\"/></svg>",
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
        d
    )
}

fn push_svg_cmd(d: &mut String, cmd: char, values: &[f32]) {
    if !d.is_empty() {
        d.push(' ');
    }
    d.push(cmd);
    for value in values {
        d.push(' ');
        d.push_str(&format_number(*value));
    }
}

fn format_number(v: f32) -> String {
    let mut s = format!("{v:.3}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() { "0".to_string() } else { s }
}

fn glyph_outline_bounds(outline: &[icu_lib::mirx::PathCmd]) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut seen = false;
    for cmd in outline {
        match cmd {
            icu_lib::mirx::PathCmd::MoveTo(p) | icu_lib::mirx::PathCmd::LineTo(p) => {
                let x = p.x.to_f32();
                let y = p.y.to_f32();
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                seen = true;
            }
            icu_lib::mirx::PathCmd::QuadTo { ctrl, end } => {
                for p in [ctrl, end] {
                    let x = p.x.to_f32();
                    let y = p.y.to_f32();
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    seen = true;
                }
            }
            icu_lib::mirx::PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                for p in [ctrl1, ctrl2, end] {
                    let x = p.x.to_f32();
                    let y = p.y.to_f32();
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    seen = true;
                }
            }
            icu_lib::mirx::PathCmd::Close => continue,
        }
    }
    seen.then_some((min_x, min_y, max_x, max_y))
}

fn render_glyph_outline_image(outline: &[icu_lib::mirx::PathCmd]) -> icu_lib::image::RgbaImage {
    let Some((min_x, min_y, max_x, max_y)) = glyph_outline_bounds(outline) else {
        return icu_lib::image::RgbaImage::new(0, 0);
    };
    let pad = 4.0f32;
    let width = (max_x - min_x + pad * 2.0).ceil().max(1.0) as u32;
    let height = (max_y - min_y + pad * 2.0).ceil().max(1.0) as u32;
    let mut cmds = Vec::with_capacity(outline.len());
    for cmd in outline {
        match cmd {
            icu_lib::mirx::PathCmd::MoveTo(p) => cmds.push(icu_lib::mirx::PathCmd::MoveTo(
                shift_cmd(*p, min_x, min_y, pad),
            )),
            icu_lib::mirx::PathCmd::LineTo(p) => cmds.push(icu_lib::mirx::PathCmd::LineTo(
                shift_cmd(*p, min_x, min_y, pad),
            )),
            icu_lib::mirx::PathCmd::QuadTo { ctrl, end } => {
                cmds.push(icu_lib::mirx::PathCmd::QuadTo {
                    ctrl: shift_cmd(*ctrl, min_x, min_y, pad),
                    end: shift_cmd(*end, min_x, min_y, pad),
                });
            }
            icu_lib::mirx::PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                cmds.push(icu_lib::mirx::PathCmd::CubicTo {
                    ctrl1: shift_cmd(*ctrl1, min_x, min_y, pad),
                    ctrl2: shift_cmd(*ctrl2, min_x, min_y, pad),
                    end: shift_cmd(*end, min_x, min_y, pad),
                });
            }
            icu_lib::mirx::PathCmd::Close => cmds.push(icu_lib::mirx::PathCmd::Close),
        }
    }
    let scene = mirx::Scene {
        ops: vec![mirx::SceneOp::FillPath {
            path: mirx::Path { cmds },
            transform: mirx::Transform::IDENTITY,
            paint: mirx::Paint::Color(mirx::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }),
            opa: 255,
            fill_rule: mirx::FillRule::NonZero,
        }],
    };
    icu_lib::endecoder::mirui::scene_render::render_scene(&scene, width, height)
}

fn shift_cmd(p: icu_lib::mirx::Point, min_x: f32, min_y: f32, pad: f32) -> icu_lib::mirx::Point {
    icu_lib::mirx::Point::new(
        icu_lib::mirx::Fixed::from_raw(((p.x.to_f32() - min_x + pad) * 256.0) as i32),
        icu_lib::mirx::Fixed::from_raw(((p.y.to_f32() - min_y + pad) * 256.0) as i32),
    )
}

fn draw_merge_fonts_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
) {
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_merge_fonts").as_ref(), |ui| {
        if ui.button(t!("btn_add_font_file")).clicked() {
            if let Some(path) = super::pick_file(&[("mirx", &["mirx"])]) {
                state.merge_font_paths.push(path.to_string_lossy().into());
            }
        }
        for (i, p) in state.merge_font_paths.clone().iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(p);
                if ui.button("×").clicked() {
                    state.merge_font_paths.remove(i);
                }
            });
        }
        if state.merge_font_paths.len() >= 2
            && crate::image_viewer::ui::widgets::primary_action_button(ui, t!("btn_merge_save"))
                .clicked()
        {
            let inputs: Vec<Vec<u8>> = state
                .merge_font_paths
                .iter()
                .filter_map(|p| std::fs::read(p).ok())
                .collect();
            let merged = icu_lib::endecoder::mirui::font_bake::merge_font_chunks(&inputs);
            if let Some(path) = super::pick_save_file(&[("mirx", &["mirx"])], "bundle.mirx") {
                let _ = std::fs::write(&path, merged);
            }
        }
    });
}

fn draw_font_bake_section(
    ui: &mut egui::Ui,
    state: &mut crate::image_viewer::model::ViewerState,
    image: &crate::image_viewer::model::ImageItem,
    f: &icu_lib::midata::FreeTypeFontData,
) {
    crate::image_viewer::ui::widgets::section_card(ui, t!("section_bake_to_mirx").as_ref(), |ui| {
        ui.horizontal(|ui| {
            ui.label(t!("label_size"));
            ui.add(egui::DragValue::new(&mut state.font_bake_size).range(8..=64));
            ui.label(t!("label_format"));
            egui::ComboBox::from_label("")
                .selected_text(&state.font_bake_format)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.font_bake_format, "sdf".into(), "sdf");
                    ui.selectable_value(&mut state.font_bake_format, "gray".into(), "gray");
                });
            ui.label(t!("label_bit_depth"));
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
                        ui.selectable_value(&mut state.font_bake_bit_depth, d, format!("{d}"));
                    }
                });
        });

        ui.add_space(4.0);
        crate::image_viewer::ui::widgets::mode_tabs(
            ui,
            &mut state.font_bake_charset_tab,
            &[
                (BakeCharsetTab::Text, t!("tab_text").as_ref()),
                (BakeCharsetTab::Range, t!("tab_range").as_ref()),
                (BakeCharsetTab::File, t!("tab_file").as_ref()),
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
                    egui::RichText::new(t!("charset_range_hint"))
                        .size(9.0)
                        .color(ui.style().visuals.weak_text_color()),
                );
            }
            BakeCharsetTab::File => {
                ui.horizontal(|ui| {
                    if ui.button(t!("btn_choose_charset_file")).clicked() {
                        if let Some(path) = super::pick_file(&[(t!("tab_text").as_ref(), &["txt"])])
                        {
                            state.font_bake_charset_file = Some(path.to_string_lossy().into());
                        }
                    }
                    if let Some(p) = &state.font_bake_charset_file {
                        ui.label(p);
                    }
                });
            }
        }

        if crate::image_viewer::ui::widgets::primary_action_button(ui, t!("btn_bake_save"))
            .clicked()
        {
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
                ) {
                    let _ = std::fs::write(&path, bytes);
                }
            }
        }
    });
}

fn font_atlas_cache_key(
    ctx: &egui::Context,
    state: &crate::image_viewer::model::ViewerState,
) -> Option<String> {
    let image = state.current_image()?;
    let fg = ctx.global_style().visuals.text_color();
    let bg = ctx.global_style().visuals.panel_fill;
    Some(format!(
        "{:?}_{:?}_{}_{}",
        fg, bg, image.path, state.font_bundle_index
    ))
}

pub fn font_atlas_is_current(
    ctx: &egui::Context,
    state: &crate::image_viewer::model::ViewerState,
) -> bool {
    let Some(key) = font_atlas_cache_key(ctx, state) else {
        return false;
    };
    state
        .font_atlas_cached
        .as_ref()
        .is_some_and(|(cached_key, ..)| cached_key == &key)
}

pub fn font_vector_has_outline(state: &crate::image_viewer::model::ViewerState) -> Option<bool> {
    let index = state.selected_glyph?;
    let MiData::FONT(font_data) = state.current_image()?.midata.as_ref()? else {
        return None;
    };
    match font_data {
        FontData::FreeType(font) => font
            .glyphs
            .get(index)
            .map(|glyph| !glyph.outline.is_empty()),
        FontData::Mirx(font) => font.metrics.get(index).map(|_| false),
        FontData::MirxBundle(fonts) => fonts
            .get(state.font_bundle_index)
            .or_else(|| fonts.first())
            .and_then(|font| font.metrics.get(index))
            .map(|_| false),
    }
}

pub fn draw_font_canvas(ui: &mut egui::Ui, state: &mut crate::image_viewer::model::ViewerState) {
    let ctx = ui.ctx().clone();
    let Some(image) = state.current_image().cloned() else {
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
                    &mut state.font_mode,
                    &[
                        (FontMode::Grid, t!("tab_grid").as_ref()),
                        (FontMode::Rendered, t!("tab_rendered").as_ref()),
                        (FontMode::Atlas, t!("tab_atlas").as_ref()),
                    ],
                );
            });
        });
    ui.separator();

    match state.font_mode {
        FontMode::Rendered => {
            if state.font_rendered_preview.is_none() {
                if let Some(font) = selected_mirx_font(font_data, state.font_bundle_index) {
                    let preview_h = font.atlas.line_height.max(1) as u32 * 2;
                    let img = icu_lib::endecoder::mirui::font_render::render_font_text(
                        font,
                        &state.font_preview_text,
                        400,
                        preview_h,
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
                let texture = ui.ctx().load_texture(
                    "font_rendered",
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                let tex_id = texture.id();

                let view = &mut state.render_canvas_view;
                let avail = ui.available_size_before_wrap();
                let (response, painter) = ui.allocate_painter(avail, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                let fit_scale = (canvas_rect.width() / w as f32)
                    .min(canvas_rect.height() / h as f32)
                    .min(1.0);

                apply_canvas_command(view, fit_scale);

                if response.contains_pointer() {
                    let (zoom_delta, scroll_delta, pointer) = ui
                        .input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.pointer.hover_pos()));
                    if zoom_delta != 1.0 {
                        let old_zoom = view.zoom;
                        let new_zoom = (old_zoom * zoom_delta).clamp(0.1, 64.0);
                        if let Some(pointer) = pointer {
                            let anchor = pointer - canvas_rect.center();
                            view.pan = anchor - (anchor - view.pan) * (new_zoom / old_zoom);
                        }
                        view.zoom = new_zoom;
                    }
                    view.pan += response.drag_delta();
                    view.pan += scroll_delta;
                }

                if response.double_clicked() {
                    view.zoom = 1.0;
                    view.pan = egui::Vec2::ZERO;
                }

                let scale = fit_scale * view.zoom;
                let draw_w = w as f32 * scale;
                let draw_h = h as f32 * scale;
                let center = canvas_rect.center() + view.pan;
                let img_rect = egui::Rect::from_center_size(center, egui::vec2(draw_w, draw_h));

                if ui.is_rect_visible(canvas_rect) {
                    painter.rect(
                        canvas_rect,
                        crate::image_viewer::ui::theme::RADIUS,
                        ui.style().visuals.panel_fill,
                        egui::Stroke::new(1.0, ui.style().visuals.window_stroke.color),
                        egui::StrokeKind::Inside,
                    );
                    painter.image(
                        tex_id,
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            } else {
                ui.label(t!("rendering_not_available"));
            }
        }
        FontMode::Grid => {
            let grid_key = format!("{}_{:?}_{}", image.path, fg, state.font_bundle_index);
            let cache_changed = match &state.font_grid_cached {
                Some(cache) => cache.key != grid_key,
                None => true,
            };
            match &mut state.font_grid_cached {
                Some(cache) if cache.key == grid_key => {}
                Some(cache) => {
                    cache.map.clear();
                    cache.key = grid_key.clone();
                    state.font_grid_big_cached = None;
                }
                None => {
                    state.font_grid_cached = Some(GlyphTextureCache {
                        map: std::collections::HashMap::new(),
                        key: grid_key.clone(),
                    });
                    state.font_grid_big_cached = None;
                }
            }

            let count = glyph_count(font_data, state.font_bundle_index);
            if count == 0 {
                ui.label(t!("rendering_not_available"));
                return;
            }

            let cell = match font_data {
                FontData::Mirx(font) => font.atlas.source_size as f32 + 4.0,
                FontData::MirxBundle(fonts) => fonts
                    .get(state.font_bundle_index)
                    .or_else(|| fonts.first())
                    .map(|font| font.atlas.source_size as f32 + 4.0)
                    .unwrap_or(48.0),
                FontData::FreeType(_) => 48.0,
            };
            let spacing = 2.0;

            let scroll_id = egui::Id::new("glyph_grid_scroll");
            let mut scroll_area = egui::ScrollArea::vertical().id_salt(scroll_id);
            if cache_changed {
                scroll_area = scroll_area.scroll_offset(egui::Vec2::ZERO);
            }
            scroll_area.show(ui, |ui| {
                let avail = ui.available_width();
                let btn_pad = ui.style().spacing.button_padding.x * 2.0;
                let col_w = cell + btn_pad + 4.0;
                let cols = (((avail - spacing) / (col_w + spacing)).floor() as usize).max(1);
                let total_rows = count.div_ceil(cols);
                let row_height = cell + spacing;
                let clip_rect = ui.clip_rect();
                let cursor_y = ui.cursor().top();
                let first_visible_row =
                    (((clip_rect.top() - cursor_y) / row_height).floor() as isize).max(0) as usize;
                let visible_rows =
                    ((clip_rect.height() / row_height).ceil() as usize).saturating_add(2);
                let last_visible_row = first_visible_row
                    .saturating_add(visible_rows)
                    .min(total_rows);
                let prefetch_rows = 500usize.div_ceil(cols.max(1));
                let first_render_row = first_visible_row.saturating_sub(prefetch_rows);
                let last_render_row = last_visible_row
                    .saturating_add(prefetch_rows)
                    .min(total_rows);
                let total_width = cols as f32 * col_w + (cols.saturating_sub(1)) as f32 * spacing;
                let left_pad = ((avail - total_width) * 0.5).max(0.0);
                let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

                if let Some(cache) = state.font_grid_cached.as_mut() {
                    for i in (first_render_row * cols)..(last_render_row * cols).min(count) {
                        if !cache.map.contains_key(&i) {
                            if let Some(tex) = render_glyph_grid_texture(
                                &ctx,
                                font_data,
                                state.font_bundle_index,
                                i,
                                cell as u32,
                                text_color,
                            ) {
                                cache.map.insert(i, tex);
                            } else {
                                let empty = egui::ColorImage::new(
                                    [cell as usize, cell as usize],
                                    vec![
                                        egui::Color32::TRANSPARENT;
                                        (cell as usize) * (cell as usize)
                                    ],
                                );
                                cache.map.insert(
                                    i,
                                    ctx.load_texture(
                                        format!("glyph_grid_empty_{i}"),
                                        empty,
                                        egui::TextureOptions::LINEAR,
                                    ),
                                );
                            }
                        }
                    }
                }

                ui.add_space(first_visible_row as f32 * row_height);
                for row in first_visible_row..last_visible_row {
                    let start = row * cols;
                    let end = (start + cols).min(count);
                    ui.horizontal(|ui| {
                        ui.add_space(left_pad);
                        for i in start..end {
                            let is_sel = state.selected_glyph == Some(i);
                            let is_opened = state.opened_glyphs.iter().any(|og| {
                                glyph_codepoint(font_data, state.font_bundle_index, i)
                                    == Some(og.codepoint)
                            });
                            let Some(cache) = state.font_grid_cached.as_ref() else {
                                continue;
                            };
                            let Some(tex) = cache.map.get(&i) else {
                                continue;
                            };
                            let btn = egui::Button::image(egui::load::SizedTexture::new(
                                tex.id(),
                                [cell; 2],
                            ))
                            .corner_radius(egui::CornerRadius::same(2))
                            .stroke(if is_sel {
                                egui::Stroke::new(2.0, p.accent())
                            } else if is_opened {
                                egui::Stroke::new(1.0, p.peach)
                            } else {
                                egui::Stroke::new(1.0, p.surface0)
                            });
                            let resp = ui.add(btn);
                            if resp.clicked() {
                                state.selected_glyph = Some(i);
                            }
                            if resp.double_clicked() {
                                if let Some(og) =
                                    build_opened_glyph(font_data, i, state.font_bundle_index)
                                {
                                    state.opened_glyphs.push(og.clone());
                                    state.insert_glyph_after_selected(og);
                                    state.font_mode = FontMode::Vector;
                                }
                            }
                            if is_sel {
                                ui.painter().rect_filled(
                                    resp.rect.expand(2.0),
                                    egui::CornerRadius::same(3),
                                    p.accent_dim(),
                                );
                            }
                            if i + 1 < end {
                                ui.add_space(spacing);
                            }
                        }
                    });
                    ui.add_space(spacing);
                }
                ui.add_space(total_rows.saturating_sub(last_visible_row) as f32 * row_height);
            });
        }
        FontMode::Vector => {
            let glyph = match font_data {
                FontData::FreeType(f) => state
                    .selected_glyph
                    .and_then(|idx| f.glyphs.get(idx))
                    .map(|g| {
                        (
                            g.codepoint,
                            g.advance,
                            g.bearing_x,
                            g.bearing_y,
                            g.bbox,
                            g.outline.clone(),
                            false,
                        )
                    }),
                FontData::Mirx(font) => state
                    .selected_glyph
                    .and_then(|idx| font.metrics.get(idx))
                    .map(|m| {
                        (
                            m.codepoint,
                            m.advance,
                            m.bearing_x as i16,
                            m.bearing_y as i16,
                            (0, 0, 0, 0),
                            Vec::new(),
                            true,
                        )
                    }),
                FontData::MirxBundle(fonts) => fonts
                    .get(state.font_bundle_index)
                    .or_else(|| fonts.first())
                    .and_then(|font| state.selected_glyph.and_then(|idx| font.metrics.get(idx)))
                    .map(|m| {
                        (
                            m.codepoint,
                            m.advance,
                            m.bearing_x as i16,
                            m.bearing_y as i16,
                            (0, 0, 0, 0),
                            Vec::new(),
                            true,
                        )
                    }),
            };

            if let Some((cp, advance, bx, by, bbox, mut outline, approx)) = glyph {
                draw_glyph_vector_view(
                    ui,
                    cp,
                    advance,
                    bx,
                    by,
                    bbox,
                    &mut outline,
                    approx,
                    &mut state.glyph_canvas_view,
                    None,
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(t!("select_glyph_in_grid"))
                            .color(ui.style().visuals.weak_text_color()),
                    );
                });
            }
        }
        FontMode::Atlas => {
            let theme_key = font_atlas_cache_key(&ctx, state).unwrap_or_default();
            let need_render = match &state.font_atlas_cached {
                Some((k, _, _, _, _)) => k != &theme_key,
                None => true,
            };
            if need_render {
                let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
                ui.centered_and_justified(|ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new(t!("btn_render_atlas"))
                            .heading()
                            .color(p.accent()),
                    );
                    if ui.add(btn).clicked() {
                        let rendered = match font_data {
                            FontData::Mirx(font) => {
                                let atlas_img =
                                    icu_lib::endecoder::mirui::font_render::render_font_atlas(font);
                                tint_image(&atlas_img)
                            }
                            FontData::MirxBundle(fonts) => {
                                if let Some(font) =
                                    fonts.get(state.font_bundle_index).or_else(|| fonts.first())
                                {
                                    let atlas_img =
                                        icu_lib::endecoder::mirui::font_render::render_font_atlas(
                                            font,
                                        );
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
                        state.font_atlas_cached =
                            Some((theme_key.clone(), image.path.clone(), data.clone(), w, h));
                    }
                });
            } else if let Some((_, _, cached_data, cw, ch)) = &state.font_atlas_cached {
                let color_image = egui::ColorImage {
                    size: [*cw as usize, *ch as usize],
                    source_size: egui::vec2(*cw as f32, *ch as f32),
                    pixels: cached_data.clone(),
                };
                let _ =
                    ui.ctx()
                        .load_texture("font_atlas", color_image, egui::TextureOptions::LINEAR);
                ImagePlotter::new("font_atlas_viewer")
                    .anti_alias(state.context.anti_alias)
                    .show_grid(false)
                    .show_only(true)
                    .background_color(state.context.background_color)
                    .badge(format!("{}×{}", cw, ch))
                    .show(
                        ui,
                        &Some(crate::image_viewer::utils::single_image_item(
                            "atlas".to_string(),
                            icu_lib::endecoder::ImageInfo {
                                width: *cw,
                                height: *ch,
                                data_size: 0,
                                format: "atlas".to_string(),
                                other_info: serde_json::Value::Null,
                            },
                            icu_lib::image::RgbaImage::from_raw(
                                *cw,
                                *ch,
                                crate::image_viewer::utils::straight_rgba_from_color32(cached_data),
                            )
                            .unwrap_or_default(),
                            None,
                        )),
                    );
            }
        }
    }
}

fn build_opened_glyph(
    font_data: &FontData,
    idx: usize,
    bundle_index: usize,
) -> Option<crate::image_viewer::model::OpenedGlyph> {
    use crate::image_viewer::model::OpenedGlyph;
    match font_data {
        FontData::FreeType(f) => {
            let g = f.glyphs.get(idx)?;
            let ch = char::from_u32(g.codepoint).unwrap_or('?');
            Some(OpenedGlyph {
                name: format!("glyph_{} (U+{:04X})", ch, g.codepoint),
                codepoint: g.codepoint,
                char_repr: ch.to_string(),
                advance: g.advance,
                bearing: (g.bearing_x, g.bearing_y),
                bbox: g.bbox,
                outline: g.outline.clone(),
                outline_approximate: false,
                source_font: f.family.clone(),
                source_is_sdf: false,
                editor: crate::image_viewer::model::GlyphEditorState::default(),
            })
        }
        FontData::Mirx(font) => {
            let m = font.metrics.get(idx)?;
            let ch = char::from_u32(m.codepoint).unwrap_or('?');
            let outline =
                icu_lib::endecoder::mirui::font_contour::approximate_glyph_contour(font, idx)
                    .unwrap_or_default();
            let bbox = glyph_outline_bounds(&outline)
                .map(|(min_x, min_y, max_x, max_y)| {
                    (
                        min_x.round() as i16,
                        min_y.round() as i16,
                        max_x.round() as i16,
                        max_y.round() as i16,
                    )
                })
                .unwrap_or((0, 0, 0, 0));
            Some(OpenedGlyph {
                name: format!("glyph_{} (U+{:04X})", ch, m.codepoint),
                codepoint: m.codepoint,
                char_repr: ch.to_string(),
                advance: m.advance,
                bearing: (m.bearing_x as i16, m.bearing_y as i16),
                bbox,
                outline,
                outline_approximate: true,
                source_font: format!("{:?}", font.chunk_header.kind),
                source_is_sdf: matches!(font.chunk_header.kind, mirx::FontChunkKind::Sdf),
                editor: crate::image_viewer::model::GlyphEditorState::default(),
            })
        }
        FontData::MirxBundle(fonts) => {
            let font = fonts.get(bundle_index).or_else(|| fonts.first())?;
            let m = font.metrics.get(idx)?;
            let ch = char::from_u32(m.codepoint).unwrap_or('?');
            let outline =
                icu_lib::endecoder::mirui::font_contour::approximate_glyph_contour(font, idx)
                    .unwrap_or_default();
            let bbox = glyph_outline_bounds(&outline)
                .map(|(min_x, min_y, max_x, max_y)| {
                    (
                        min_x.round() as i16,
                        min_y.round() as i16,
                        max_x.round() as i16,
                        max_y.round() as i16,
                    )
                })
                .unwrap_or((0, 0, 0, 0));
            Some(OpenedGlyph {
                name: format!("glyph_{} (U+{:04X})", ch, m.codepoint),
                codepoint: m.codepoint,
                char_repr: ch.to_string(),
                advance: m.advance,
                bearing: (m.bearing_x as i16, m.bearing_y as i16),
                bbox,
                outline,
                outline_approximate: true,
                source_font: format!("{:?}", font.chunk_header.kind),
                source_is_sdf: matches!(font.chunk_header.kind, mirx::FontChunkKind::Sdf),
                editor: crate::image_viewer::model::GlyphEditorState::default(),
            })
        }
    }
}

fn apply_canvas_command(view: &mut GlyphCanvasView, fit_scale: f32) {
    let Some(command) = view.pending.take() else {
        return;
    };
    match command {
        CanvasViewCommand::Fit => {
            view.zoom = 1.0;
            view.pan = egui::Vec2::ZERO;
        }
        CanvasViewCommand::ActualSize if fit_scale.is_finite() && fit_scale > 0.0 => {
            view.zoom = 1.0 / fit_scale;
            view.pan = egui::Vec2::ZERO;
        }
        CanvasViewCommand::ActualSize => {}
    }
}

fn glyph_view_bounds(
    bbox: (i16, i16, i16, i16),
    bearing_x: i16,
    advance: u16,
) -> (i32, i32, i32, i32) {
    const PADDING: i32 = 4;
    let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = bbox;
    let guide_left = i32::from(bearing_x).min(0);
    let guide_right = i32::from(bearing_x) + i32::from(advance);
    (
        i32::from(bbox_min_x).min(guide_left).min(guide_right) - PADDING,
        i32::from(bbox_min_y).min(0) - PADDING,
        i32::from(bbox_max_x).max(guide_left).max(guide_right) + PADDING,
        i32::from(bbox_max_y).max(0) + PADDING,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph_vector_view(
    ui: &mut egui::Ui,
    codepoint: u32,
    advance: u16,
    bearing_x: i16,
    bearing_y: i16,
    bbox: (i16, i16, i16, i16),
    outline: &mut [icu_lib::mirx::PathCmd],
    approximate: bool,
    view: &mut GlyphCanvasView,
    mut editor: Option<&mut crate::image_viewer::model::GlyphEditorState>,
) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    let ch = char::from_u32(codepoint).unwrap_or('?');

    const DETAILS_HEIGHT: f32 = 190.0;
    let available = ui.available_size_before_wrap();
    let canvas_size = egui::vec2(available.x, (available.y - DETAILS_HEIGHT).max(1.0));
    let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
    let canvas_rect = response.rect;

    let (min_x, min_y, max_x, max_y) = glyph_view_bounds(bbox, bearing_x, advance);
    let gw = (max_x - min_x).max(1) as f32;
    let gh = (max_y - min_y).max(1) as f32;

    let fit_rect = canvas_rect.shrink(16.0);
    let fit_scale = (fit_rect.width() / gw).min(fit_rect.height() / gh);
    apply_canvas_command(view, fit_scale);

    if response.contains_pointer() {
        let (zoom_delta, scroll_delta, pointer) =
            ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.pointer.hover_pos()));
        if zoom_delta != 1.0 {
            let old_zoom = view.zoom;
            let new_zoom = (old_zoom * zoom_delta).clamp(0.1, 64.0);
            if let Some(pointer) = pointer {
                let anchor = pointer - canvas_rect.center();
                view.pan = anchor - (anchor - view.pan) * (new_zoom / old_zoom);
            }
            view.zoom = new_zoom;
        }
        let dragging_node = editor
            .as_ref()
            .is_some_and(|editor| editor.selected_node.is_some())
            && response.dragged();
        if !dragging_node {
            view.pan += response.drag_delta();
        }
        view.pan += scroll_delta;
    }

    if response.double_clicked() {
        view.zoom = 1.0;
        view.pan = egui::Vec2::ZERO;
    }

    let scale = fit_scale * view.zoom;
    let ox = canvas_rect.center().x + view.pan.x - (min_x as f32 + gw / 2.0) * scale;
    let oy = canvas_rect.center().y + view.pan.y + (min_y as f32 + gh / 2.0) * scale;

    let to_screen =
        |x: i32, y: i32| -> egui::Pos2 { egui::pos2(ox + x as f32 * scale, oy - y as f32 * scale) };

    if let Some(editor) = editor.as_deref_mut() {
        let nodes = crate::image_viewer::model::glyph_nodes(outline);
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            editor.selected_node = nodes
                .iter()
                .filter_map(|(node, point)| {
                    let screen =
                        egui::pos2(ox + point.x.to_f32() * scale, oy - point.y.to_f32() * scale);
                    let distance = screen.distance(pointer);
                    (distance <= 8.0).then_some((*node, distance))
                })
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(node, _)| node);
        }
        if response.drag_started() && editor.selected_node.is_some() {
            editor.drag_before = Some(outline.to_vec());
        }
        if response.dragged()
            && let Some(node) = editor.selected_node
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let point = icu_lib::mirx::Point::new(
                icu_lib::mirx::Fixed::from_raw(((pointer.x - ox) / scale * 256.0).round() as i32),
                icu_lib::mirx::Fixed::from_raw(((oy - pointer.y) / scale * 256.0).round() as i32),
            );
            crate::image_viewer::model::move_glyph_node(outline, node, point);
        }
        if response.drag_stopped()
            && let Some(before) = editor.drag_before.take()
            && before != outline
        {
            editor.record(before);
        }
    }

    if ui.is_rect_visible(canvas_rect) {
        painter.rect(
            canvas_rect,
            crate::image_viewer::ui::theme::RADIUS,
            p.surface0,
            egui::Stroke::new(1.0, p.surface1),
            egui::StrokeKind::Inside,
        );

        let baseline_y = 0i32;
        let left_x = i32::from(bearing_x);
        let right_x = left_x + i32::from(advance);
        let guide_stroke = egui::Stroke::new(1.0, p.overlay1);
        for x in [left_x, right_x] {
            let p1 = to_screen(x, min_y);
            let p2 = to_screen(x, max_y);
            paint_dashed_line(&painter, p1, p2, guide_stroke, 5.0);
        }
        let p1 = to_screen(min_x, baseline_y);
        let p2 = to_screen(max_x, baseline_y);
        paint_dashed_line(&painter, p1, p2, guide_stroke, 5.0);

        let label_font = egui::FontId::monospace(11.0);
        let label_color = p.text;
        let label_pos = |pos: egui::Pos2| {
            egui::pos2(
                pos.x
                    .clamp(canvas_rect.left() + 4.0, canvas_rect.right() - 4.0),
                pos.y
                    .clamp(canvas_rect.top() + 4.0, canvas_rect.bottom() - 4.0),
            )
        };
        let draw_label = |pos: egui::Pos2, align: egui::Align2, text: String| {
            let pos = label_pos(pos);
            painter.text(
                pos + egui::vec2(1.0, 1.0),
                align,
                &text,
                label_font.clone(),
                p.mantle,
            );
            painter.text(pos, align, text, label_font.clone(), label_color);
        };
        draw_label(
            to_screen(left_x, max_y) + egui::vec2(4.0, -5.0),
            egui::Align2::LEFT_BOTTOM,
            t!("label_bearing_x").to_string(),
        );
        draw_label(
            to_screen(right_x, max_y) + egui::vec2(4.0, -5.0),
            egui::Align2::LEFT_BOTTOM,
            t!("label_advance").to_string(),
        );
        draw_label(
            to_screen(min_x, baseline_y) + egui::vec2(4.0, 5.0),
            egui::Align2::LEFT_TOP,
            t!("label_baseline").to_string(),
        );

        if outline.is_empty() {
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                if approximate {
                    t!("approximate_contour_warning")
                } else {
                    t!("no_outline_data")
                },
                egui::FontId::proportional(11.0),
                p.peach,
            );
        } else {
            let mut current = egui::Pos2::ZERO;
            let path_stroke = egui::Stroke::new(1.5, p.accent());
            for cmd in outline.iter() {
                match cmd {
                    icu_lib::mirx::PathCmd::MoveTo(pt) => {
                        current = to_screen(pt.x.to_int(), pt.y.to_int());
                    }
                    icu_lib::mirx::PathCmd::LineTo(pt) => {
                        let end = to_screen(pt.x.to_int(), pt.y.to_int());
                        painter.line_segment([current, end], path_stroke);
                        current = end;
                    }
                    icu_lib::mirx::PathCmd::QuadTo { ctrl, end } => {
                        let ctrl_p = to_screen(ctrl.x.to_int(), ctrl.y.to_int());
                        let end_p = to_screen(end.x.to_int(), end.y.to_int());
                        let pts = [current, ctrl_p, end_p];
                        painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
                            pts,
                            false,
                            egui::Color32::TRANSPARENT,
                            path_stroke,
                        ));
                        let handle_stroke = egui::Stroke::new(1.2, p.peach);
                        painter.line_segment([current, ctrl_p], handle_stroke);
                        painter.line_segment([end_p, ctrl_p], handle_stroke);
                        painter.circle_filled(ctrl_p, 3.0, p.peach);
                        current = end_p;
                    }
                    icu_lib::mirx::PathCmd::CubicTo { ctrl1, ctrl2, end } => {
                        let c1 = to_screen(ctrl1.x.to_int(), ctrl1.y.to_int());
                        let c2 = to_screen(ctrl2.x.to_int(), ctrl2.y.to_int());
                        let e = to_screen(end.x.to_int(), end.y.to_int());
                        let pts = [current, c1, c2, e];
                        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                            pts,
                            false,
                            egui::Color32::TRANSPARENT,
                            path_stroke,
                        ));
                        let handle_stroke = egui::Stroke::new(1.2, p.peach);
                        painter.line_segment([current, c1], handle_stroke);
                        painter.line_segment([e, c2], handle_stroke);
                        painter.circle_filled(c1, 3.0, p.peach);
                        painter.circle_filled(c2, 3.0, p.peach);
                        current = e;
                    }
                    icu_lib::mirx::PathCmd::Close => {}
                }
            }

            let selected_node = editor.as_ref().and_then(|editor| editor.selected_node);
            for (node, point) in crate::image_viewer::model::glyph_nodes(outline) {
                let pos = egui::pos2(ox + point.x.to_f32() * scale, oy - point.y.to_f32() * scale);
                let is_handle = node.role != crate::image_viewer::model::GlyphNodeRole::Endpoint;
                let selected = selected_node == Some(node);
                let radius = if selected {
                    5.0
                } else if is_handle {
                    3.0
                } else {
                    3.5
                };
                let color = if selected {
                    p.peach
                } else if is_handle {
                    p.peach
                } else {
                    p.accent()
                };
                painter.circle_filled(pos, radius, color);
                painter.circle_stroke(pos, radius, egui::Stroke::new(1.0, p.base));
            }
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Glyph '{}' (U+{:04X})", ch, codepoint))
                .size(12.0)
                .color(p.text),
        );
    });
    glyph_metrics_card(
        ui,
        codepoint,
        advance,
        bearing_x,
        bearing_y,
        bbox,
        outline.len(),
        approximate,
    );
}

fn glyph_metrics_card(
    ui: &mut egui::Ui,
    codepoint: u32,
    advance: u16,
    bearing_x: i16,
    bearing_y: i16,
    bbox: (i16, i16, i16, i16),
    outline_len: usize,
    approximate: bool,
) {
    let (min_x, min_y, max_x, max_y) = bbox;
    crate::image_viewer::ui::widgets::section_card(
        ui,
        t!("section_glyph_metrics").as_ref(),
        |ui| {
            let source_atlas = t!("source_atlas_approximate").to_string();
            let source_freetype = t!("source_freetype_true_vector").to_string();
            egui::Grid::new("glyph_metrics_grid")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
                    let row = |ui: &mut egui::Ui, label: &str, value: &str| {
                        ui.label(egui::RichText::new(label).size(11.0).color(p.overlay0));
                        ui.label(
                            egui::RichText::new(value)
                                .size(11.0)
                                .color(p.text)
                                .family(egui::FontFamily::Monospace),
                        );
                        ui.end_row();
                    };
                    row(
                        ui,
                        t!("codepoint").as_ref(),
                        &format!(
                            "U+{:04X} ({})",
                            codepoint,
                            char::from_u32(codepoint).unwrap_or('?')
                        ),
                    );
                    row(ui, t!("advance").as_ref(), &format!("{} px", advance));
                    row(ui, t!("bearing_x").as_ref(), &format!("{} px", bearing_x));
                    row(ui, t!("bearing_y").as_ref(), &format!("{} px", bearing_y));
                    row(
                        ui,
                        t!("bbox").as_ref(),
                        &format!("({}, {}, {}, {})", min_x, min_y, max_x, max_y),
                    );
                    row(ui, t!("outline_cmds").as_ref(), &format!("{}", outline_len));
                    row(
                        ui,
                        t!("source").as_ref(),
                        if approximate {
                            source_atlas.as_str()
                        } else {
                            source_freetype.as_str()
                        },
                    );
                });
        },
    );
}

fn paint_dashed_line(
    painter: &egui::Painter,
    p1: egui::Pos2,
    p2: egui::Pos2,
    stroke: egui::Stroke,
    dash: f32,
) {
    let delta = p2 - p1;
    let len = delta.length();
    if len < 1.0 {
        return;
    }
    let dir = delta / len;
    let mut t = 0.0;
    let mut on = true;
    while t < len {
        let next = (t + dash).min(len);
        if on {
            painter.line_segment([p1 + dir * t, p1 + dir * next], stroke);
        }
        t = next;
        on = !on;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_size_command_sets_unit_scale_and_recenters() {
        let mut view = GlyphCanvasView {
            zoom: 3.0,
            pan: egui::vec2(12.0, -8.0),
            pending: Some(CanvasViewCommand::ActualSize),
        };

        apply_canvas_command(&mut view, 0.25);

        assert_eq!(view.zoom, 4.0);
        assert_eq!(view.pan, egui::Vec2::ZERO);
        assert_eq!(view.pending, None);
    }

    #[test]
    fn zero_bbox_uses_guides_for_finite_bounds() {
        let (min_x, min_y, max_x, max_y) = glyph_view_bounds((0, 0, 0, 0), -2, 8);

        assert_eq!((min_x, min_y, max_x, max_y), (-6, -4, 10, 4));
        assert!(min_x < max_x);
        assert!(min_y < max_y);
    }
}
