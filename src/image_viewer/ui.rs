use crate::cus_component::toggle;
use crate::image_viewer::model::{DiffSorting, ViewerState};
use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use eframe::egui::color_picker::Alpha;
use eframe::egui::{Color32, Sense};
use serde::Serialize;

pub fn draw_top_panel(ctx: &egui::Context, state: &mut ViewerState) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_height(30.0);
            egui::widgets::global_theme_preference_switch(ui);
            ui.separator();
            egui::ComboBox::from_id_salt("Language")
                .selected_text(t!("language"))
                .show_ui(ui, |ui| {
                    let lang_choices = [
                        ("en-US", "English"),
                        ("zh-CN", "简体中文"),
                    ];
                    for (code, label) in lang_choices {
                        if ui
                            .selectable_value(&mut state.context.language, code.to_owned(), label)
                            .clicked()
                        {
                            rust_i18n::set_locale(code);
                        }
                    }
                });
            ui.separator();
            ui.toggle_value(&mut state.context.show_grid, t!("show_grid"));
            ui.toggle_value(&mut state.context.anti_alias, t!("anti_aliasing"));

            ui.separator();
            if ui.button(t!("clear")).clicked() {
                state.context.background_color =
                    state.context.background_color.linear_multiply(0.0);
            }
            egui::widgets::color_picker::color_edit_button_srgba(
                ui,
                &mut state.context.background_color,
                Alpha::BlendOrAdditive,
            );

            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.toggle_value(&mut state.context.image_diff, t!("image_diff"));
                },
            );
        });
    });
}

pub fn draw_bottom_panel(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let show_lesser = ui.ctx().viewport_rect().width() <= 450.0;
        use egui::special_emojis::GITHUB;

        ui.horizontal_wrapped(|ui| {
            egui::widgets::global_theme_preference_switch(ui);
            ui.separator();
            if show_lesser {
                ui.heading("ICU");
            } else {
                ui.heading("Image Converter Ultra");
            }

            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to(
                        format!("v{VERSION}"),
                        format!("{}/releases", env!("CARGO_PKG_REPOSITORY")),
                    );

                    #[cfg(not(target_arch = "wasm32"))]
                    let str_web_version;

                    let str_cli_version;
                    let str_source_code;

                    if show_lesser {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            str_web_version = "🌐".to_string();
                        }
                        str_cli_version = ">_".to_string();
                        str_source_code = format!("{GITHUB}");
                    } else {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            str_web_version = format!("🌐 {}", t!("web_version"));
                        }
                        str_cli_version = format!(">_ {}", t!("cli_version"));
                        str_source_code = format!("{GITHUB} {}", t!("source_code"));
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.separator();
                        ui.hyperlink_to(
                            str_web_version,
                            format!("{}i", env!("CARGO_PKG_HOMEPAGE")),
                        );
                    }
                    ui.separator();
                    ui.hyperlink_to(str_cli_version, env!("CARGO_PKG_HOMEPAGE"));
                    ui.separator();
                    ui.hyperlink_to(str_source_code, env!("CARGO_PKG_REPOSITORY"));
                });
            });
        });
    });
}

