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
            draw_central_toolbar(ui, state, content_type);
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(crate::image_viewer::ui::theme::tokens::palette(ui.ctx()).mantle)
                .stroke(egui::Stroke::new(
                    1.0,
                    crate::image_viewer::ui::theme::tokens::palette(ui.ctx()).surface0,
                ))
                .corner_radius(crate::image_viewer::ui::theme::RADIUS)
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    paint_checkerboard(ui);
                    match (
                        state.context.diff_active,
                        get_diff_mode(state),
                        content_type,
                    ) {
                        (true, DiffMode::Glyph | DiffMode::Image, _)
                        | (_, _, ContentType::Rgba) => draw_rgba_canvas(ui, state),
                        (_, _, ContentType::Font) => {
                            panels::font_panel::draw_font_canvas(ui, state)
                        }
                        (_, _, ContentType::Path) => {
                            panels::path_panel::draw_path_canvas(ui, state)
                        }
                        (_, _, ContentType::Indexed) => {
                            panels::indexed_panel::draw_indexed_canvas(ui, state)
                        }
                        (_, _, ContentType::Glyph) => {
                            panels::font_panel::draw_glyph_canvas(ui, state)
                        }
                    }
                });
        });
}

fn paint_checkerboard(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    let cell = 12.0;
    let light = p.surface0;
    let dark = p.mantle;
    let cols = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            let min = egui::pos2(
                rect.left() + col as f32 * cell,
                rect.top() + row as f32 * cell,
            );
            let tile = egui::Rect::from_min_size(min, egui::vec2(cell, cell));
            ui.painter().rect_filled(
                tile,
                egui::CornerRadius::ZERO,
                if (row + col) % 2 == 0 { light } else { dark },
            );
        }
    }
}

fn draw_central_toolbar(ui: &mut egui::Ui, state: &mut ViewerState, content_type: ContentType) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.mantle)
        .stroke(egui::Stroke::new(1.0, p.surface0))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            let (name, metadata) = selected_resource_toolbar_text(state);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 28.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if !matches!(content_type, ContentType::Path)
                        && crate::image_viewer::ui::widgets::button_opts(
                            ui,
                            "Fit",
                            crate::image_viewer::ui::widgets::ButtonOpts {
                                small: true,
                                ..Default::default()
                            },
                        )
                        .clicked()
                    {
                        fit_canvas(ui.ctx(), state, content_type);
                    }
                    if matches!(content_type, ContentType::Rgba | ContentType::Indexed)
                        && crate::image_viewer::ui::widgets::button_opts(
                            ui,
                            "1:1",
                            crate::image_viewer::ui::widgets::ButtonOpts {
                                small: true,
                                ..Default::default()
                            },
                        )
                        .clicked()
                    {
                        actual_size_canvas(ui.ctx(), state, content_type);
                    }
                    ui.add_sized(
                        [112.0, 28.0],
                        egui::Label::new(
                            egui::RichText::new(metadata).size(10.0).color(p.overlay0),
                        )
                        .truncate(),
                    );
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_sized(
                            [ui.available_width().max(1.0), 28.0],
                            egui::Label::new(egui::RichText::new(name).size(11.0).color(p.text))
                                .truncate(),
                        );
                    });
                },
            );
        });
}

fn canvas_plot_id(content_type: ContentType) -> egui::Id {
    ImagePlotter::plot_id(match content_type {
        ContentType::Indexed => "indexed_view",
        _ => "viewer",
    })
}

fn actual_size_canvas(ctx: &egui::Context, state: &ViewerState, content_type: ContentType) {
    let Some(image) = state.current_image() else {
        return;
    };
    let plot_id = canvas_plot_id(content_type);
    let Some(mut memory) = egui_plot::PlotMemory::load(ctx, plot_id) else {
        return;
    };
    let frame = memory.transform().frame().size();
    if frame.x <= 0.0 || frame.y <= 0.0 {
        return;
    }
    let center_x = image.width as f64 / 2.0;
    let center_y = -(image.height as f64) / 2.0;
    let half_w = frame.x as f64 / 2.0;
    let half_h = frame.y as f64 / 2.0;
    memory.auto_bounds = false.into();
    memory.set_bounds(egui_plot::PlotBounds::from_min_max(
        [center_x - half_w, center_y - half_h],
        [center_x + half_w, center_y + half_h],
    ));
    memory.store(ctx, plot_id);
    ctx.request_repaint();
}

fn fit_canvas(ctx: &egui::Context, state: &mut ViewerState, content_type: ContentType) {
    let plot_id = canvas_plot_id(content_type);
    if let Some(mut memory) = egui_plot::PlotMemory::load(ctx, plot_id) {
        memory.auto_bounds = true.into();
        memory.store(ctx, plot_id);
    }
    state.render_canvas_view.zoom = 1.0;
    state.render_canvas_view.pan = egui::Vec2::ZERO;
    state.glyph_canvas_view.zoom = 1.0;
    state.glyph_canvas_view.pan = egui::Vec2::ZERO;
    ctx.request_repaint();
}

