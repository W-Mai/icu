use crate::image_viewer::model::{FrameSource, GlyphCanvasView, ImageItem, ViewerState};
use crate::image_viewer::plotter::ImagePlotter;
use crate::image_viewer::ui::panels;
use eframe::egui;
use icu_lib::midata::MiData;
use serde::Serialize;

pub fn draw_central_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    let content_type = get_content_type(state);

    egui::CentralPanel::default()
        .frame(crate::image_viewer::ui::theme::central_panel_frame(
            ui.ctx(),
        ))
        .show(ui, |ui| {
            match (
                state.context.diff_active,
                get_diff_mode(state),
                content_type,
            ) {
                (true, DiffMode::Glyph | DiffMode::Image, _) | (_, _, ContentType::Rgba) => {
                    draw_rgba_canvas(ui, state)
                }
                (_, _, ContentType::Font) => panels::font_panel::draw_font_canvas(ui, state),
                (_, _, ContentType::Path) => panels::path_panel::draw_path_canvas(ui, state),
                (_, _, ContentType::Indexed) => {
                    panels::indexed_panel::draw_indexed_canvas(ui, state)
                }
                (_, _, ContentType::Glyph) => panels::font_panel::draw_glyph_canvas(ui, state),
            }
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
    if let Some(crate::image_viewer::model::SidebarItem::Glyph(_)) = state.selected_item() {
        return ContentType::Glyph;
    }

    if let Some(image) = state.current_image() {
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
    let (Some(i1), Some(i2)) = (state.diff_image1_id, state.diff_image2_id) else {
        return DiffMode::None;
    };
    let (Some(item1), Some(item2)) = (state.item(i1), state.item(i2)) else {
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

    let selected_index = state.selected_id.and_then(|id| state.index_of(id));
    let mut plot_hover = state.hovered_diff_pixel_from_plot;
    let image_plotter = ImagePlotter::new("viewer")
        .anti_alias(state.context.anti_alias)
        .show_grid(state.context.show_grid)
        .background_color(state.context.background_color)
        .highlight(if state.hovered_diff_pixel.is_none() {
            state.selected_diff_pixel
        } else {
            state.hovered_diff_pixel
        })
        .on_hover(&mut plot_hover);

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
    } else if let Some(crate::image_viewer::model::SidebarItem::Image(image)) =
        selected_index.and_then(|index| state.content_at_mut(index))
    {
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
                        crate::image_viewer::utils::process_images_with_format(
                            &files,
                            state.input_format,
                        )
                        .into_iter()
                        .map(crate::image_viewer::model::SidebarItem::Image)
                        .collect();
                    state.insert_and_select_first(new_items);
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
    state.hovered_diff_pixel_from_plot = plot_hover;
}

fn draw_glyph_diff_canvas(ui: &mut egui::Ui, state: &mut ViewerState) {
    let Some(result) = panels::font_panel::build_selected_glyph_diff_result(state) else {
        ui.centered_and_justified(|ui| {
            ui.label(t!("hint_select_two_fonts_for_diff"));
        });
        return;
    };
    let names = crate::image_viewer::ui::diff_panel::diff_source_names(state);
    let glyph_badge = format!("U+{:04X} · {}", result.codepoint, result.char_repr);
    let img_a = glyph_panel_image_item(
        "glyph_diff_a".to_string(),
        glyph_badge.clone(),
        result.img_a,
    );
    let img_b = glyph_panel_image_item(
        "glyph_diff_b".to_string(),
        glyph_badge.clone(),
        result.img_b,
    );
    let Some((mut diff_img, diff)) = crate::utils::diff_image(
        &img_a,
        &img_b,
        state.context.diff_blend,
        state.context.diff_tolerance,
        state.context.only_show_diff,
    ) else {
        ui.centered_and_justified(|ui| {
            ui.label(t!("hint_select_two_fonts_for_diff"));
        });
        return;
    };
    let diff_pixels = diff.diff_filter(state.context.diff_tolerance).count();
    diff_img.path = format!("glyph diff U+{:04X}", result.codepoint);
    diff_img.info.format = format!("glyph diff · {}", result.char_repr);
    state.context.min_diff = diff.min_diff() + 1.0;
    state.context.max_diff = diff.max_diff() + 1.0;
    state.diff_result = Some((diff_img.clone(), diff.clone()));

    let panels = [
        GlyphDiffPanel {
            title: format!("A: {}", names.0.unwrap_or_else(|| "font A".to_string())),
            id: "glyph_diff_a",
            image: img_a,
            badge: glyph_badge.clone(),
            highlight: None,
        },
        GlyphDiffPanel {
            title: format!("diff · {diff_pixels} pixels"),
            id: "glyph_diff_middle",
            image: diff_img,
            badge: format!("{}×{} · diff", result.diff.size().0, result.diff.size().1),
            highlight: if state.hovered_diff_pixel.is_none() {
                state.selected_diff_pixel
            } else {
                state.hovered_diff_pixel
            },
        },
        GlyphDiffPanel {
            title: format!("B: {}", names.1.unwrap_or_else(|| "font B".to_string())),
            id: "glyph_diff_b",
            image: img_b,
            badge: glyph_badge,
            highlight: None,
        },
    ];

    ui.add_space(10.0);
    ui.columns(3, |columns| {
        for (column, panel) in columns.iter_mut().zip(panels) {
            draw_glyph_diff_image_panel(
                column,
                &mut state.glyph_canvas_view,
                state.context.anti_alias,
                state.context.background_color,
                panel,
            );
        }
    });
}

struct GlyphDiffPanel {
    title: String,
    id: &'static str,
    image: ImageItem,
    badge: String,
    highlight: Option<[u32; 2]>,
}

fn draw_glyph_diff_image_panel(
    ui: &mut egui::Ui,
    view: &mut GlyphCanvasView,
    anti_alias: bool,
    background_color: egui::Color32,
    panel: GlyphDiffPanel,
) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface0)
        .stroke(egui::Stroke::new(1.0, p.surface1))
        .corner_radius(crate::image_viewer::ui::theme::RADIUS_LG)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(panel.title)
                        .strong()
                        .color(p.text)
                        .size(13.0),
                );
            });
            ui.add_space(8.0);
            draw_synced_image_canvas(
                ui,
                view,
                panel.id,
                &panel.image,
                panel.badge,
                panel.highlight,
                anti_alias,
                background_color,
            );
        });
}