pub fn draw_left_panel(
    ctx: &egui::Context,
    state: &mut ViewerState,
    reset_callback: impl FnOnce(&mut ViewerState),
) {
    if state.image_items.len() > 1 {
        egui::SidePanel::left("ImagePicker").show(ctx, |ui| {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(egui::RichText::new("🗑").color(Color32::RED))
                    .clicked()
                {
                    state.image_items.clear();
                    reset_callback(state);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, image_item) in state.image_items.iter().enumerate() {
                    let is_selected = state.selected_image_item_index == Some(index);
                    egui::containers::Frame::default()
                        .inner_margin(6.0)
                        .outer_margin(6.0)
                        .corner_radius(10.0)
                        .show(ui, |ui| {
                            ui.set_height(100.0);
                            let one_sample = ui.vertical_centered(|ui| {
                                ui.vertical_centered(|ui| {
                                    let mut image_plotter = ImagePlotter::new(index.to_string())
                                        .anti_alias(state.context.anti_alias)
                                        .show_grid(false)
                                        .show_only(true);

                                    image_plotter.show(ui, &Some(image_item.clone()));
                                    ui.add(egui::Label::new(&image_item.path).truncate());
                                });
                            });

                            if state.context.image_diff {
                                ui.add_space(8.0);

                                // diff buttons
                                ui.horizontal(|ui| {
                                    let diff1_selected = state.diff_image1_index == Some(index);
                                    let diff2_selected = state.diff_image2_index == Some(index);
                                    if ui.selectable_label(diff1_selected, t!("diff1")).clicked() {
                                        if state.diff_image1_index == Some(index) {
                                            state.diff_image1_index = None;
                                        } else {
                                            state.diff_image1_index = Some(index);
                                            // avoid selecting the same image
                                            if state.diff_image2_index == Some(index) {
                                                state.diff_image2_index = None;
                                            }
                                        }
                                    }
                                    if ui.selectable_label(diff2_selected, t!("diff2")).clicked() {
                                        if state.diff_image2_index == Some(index) {
                                            state.diff_image2_index = None;
                                        } else {
                                            state.diff_image2_index = Some(index);
                                            if state.diff_image1_index == Some(index) {
                                                state.diff_image1_index = None;
                                            }
                                        }
                                    }
                                });
                            }

                            let response = one_sample.response;
                            let visuals = ui.style().interact_selectable(&response, is_selected);
                            let rect = response.rect;
                            let response = ui.allocate_rect(rect, Sense::click());
                            if response.clicked() {
                                state.selected_image_item_index = Some(index);
                                state.current_image = Some(image_item.clone());
                            }
                            if response.hovered() {
                                state.hovered_image_item_index = Some(index);
                            }

                            if is_selected
                                || response.hovered()
                                || response.highlighted()
                                || response.has_focus()
                            {
                                let rect = rect.expand(10.0);
                                let painter = ui.painter_at(rect);
                                let rect = rect.expand(-2.0);
                                painter.rect(
                                    rect,
                                    10.0,
                                    Color32::TRANSPARENT,
                                    egui::Stroke::new(2.0, ui.style().visuals.hyperlink_color),
                                    egui::StrokeKind::Inside,
                                );
                                painter.rect(
                                    rect,
                                    10.0,
                                    visuals.text_color().linear_multiply(0.3),
                                    egui::Stroke::NONE,
                                    egui::StrokeKind::Inside,
                                );
                            }
                        });
                }
            })
        });
    }
}