fn selected_resource_toolbar_text(state: &ViewerState) -> (String, String) {
    if state.context.diff_active {
        let mode = match get_diff_mode(state) {
            DiffMode::Glyph => "Glyph diff",
            DiffMode::Image => "Image diff",
            DiffMode::Incompatible => "Incompatible diff",
            DiffMode::None => "Diff",
        };
        return (mode.to_string(), String::new());
    }

    match state.selected_item() {
        Some(crate::image_viewer::model::SidebarItem::Image(image)) => (
            resource_name(&image.path),
            format!("{}×{} · {}", image.width, image.height, image.info.format),
        ),
        Some(crate::image_viewer::model::SidebarItem::Glyph(glyph)) => {
            (glyph.name.clone(), String::new())
        }
        None => (t!("app_title_short").to_string(), String::new()),
    }
}

fn resource_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
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
    match (
        is_font(a),
        is_font(b),
        supports_pixel_diff(a),
        supports_pixel_diff(b),
    ) {
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

fn supports_pixel_diff(image: &ImageItem) -> bool {
    matches!(
        image.midata,
        Some(MiData::RGBA(_)) | Some(MiData::INDEXED(_)) | None
    )
}

fn draw_rgba_canvas(ui: &mut egui::Ui, state: &mut ViewerState) {
    if state.context.diff_active && get_diff_mode(state) == DiffMode::Glyph {
        draw_glyph_diff_canvas(ui, state);
        return;
    }

    let selected_index = state.selected_id.and_then(|id| state.index_of(id));
    let mut plot_hover = state.hovered_diff_pixel_from_plot;
    let mut image_plotter = ImagePlotter::new("viewer")
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
            image_plotter.show(ui, &Some(diff_img.clone()));
        }
    } else if let Some((diff_img, _)) = &state.diff_result
        && state.context.diff_active
    {
        image_plotter.show(ui, &Some(diff_img.clone()));
    } else if let Some(crate::image_viewer::model::SidebarItem::Image(image)) =
        selected_index.and_then(|index| state.content_at_mut(index))
    {
        let advanced = image.advance_frame();
        if advanced || image.autoplay() {
            ui.ctx().request_repaint();
        }
        let image_for_plot = image.clone();
        if image.frame_count() > 1 {
            let plot_height = (ui.available_height() - 32.0).max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), plot_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    image_plotter.show(ui, &Some(image_for_plot));
                },
            );
            draw_frame_controls(ui, image);
        } else {
            image_plotter.show(ui, &Some(image_for_plot));
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
        let text_pos = rect.center() - 0.5 * galley.size();
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

pub(crate) fn paint_dashed_rect(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_viewer::model::{FrameSource, SidebarItem};
    use eframe::egui::Color32;
    use icu_lib::endecoder::ImageInfo;

    fn pixel_item(path: &str, color: Color32, indexed: bool) -> ImageItem {
        let rgba = icu_lib::image::RgbaImage::from_pixel(
            1,
            1,
            icu_lib::image::Rgba(color.to_srgba_unmultiplied()),
        );
        let midata = if indexed {
            MiData::INDEXED(icu_lib::midata::IndexedImageData {
                rgba,
                palette: vec![color.to_srgba_unmultiplied()],
                indexes: vec![0],
                bpp: 1,
                width: 1,
                height: 1,
            })
        } else {
            MiData::RGBA(rgba)
        };
        ImageItem {
            path: path.to_string(),
            info: ImageInfo {
                width: 1,
                height: 1,
                data_size: 4,
                format: if indexed { "indexed" } else { "rgba" }.to_string(),
                other_info: serde_json::Value::Null,
            },
            width: 1,
            height: 1,
            frames: FrameSource::single(vec![color], 1, 1),
            midata: Some(midata),
            expanded: false,
        }
    }

    fn assert_pixel_diff(left: ImageItem, right: ImageItem) {
        let mut state = ViewerState::default();
        let ids = state.insert_and_select_first([
            SidebarItem::Image(left.clone()),
            SidebarItem::Image(right.clone()),
        ]);
        state.diff_image1_id = Some(ids[0]);
        state.diff_image2_id = Some(ids[1]);

        assert_eq!(get_diff_mode(&state), DiffMode::Image);
        let (_, diff) = crate::utils::diff_image(&left, &right, 0.5, 0.0, false).unwrap();
        assert_eq!(diff.diff_filter(0.0).count(), 1);
    }

    #[test]
    fn indexed_images_support_pixel_diff() {
        assert_pixel_diff(
            pixel_item("left.idx", Color32::BLACK, true),
            pixel_item("right.idx", Color32::WHITE, true),
        );
    }

    #[test]
    fn indexed_and_rgba_images_support_pixel_diff() {
        assert_pixel_diff(
            pixel_item("left.idx", Color32::BLACK, true),
            pixel_item("right.rgba", Color32::WHITE, false),
        );
    }
}
