use crate::cus_component::toggle;
use crate::image_viewer::model::{DiffSorting, ViewerState};
use clap::ValueEnum;
use eframe::egui;
use eframe::egui::{Color32, Sense};
use icu_lib::endecoder::utils::diff::ImageDiffPixel;

/// Draws the right panel containing difference settings and pixel details.
pub fn draw_right_panel(ctx: &egui::Context, state: &mut ViewerState) {
    if state.context.image_diff {
        egui::SidePanel::right("DiffPanel")
            .exact_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.y = 6.0;

                draw_diff_panel_controls(ui, state);

                ui.separator();

                state.hovered_diff_pixel = None;
                if let Some((_, diff_result)) = &state.diff_result {
                    if let (Some(i1), Some(i2)) = (state.diff_image1_index, state.diff_image2_index)
                    {
                        if i1 != i2 {
                            draw_diff_pixel_list(
                                ui,
                                &mut state.context,
                                &mut state.selected_diff_pixel,
                                &mut state.hovered_diff_pixel,
                                state.hovered_diff_pixel_from_plot,
                                diff_result,
                            );
                        }
                    }
                }
            });
    }
}

/// Draws the control sliders and toggles for the difference view.
fn draw_diff_panel_controls(ui: &mut egui::Ui, state: &mut ViewerState) {
    ui.add(toggle(
        t!("only_show_diff_area"),
        &mut state.context.only_show_diff,
    ));
    ui.add(
        egui::Slider::new(
            &mut state.context.diff_tolerance,
            state.context.min_diff..=state.context.max_diff,
        )
        .text(t!("diff_tolerance")),
    );
    if !state.context.only_show_diff {
        draw_diff_blend_settings(ui, state);
    } else {
        state.context.fast_switch = false;
    }
}

/// Draws preset buttons for diff blend (Diff1, Blended, Diff2).
fn draw_blend_preset_buttons(ui: &mut egui::Ui, state: &mut ViewerState, avail_width: f32) {
    ui.horizontal(|ui| {
        let btn_w = 50.0;
        let btn_h = 20.0;
        let total_btn = btn_w * 3.0;
        let spacing = ((avail_width - total_btn) / 2.0).max(0.0);

        let diff1_selected = (state.context.diff_blend - 0.0).abs() < f32::EPSILON;
        let blended_selected = (state.context.diff_blend - 0.5).abs() < f32::EPSILON;
        let diff2_selected = (state.context.diff_blend - 1.0).abs() < f32::EPSILON;

        if ui
            .add_sized(
                [btn_w, btn_h],
                egui::Button::selectable(diff1_selected, t!("diff1")),
            )
            .clicked()
        {
            state.context.diff_blend = 0.0;
        }

        ui.add_space(spacing);

        if ui
            .add_sized(
                [btn_w, btn_h],
                egui::Button::selectable(blended_selected, t!("blended")),
            )
            .clicked()
        {
            state.context.diff_blend = 0.5;
        }

        ui.add_space(spacing);

        if ui
            .add_sized(
                [btn_w, btn_h],
                egui::Button::selectable(diff2_selected, t!("diff2")),
            )
            .clicked()
        {
            state.context.diff_blend = 1.0;
        }
    });
}

/// Draws the list of pixels that differ between the two images.
fn draw_diff_pixel_list(
    ui: &mut egui::Ui,
    context: &mut crate::image_viewer::model::AppContext,
    selected_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel_from_plot: Option<[u32; 2]>,
    diff_result: &icu_lib::endecoder::utils::diff::ImageDiffResult,
) {
    let diff_pixels = get_sorted_diff_pixels(context, diff_result);

    draw_diff_sorting_controls(ui, context);

    if let Some(hovered) = hovered_diff_pixel_from_plot {
        update_diff_page_from_hover(context, &diff_pixels, hovered);
    }

    let total_pixels = diff_pixels.len();
    let total_pages = (total_pixels + context.diff_page_size - 1) / context.diff_page_size.max(1);

    if context.diff_page_index >= total_pages {
        context.diff_page_index = total_pages.saturating_sub(1);
    }

    draw_diff_pagination_controls(ui, context, total_pages, total_pixels);

    let start = context.diff_page_index * context.diff_page_size;

    draw_diff_list_header(ui);

    ui.separator();
    draw_diff_list_scroll_area(
        ui,
        diff_pixels,
        start,
        context.diff_page_size,
        selected_diff_pixel,
        hovered_diff_pixel,
        hovered_diff_pixel_from_plot,
    );
}