pub fn draw_right_panel(ctx: &egui::Context, state: &mut ViewerState) {
    if state.context.image_diff {
        egui::SidePanel::right("DiffPanel")
            .exact_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.y = 6.0;
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
                                egui::Slider::new(&mut state.context.diff_blend, 0.0..=1.0)
                                    .text(t!("diff_blend")),
                            );

                            if diff_blend_slider.double_clicked() {
                                state.context.diff_blend = 0.5;
                            }

                            ui.horizontal(|ui| {
                                let avail = diff_blend_slider.interact_rect.width();
                                let btn_w = 50.0;
                                let btn_h = 20.0;
                                let total_btn = btn_w * 3.0;
                                let spacing = ((avail - total_btn) / 2.0).max(0.0);

                                let diff1_selected =
                                    (state.context.diff_blend - 0.0).abs() < f32::EPSILON;
                                let blended_selected =
                                    (state.context.diff_blend - 0.5).abs() < f32::EPSILON;
                                let diff2_selected =
                                    (state.context.diff_blend - 1.0).abs() < f32::EPSILON;

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

                            ui.add(toggle(t!("fast_switch"), &mut state.context.fast_switch));
                            if state.context.fast_switch {
                                ui.add(
                                    egui::Slider::new(
                                        &mut state.context.fast_switch_speed,
                                        0.5..=10.0,
                                    )
                                    .text(t!("switch_speed")),
                                );
                            }
                        });
                } else {
                    state.context.fast_switch = false;
                }

                ui.separator();

                state.hovered_diff_pixel = None;
                if let Some((_, diff_result)) = &state.diff_result {
                    if let (Some(i1), Some(i2)) = (state.diff_image1_index, state.diff_image2_index)
                        && i1 != i2
                    {
                        let mut diff_pixels: Vec<_> = diff_result
                            .diff_filter(state.context.diff_tolerance)
                            .collect();

                        // Sort
                        match state.context.diff_sorting {
                            DiffSorting::Z => {
                                // Default is already Z-like (row major) usually, but ensure it:
                                diff_pixels.sort_by_key(|p| (p.pos.1, p.pos.0));
                            }
                            DiffSorting::N => {
                                diff_pixels.sort_by_key(|p| (p.pos.0, p.pos.1));
                            }
                            DiffSorting::ReverseZ => {
                                diff_pixels.sort_by_key(|p| {
                                    (std::cmp::Reverse(p.pos.1), std::cmp::Reverse(p.pos.0))
                                });
                            }
                            DiffSorting::ReverseN => {
                                diff_pixels.sort_by_key(|p| {
                                    (std::cmp::Reverse(p.pos.0), std::cmp::Reverse(p.pos.1))
                                });
                            }
                            DiffSorting::DiffAsc => {
                                diff_pixels.sort_by(|a, b| {
                                    let diff_a =
                                        a.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                                    let diff_b =
                                        b.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                                    diff_a
                                        .partial_cmp(&diff_b)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                            }
                            DiffSorting::DiffDesc => {
                                diff_pixels.sort_by(|a, b| {
                                    let diff_a =
                                        a.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                                    let diff_b =
                                        b.diff.iter().cloned().reduce(f32::max).unwrap_or(0.0);
                                    diff_b
                                        .partial_cmp(&diff_a)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                            }
                        }

                        // Controls
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_label(t!("sort"))
                                .selected_text(format!("{:?}", state.context.diff_sorting))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::Z,
                                        "Z",
                                    );
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::N,
                                        "N",
                                    );
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::ReverseZ,
                                        "Rev Z",
                                    );
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::ReverseN,
                                        "Rev N",
                                    );
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::DiffAsc,
                                        "Diff Asc",
                                    );
                                    ui.selectable_value(
                                        &mut state.context.diff_sorting,
                                        DiffSorting::DiffDesc,
                                        "Diff Desc",
                                    );
                                });
                        });

                        // Auto jump to page
                        if let Some(hovered) = state.hovered_diff_pixel_from_plot {
                            if let Some(index) = diff_pixels
                                .iter()
                                .position(|p| p.pos.0 == hovered[0] && p.pos.1 == hovered[1])
                            {
                                state.context.diff_page_index =
                                    index / state.context.diff_page_size;
                            }
                        }

                        let total_pixels = diff_pixels.len();
                        let total_pages = (total_pixels + state.context.diff_page_size - 1)
                            / state.context.diff_page_size.max(1);

                        if state.context.diff_page_index >= total_pages {
                            state.context.diff_page_index = total_pages.saturating_sub(1);
                        }

                        ui.horizontal(|ui| {
                            if ui.button("<").clicked() && state.context.diff_page_index > 0 {
                                state.context.diff_page_index -= 1;
                            }
                            ui.label(format!(
                                "{}/{}",
                                state.context.diff_page_index + 1,
                                total_pages
                            ));
                            if ui.button(">").clicked()
                                && state.context.diff_page_index + 1 < total_pages
                            {
                                state.context.diff_page_index += 1;
                            }
                            ui.label(format!("Total: {total_pixels}"));
                        });

                        let start = state.context.diff_page_index * state.context.diff_page_size;
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
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let mut target_rect = None;
                            for diff_pixel in diff_pixels
                                .into_iter()
                                .skip(start)
                                .take(state.context.diff_page_size)
                            {
                                let is_selected = state.selected_diff_pixel
                                    == Some([diff_pixel.pos.0, diff_pixel.pos.1]);
                                let is_hovered = state.hovered_diff_pixel_from_plot
                                    == Some([diff_pixel.pos.0, diff_pixel.pos.1]);

                                egui::containers::Frame::default()
                                    .inner_margin(2.0)
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        let mut color1 = diff_pixel.color_rhs.0;
                                        let mut color2 = diff_pixel.color_lhs.0;

                                        let grid_id = format!(
                                            "diff_row_{}_{}",
                                            diff_pixel.pos.0, diff_pixel.pos.1
                                        );
                                        let inner = egui::Grid::new(grid_id)
                                            .num_columns(4)
                                            .spacing([8.0, 4.0])
                                            .min_col_width(60.0)
                                            .show(ui, |ui| {
                                                ui.add(
                                                    egui::Label::new(format!(
                                                        "({}, {})",
                                                        diff_pixel.pos.0, diff_pixel.pos.1
                                                    ))
                                                    .wrap(),
                                                );
                                                ui.color_edit_button_srgba_unmultiplied(
                                                    &mut color1,
                                                );
                                                ui.color_edit_button_srgba_unmultiplied(
                                                    &mut color2,
                                                );
                                                let diff = diff_pixel
                                                    .diff
                                                    .into_iter()
                                                    .reduce(f32::max)
                                                    .unwrap_or(0.0);
                                                ui.add(
                                                    egui::Label::new(format!("{diff:.3}")).wrap(),
                                                );
                                                ui.end_row();
                                            });

                                        let response = inner.response;
                                        let rect = response.rect;
                                        let response = ui.allocate_rect(rect, Sense::click());

                                        if response.clicked() {
                                            if state.selected_diff_pixel
                                                == Some([diff_pixel.pos.0, diff_pixel.pos.1])
                                            {
                                                state.selected_diff_pixel = None;
                                            } else {
                                                state.selected_diff_pixel =
                                                    Some([diff_pixel.pos.0, diff_pixel.pos.1]);
                                            }
                                        }
                                        if response.hovered() {
                                            state.hovered_diff_pixel =
                                                Some([diff_pixel.pos.0, diff_pixel.pos.1]);
                                        }

                                        if is_selected
                                            || response.hovered()
                                            || response.highlighted()
                                            || response.has_focus()
                                            || is_hovered
                                        {
                                            let rect = rect.expand(4.0);
                                            let painter = ui.painter_at(rect);
                                            let rect = rect.expand(-2.0);
                                            painter.rect(
                                                rect,
                                                4.0,
                                                Color32::TRANSPARENT,
                                                egui::Stroke::new(
                                                    2.0,
                                                    ui.style().visuals.hyperlink_color,
                                                ),
                                                egui::StrokeKind::Inside,
                                            );
                                            painter.rect(
                                                rect,
                                                4.0,
                                                ui.style()
                                                    .visuals
                                                    .hyperlink_color
                                                    .linear_multiply(0.1),
                                                egui::Stroke::NONE,
                                                egui::StrokeKind::Inside,
                                            );

                                            if is_hovered {
                                                target_rect = Some(rect);
                                            }
                                        }
                                    });
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
                }
            });
    }
}

