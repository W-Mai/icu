use crate::image_viewer::model::{FrameSource, ImageItem, ViewerState};
use crate::image_viewer::plotter::ImagePlotter;
use crate::image_viewer::ui::panels;
use eframe::egui;
use icu_lib::midata::MiData;
use serde::Serialize;

pub fn draw_central_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    let content_type = get_content_type(state);

    egui::CentralPanel::default()
        .frame(crate::image_viewer::ui::theme::central_panel_frame(ui.ctx()))
        .show(ui, |ui| match (state.context.diff_active, get_diff_mode(state), content_type) {
            (true, DiffMode::Glyph | DiffMode::Image, _) | (_, _, ContentType::Rgba) => {
                draw_rgba_canvas(ui, state)
            }
            (_, _, ContentType::Font) => panels::font_panel::draw_font_canvas(ui, state),
            (_, _, ContentType::Path) => panels::path_panel::draw_path_canvas(ui, state),
            (_, _, ContentType::Indexed) => panels::indexed_panel::draw_indexed_canvas(ui, state),
            (_, _, ContentType::Glyph) => panels::font_panel::draw_glyph_canvas(ui, state),
        });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffMode {
    None,
    Image,
    Glyph,
    Incompatible,
}

#[derive(Clone, Copy, Debug)]
pub enum ContentType {
    Rgba,
    Font,
    Path,
    Indexed,
    Glyph,
}

pub fn get_content_type(state: &ViewerState) -> ContentType {
    if let Some(idx) = state.selected_index {
        if let Some(crate::image_viewer::model::SidebarItem::Glyph(_)) = state.items.get(idx) {
            return ContentType::Glyph;
        }
    }

    if let Some(image) = &state.current_image {
        if let Some(midata) = &image.midata {
            match midata {
                MiData::FONT(_) => return ContentType::Font,
                MiData::PATH(_) => return ContentType::Path,
                MiData::INDEXED(_) => return ContentType::Indexed,
                _ => {}
            }
        }
    }

    ContentType::Rgba
}

pub fn get_diff_mode(state: &ViewerState) -> DiffMode {
    let (Some(i1), Some(i2)) = (state.diff_image1_index, state.diff_image2_index) else {
        return DiffMode::None;
    };
    let (Some(item1), Some(item2)) = (state.items.get(i1), state.items.get(i2)) else {
        return DiffMode::None;
    };
    let (Some(a), Some(b)) = (item_image(item1), item_image(item2)) else {
        return DiffMode::None;
    };
    match (is_font(a), is_font(b), is_rgba_image(a), is_rgba_image(b)) {
        (true, true, _, _) => DiffMode::Glyph,
        (_, _, true, true) => DiffMode::Image,
        _ => DiffMode::Incompatible,
    }
}

fn item_image(item: &crate::image_viewer::model::SidebarItem) -> Option<&ImageItem> {
    match item {
        crate::image_viewer::model::SidebarItem::Image(image) => Some(image),
        crate::image_viewer::model::SidebarItem::Glyph(_) => None,
    }
}

fn is_font(image: &ImageItem) -> bool {
    matches!(image.midata, Some(MiData::FONT(_)))
}

fn is_rgba_image(image: &ImageItem) -> bool {
    matches!(image.midata, Some(MiData::RGBA(_)) | None)
}