/// Filters and sorts the diff pixels based on current context settings.
fn get_sorted_diff_pixels<'a>(
    context: &crate::image_viewer::model::AppContext,
    diff_result: &'a icu_lib::endecoder::utils::diff::ImageDiffResult,
) -> Vec<&'a ImageDiffPixel> {
    let mut diff_pixels: Vec<_> = diff_result
        .diff_filter(context.diff_tolerance)
        .collect();

    match context.diff_sorting {
        DiffSorting::Z => {
            diff_pixels.sort_by_key(|p| (p.pos.1, p.pos.0));
        }
        DiffSorting::N => {
            diff_pixels.sort_by_key(|p| (p.pos.0, p.pos.1));
        }
        DiffSorting::ReverseZ => {
            diff_pixels.sort_by_key(|p| (std::cmp::Reverse(p.pos.1), std::cmp::Reverse(p.pos.0)));
        }
        DiffSorting::ReverseN => {
            diff_pixels.sort_by_key(|p| (std::cmp::Reverse(p.pos.0), std::cmp::Reverse(p.pos.1)));
        }
        DiffSorting::DiffAsc => {
            diff_pixels.sort_by(|a, b| {
                let diff_a = a.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                let diff_b = b.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        DiffSorting::DiffDesc => {
            diff_pixels.sort_by(|a, b| {
                let diff_a = a.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                let diff_b = b.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                diff_b.partial_cmp(&diff_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
    diff_pixels
}

/// Draws the header row for the diff pixel list.
fn draw_diff_list_header(ui: &mut egui::Ui) {
    egui::Grid::new("diff_header")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .min_col_width(60.0)
        .show(ui, |ui| {
            ui.label("(X, Y)");
            ui.label("color1");
            ui.label("color2");
            ui.label("diff");
        });
}

/// Draws a single row in the diff pixel list. Returns the rect if the row is hovered (for auto-scrolling).
fn draw_diff_list_row(
    ui: &mut egui::Ui,
    selected_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel_from_plot: Option<[u32; 2]>,
    diff_pixel: &ImageDiffPixel,
) -> Option<egui::Rect> {
    let is_selected = *selected_diff_pixel == Some([diff_pixel.pos.0, diff_pixel.pos.1]);
    let is_hovered =
        hovered_diff_pixel_from_plot == Some([diff_pixel.pos.0, diff_pixel.pos.1]);
    let mut target_rect = None;

    egui::containers::Frame::default()
        .inner_margin(2.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            let mut color1 = diff_pixel.color_rhs.0;
            let mut color2 = diff_pixel.color_lhs.0;

            let grid_id = format!("diff_row_{}_{}", diff_pixel.pos.0, diff_pixel.pos.1);
            let inner = egui::Grid::new(grid_id)
                .num_columns(4)
                .spacing([8.0, 4.0])
                .min_col_width(60.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(format!("({}, {})", diff_pixel.pos.0, diff_pixel.pos.1)).wrap());
                    ui.color_edit_button_srgba_unmultiplied(&mut color1);
                    ui.color_edit_button_srgba_unmultiplied(&mut color2);
                    let diff = diff_pixel.diff.into_iter().reduce(f32::max).unwrap_or(0.0);
                    ui.add(egui::Label::new(format!("{diff:.3}")).wrap());
                    ui.end_row();
                });

            let response = inner.response;
            let rect = response.rect;
            let response = ui.allocate_rect(rect, Sense::click());

            if response.clicked() {
                if *selected_diff_pixel == Some([diff_pixel.pos.0, diff_pixel.pos.1]) {
                    *selected_diff_pixel = None;
                } else {
                    *selected_diff_pixel = Some([diff_pixel.pos.0, diff_pixel.pos.1]);
                }
            }
            if response.hovered() {
                *hovered_diff_pixel = Some([diff_pixel.pos.0, diff_pixel.pos.1]);
            }

            if is_selected || response.hovered() || response.highlighted() || response.has_focus() || is_hovered {
                let rect = rect.expand(4.0);
                let painter = ui.painter_at(rect);
                let rect = rect.expand(-2.0);
                painter.rect(
                    rect,
                    4.0,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(2.0, ui.style().visuals.hyperlink_color),
                    egui::StrokeKind::Inside,
                );
                painter.rect(
                    rect,
                    4.0,
                    ui.style().visuals.hyperlink_color.linear_multiply(0.1),
                    egui::Stroke::NONE,
                    egui::StrokeKind::Inside,
                );

                if is_hovered {
                    target_rect = Some(rect);
                }
            }
        });

    target_rect
}

fn draw_diff_blend_settings(ui: &mut egui::Ui, state: &mut ViewerState) {
    egui::containers::Frame::new()
        .inner_margin(6.0)
        .outer_margin(4.0)
        .stroke(egui::Stroke::new(
            1.0,
            ui.style().visuals.widgets.noninteractive.fg_stroke.color,
        ))
        .corner_radius(6.0)
        .show(ui, |ui| {
            let diff_blend_slider = ui.add(
                egui::Slider::new(&mut state.context.diff_blend, 0.0..=1.0).text(t!("diff_blend")),
            );

            if diff_blend_slider.double_clicked() {
                state.context.diff_blend = 0.5;
            }

            draw_blend_preset_buttons(ui, state, diff_blend_slider.interact_rect.width());

            ui.add(toggle(t!("fast_switch"), &mut state.context.fast_switch));
            if state.context.fast_switch {
                ui.add(
                    egui::Slider::new(&mut state.context.fast_switch_speed, 0.5..=10.0)
                        .text(t!("switch_speed")),
                );
            }
        });
}

fn draw_diff_sorting_controls(
    ui: &mut egui::Ui,
    context: &mut crate::image_viewer::model::AppContext,
) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_label(t!("sort"))
            .selected_text(t!(format!("diff_order.{:?}", context.diff_sorting)))
            .show_ui(ui, |ui| {
                for &variant in DiffSorting::value_variants() {
                    ui.selectable_value(
                        &mut context.diff_sorting,
                        variant,
                        t!(format!("diff_order.{variant:?}")),
                    );
                }
            });
    });
}

fn update_diff_page_from_hover(
    context: &mut crate::image_viewer::model::AppContext,
    diff_pixels: &[&ImageDiffPixel],
    hovered: [u32; 2],
) {
    if let Some(index) = diff_pixels
        .iter()
        .position(|p| p.pos.0 == hovered[0] && p.pos.1 == hovered[1])
    {
        context.diff_page_index = index / context.diff_page_size;
    }
}

fn draw_diff_pagination_controls(
    ui: &mut egui::Ui,
    context: &mut crate::image_viewer::model::AppContext,
    total_pages: usize,
    total_pixels: usize,
) {
    ui.horizontal(|ui| {
        if ui.button("<").clicked() && context.diff_page_index > 0 {
            context.diff_page_index -= 1;
        }
        ui.label(format!(
            "{}/{}",
            context.diff_page_index + 1,
            total_pages
        ));
        if ui.button(">").clicked() && context.diff_page_index + 1 < total_pages {
            context.diff_page_index += 1;
        }
        ui.label(format!("Total: {total_pixels}"));
    });
}

fn draw_diff_list_scroll_area(
    ui: &mut egui::Ui,
    diff_pixels: Vec<&ImageDiffPixel>,
    start: usize,
    page_size: usize,
    selected_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel: &mut Option<[u32; 2]>,
    hovered_diff_pixel_from_plot: Option<[u32; 2]>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut target_rect = None;
        for diff_pixel in diff_pixels.into_iter().skip(start).take(page_size) {
            if let Some(rect) = draw_diff_list_row(
                ui,
                selected_diff_pixel,
                hovered_diff_pixel,
                hovered_diff_pixel_from_plot,
                diff_pixel,
            ) {
                target_rect = Some(rect);
            }
        }

        if let Some(target_rect) = target_rect {
            ui.scroll_to_rect_animation(
                target_rect,
                None,
                egui::style::ScrollAnimation::default(),
            );
        }
    });
}