pub fn draw_central_panel(ctx: &egui::Context, state: &mut ViewerState) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let mut image_plotter = ImagePlotter::new("viewer")
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
                image_plotter.show(ui, &Some(diff_img.clone()));
            }
        } else if let Some((diff_img, _)) = &state.diff_result
            && state.context.image_diff
        {
            image_plotter.show(ui, &Some(diff_img.clone()));
        } else {
            image_plotter.show(ui, &state.current_image);
        };
    });
}

pub fn draw_image_info(ctx: &egui::Context, state: &mut ViewerState) {
    if let Some(current_image) = &state.current_image {
        egui::Window::new(t!("image_info")).show(ctx, |ui| {
            egui::Grid::new("info_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(t!("width"));
                    ui.label(format!("{}", current_image.info.width));
                    ui.end_row();

                    ui.label(t!("height"));
                    ui.label(format!("{}", current_image.info.height));
                    ui.end_row();

                    ui.label(t!("format"));
                    ui.label(&current_image.info.format);
                    ui.end_row();

                    ui.label(t!("size"));
                    ui.label(format!("{} bytes", current_image.info.data_size));
                    ui.end_row();
                });

            ui.separator();
            ui.label(t!("other_info"));
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui_tree_view(ui, &current_image.info.other_info);
            });
        });
    }
}

fn ui_tree_view(ui: &mut egui::Ui, value: &impl Serialize) {
    if let Ok(yaml_value) = serde_yaml::to_value(value) {
        ui_yaml_tree(ui, &yaml_value);
    } else {
        ui.label("Error displaying data");
    }
}

fn ui_yaml_tree(ui: &mut egui::Ui, value: &serde_yaml::Value) {
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