fn draw_rgba_canvas(ui: &mut egui::Ui, state: &mut ViewerState) {
    if state.context.diff_active && get_diff_mode(state) == DiffMode::Glyph {
        draw_glyph_diff_canvas(ui, state);
        return;
    }

    let image_plotter = ImagePlotter::new("viewer")
        .anti_alias(state.context.anti_alias)
        .show_grid(state.context.show_grid)
        .background_color(state.context.background_color)
        .highlight(if state.hovered_diff_pixel.is_none() {
            state.selected_diff_pixel
        } else {
            state.hovered_diff_pixel
        })
        .on_hover(&mut state.hovered_diff_pixel_from_plot);

    if state.context.only_show_diff {
        if let Some((diff_img, _)) = &state.diff_result {
            image_plotter
                .badge(format!("{}×{} · diff", diff_img.width, diff_img.height))
                .show(ui, &Some(diff_img.clone()));
        }
    } else if let Some((diff_img, _)) = &state.diff_result
        && state.context.diff_active
    {
        image_plotter
            .badge(format!("{}×{} · diff", diff_img.width, diff_img.height))
            .show(ui, &Some(diff_img.clone()));
    } else if let Some(image) = state.current_image.as_mut() {
        let advanced = image.advance_frame();
        if advanced || image.autoplay() {
            ui.ctx().request_repaint();
        }
        let image_for_plot = image.clone();
        image_plotter
            .badge(format!(
                "{}×{} · {}",
                image_for_plot.width, image_for_plot.height, image_for_plot.info.format
            ))
            .show(ui, &Some(image_for_plot));
        draw_frame_controls(ui, image);
        if let Some(idx) = state.selected_index {
            if let Some(crate::image_viewer::model::SidebarItem::Image(src)) = state.items.get_mut(idx) {
                src.frames = image.frames.clone();
            }
        }
    } else {
        let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
        let avail = ui.available_size();
        let (rect, click_response) = ui.allocate_exact_size(avail, egui::Sense::click());
        let hovered = click_response.hovered();
        if ui.is_rect_visible(rect) {
            let dashed_rect = rect.shrink(24.0);
            let stroke_color = if hovered { p.accent() } else { p.surface1 };
            paint_dashed_rect(
                ui.painter(),
                dashed_rect,
                egui::CornerRadius::same(12),
                stroke_color,
                8.0,
                6.0,
            );
        }
        let text_color = if hovered { p.accent() } else { p.overlay0 };
        let galley = ui.painter().layout(
            t!("drag_here").to_string(),
            egui::FontId::proportional(72.0),
            text_color,
            f32::INFINITY,
        );
        let text_pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley, text_color);
        if click_response.clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let files: Vec<eframe::egui::DroppedFile> = rfd::FileDialog::new()
                    .pick_files()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| eframe::egui::DroppedFile {
                        path: Some(p),
                        ..Default::default()
                    })
                    .collect();
                if !files.is_empty() {
                    let new_items: Vec<crate::image_viewer::model::SidebarItem> =
                        crate::image_viewer::utils::process_images(&files)
                            .into_iter()
                            .map(crate::image_viewer::model::SidebarItem::Image)
                            .collect();
                    let start_idx = state.items.len();
                    state.items.extend(new_items);
                    if let Some(crate::image_viewer::model::SidebarItem::Image(img)) =
                        state.items.get(start_idx).cloned()
                    {
                        state.current_image = Some(img);
                        state.selected_index = Some(start_idx);
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                crate::image_viewer::utils::pick_files_web(
                    state.pending_dropped.clone(),
                    ui.ctx().clone(),
                );
            }
        }
    }
}

fn draw_glyph_diff_canvas(ui: &mut egui::Ui, state: &mut ViewerState) {
    let Some(result) = panels::font_panel::build_selected_glyph_diff_result(state) else {
        ui.centered_and_justified(|ui| {
            ui.label(t!("hint_select_two_fonts_for_diff"));
        });
        return;
    };
    let w = result.img_a.width() + result.diff_overlay.width() + result.img_b.width();
    let h = result
        .img_a
        .height()
        .max(result.diff_overlay.height())
        .max(result.img_b.height());
    let mut canvas = icu_lib::image::RgbaImage::new(w, h);
    paste_rgba(&mut canvas, &result.img_a, 0, 0);
    paste_rgba(&mut canvas, &result.diff_overlay, result.img_a.width(), 0);
    paste_rgba(
        &mut canvas,
        &result.img_b,
        result.img_a.width() + result.diff_overlay.width(),
        0,
    );

    let image = crate::image_viewer::utils::single_image_item(
        format!("glyph diff U+{:04X}", result.codepoint),
        icu_lib::endecoder::ImageInfo {
            width: w,
            height: h,
            data_size: canvas.len() as u32,
            format: format!("glyph diff · {}", result.char_repr),
            other_info: serde_json::Value::Null,
        },
        canvas,
        None,
    );
    ImagePlotter::new("glyph_diff_viewer")
        .anti_alias(state.context.anti_alias)
        .show_grid(state.context.show_grid)
        .background_color(state.context.background_color)
        .badge(format!(
            "A | diff | B · U+{:04X} · {} diff pixels",
            result.codepoint,
            result.diff.diff_filter(state.context.diff_tolerance).count()
        ))
        .show(ui, &Some(image));
}