fn draw_synced_image_canvas(
    ui: &mut egui::Ui,
    view: &mut GlyphCanvasView,
    id: &str,
    image: &ImageItem,
    badge: String,
    highlight: Option<[u32; 2]>,
    anti_alias: bool,
    background_color: egui::Color32,
) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    let size = ui.available_size_before_wrap();
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let canvas_rect = response.rect;
    let painter = painter.with_clip_rect(canvas_rect);
    let (pixels, width, height) = image.current_pixels();

    if width == 0 || height == 0 {
        return;
    }

    if response.hovered() {
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
        view.pan += response.drag_delta();
        view.pan += scroll_delta;
    }

    if response.double_clicked() {
        view.zoom = 1.0;
        view.pan = egui::Vec2::ZERO;
    }

    let color_image = egui::ColorImage::new([width as usize, height as usize], pixels.to_vec());
    let texture = ui.ctx().load_texture(
        format!("glyph_diff_canvas_{id}"),
        color_image,
        if anti_alias {
            egui::TextureOptions::LINEAR
        } else {
            egui::TextureOptions::NEAREST
        },
    );

    if ui.is_rect_visible(canvas_rect) {
        painter.rect(
            canvas_rect,
            crate::image_viewer::ui::theme::RADIUS,
            if background_color.a() > 0 {
                background_color
            } else {
                p.surface0
            },
            egui::Stroke::new(1.0, p.surface1),
            egui::StrokeKind::Inside,
        );

        let fit_rect = canvas_rect.shrink(16.0);
        let fit_scale = (fit_rect.width() / width as f32).min(fit_rect.height() / height as f32);
        let scale = fit_scale * view.zoom;
        let image_size = egui::vec2(width as f32 * scale, height as f32 * scale);
        let image_rect = egui::Rect::from_center_size(canvas_rect.center() + view.pan, image_size);
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        if let Some([x, y]) = highlight {
            let px = image_rect.left() + x as f32 * scale;
            let py = image_rect.top() + y as f32 * scale;
            let rect = egui::Rect::from_min_size(egui::pos2(px, py), egui::vec2(scale, scale));
            painter.rect_stroke(
                rect.expand(1.0),
                egui::CornerRadius::ZERO,
                egui::Stroke::new(2.0, p.accent()),
                egui::StrokeKind::Inside,
            );
        }

        draw_canvas_badge(ui, canvas_rect, &badge);
    }
}

fn draw_canvas_badge(ui: &mut egui::Ui, canvas_rect: egui::Rect, badge: &str) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    let galley =
        ui.painter()
            .layout_no_wrap(badge.to_string(), egui::FontId::monospace(11.0), p.overlay0);
    let pad = egui::vec2(10.0, 4.0);
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(
            canvas_rect.right() - galley.size().x - pad.x - 8.0,
            canvas_rect.top() + 8.0,
        ),
        galley.size() + pad * 2.0,
    );
    ui.painter().rect(
        badge_rect,
        egui::CornerRadius::same(4),
        p.mantle,
        egui::Stroke::new(1.0, p.surface1),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        badge_rect.center() - 0.5 * galley.size(),
        galley,
        p.overlay0,
    );
}

fn glyph_panel_image_item(
    path: String,
    format: String,
    image: icu_lib::image::RgbaImage,
) -> crate::image_viewer::model::ImageItem {
    crate::image_viewer::utils::single_image_item(
        path,
        icu_lib::endecoder::ImageInfo {
            width: image.width(),
            height: image.height(),
            data_size: image.len() as u32,
            format,
            other_info: serde_json::Value::Null,
        },
        image,
        None,
    )
}

fn draw_frame_controls(ui: &mut egui::Ui, image: &mut crate::image_viewer::model::ImageItem) {
    let FrameSource::Animated {
        frames,
        current,
        autoplay,
        last_advance,
    } = &mut image.frames
    else {
        return;
    };
    if frames.len() <= 1 {
        return;
    }

    ui.horizontal(|ui| {
        let label = if *autoplay {
            t!("ctx_pause")
        } else {
            t!("ctx_play")
        };
        if ui.button(label).clicked() {
            *autoplay = !*autoplay;
            *last_advance = None;
            ui.ctx().request_repaint();
        }

        let mut frame_index = *current;
        let frame_label = t!("frame_label", current = *current + 1, total = frames.len());
        let slider = egui::Slider::new(&mut frame_index, 0..=frames.len() - 1).text(frame_label);
        if ui.add(slider).changed() {
            *current = frame_index;
            *last_advance = None;
        }
    });
}
/// Renders a serializable value as a YAML tree.
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