fn paste_rgba(dst: &mut icu_lib::image::RgbaImage, src: &icu_lib::image::RgbaImage, x0: u32, y0: u32) {
    for y in 0..src.height() {
        for x in 0..src.width() {
            dst.put_pixel(x0 + x, y0 + y, *src.get_pixel(x, y));
        }
    }
}

fn draw_frame_controls(ui: &mut egui::Ui, image: &mut crate::image_viewer::model::ImageItem) {
    let FrameSource::Animated { frames, current, autoplay, last_advance } = &mut image.frames else {
        return;
    };
    if frames.len() <= 1 {
        return;
    }

    ui.horizontal(|ui| {
        let label = if *autoplay { "Pause" } else { "Play" };
        if ui.button(label).clicked() {
            *autoplay = !*autoplay;
            *last_advance = None;
            ui.ctx().request_repaint();
        }

        let mut frame_index = *current;
        let frame_label = format!("frame {} / {}", *current + 1, frames.len());
        let slider = egui::Slider::new(&mut frame_index, 0..=frames.len() - 1)
            .text(frame_label);
        if ui.add(slider).changed() {
            *current = frame_index;
            *last_advance = None;
        }
    });
}/// Renders a serializable value as a YAML tree.
pub fn ui_tree_view(ui: &mut egui::Ui, value: &impl Serialize) {
    if let Ok(yaml_value) = serde_yaml::to_value(value) {
        ui_yaml_tree(ui, &yaml_value);
    } else {
        ui.label(t!("error_displaying_data"));
    }
}

/// Recursive helper to render YAML data.
pub fn ui_yaml_tree(ui: &mut egui::Ui, value: &serde_yaml::Value) {
    match value {
        serde_yaml::Value::Null => {
            ui.label("~");
        }
        serde_yaml::Value::Bool(b) => {
            ui.label(b.to_string());
        }
        serde_yaml::Value::Number(n) => {
            ui.label(n.to_string());
        }
        serde_yaml::Value::String(s) => {
            ui.label(format!("{s:?}"));
        }
        serde_yaml::Value::Sequence(seq) => {
            ui.collapsing(format!("Sequence [{}]", seq.len()), |ui| {
                for (i, v) in seq.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("- [{i}]"));
                        ui_yaml_tree(ui, v);
                    });
                }
            });
        }
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => format!("{k:?}"),
                };

                if v.is_mapping() || v.is_sequence() {
                    ui.collapsing(key_str, |ui| {
                        ui_yaml_tree(ui, v);
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(format!("{key_str}: "));
                        ui_yaml_tree(ui, v);
                    });
                }
            }
        }
        serde_yaml::Value::Tagged(tagged) => {
            ui.horizontal(|ui| {
                ui.label(format!("!{}", tagged.tag));
                ui_yaml_tree(ui, &tagged.value);
            });
        }
    }
}

fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner: egui::CornerRadius,
    color: eframe::egui::Color32,
    dash: f32,
    gap: f32,
) {
    let stroke = egui::Stroke::new(2.0, color);
    let step = dash + gap;
    let tl = rect.left_top();
    let tr = rect.right_top();
    let br = rect.right_bottom();
    let mut t = 0.0;
    while t < rect.width() {
        let x1 = rect.left() + t;
        let x2 = (x1 + dash).min(rect.right());
        painter.line_segment([egui::pos2(x1, tl.y), egui::pos2(x2, tl.y)], stroke);
        painter.line_segment([egui::pos2(x1, br.y), egui::pos2(x2, br.y)], stroke);
        t += step;
    }
    let mut t = 0.0;
    while t < rect.height() {
        let y1 = rect.top() + t;
        let y2 = (y1 + dash).min(rect.bottom());
        painter.line_segment([egui::pos2(tl.x, y1), egui::pos2(tl.x, y2)], stroke);
        painter.line_segment([egui::pos2(tr.x, y1), egui::pos2(tr.x, y2)], stroke);
        t += step;
    }
    let _ = corner;
}
