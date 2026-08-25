use crate::image_viewer::model::ViewerState;
use crate::image_viewer::model::{ImageFormat, LvglColorFormat, LvglCompression, LvglVersion};
use clap::ValueEnum;
use eframe::egui;

fn param_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(11.0).color(p.subtext0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add(ui);
        });
    });
}

pub(crate) fn draw_output_format_selector(ui: &mut egui::Ui, state: &mut ViewerState) {
    crate::image_viewer::ui::widgets::section_card(ui, &t!("output_format"), |ui| {
        egui::ComboBox::from_id_salt("output_format")
            .selected_text(format!("{:?}", state.context.convert_params.output_format))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for &format in ImageFormat::value_variants() {
                    ui.selectable_value(
                        &mut state.context.convert_params.output_format,
                        format,
                        format!("{format:?}"),
                    );
                }
            });
    });
}

pub(crate) fn draw_lvgl_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    if state.context.convert_params.output_format == ImageFormat::LVGL {
        crate::image_viewer::ui::widgets::section_card(
            ui,
            t!("section_lvgl_settings").as_ref(),
            |ui| {
                param_row(ui, t!("lvgl_version").as_ref(), |ui| {
                    egui::ComboBox::from_id_salt("lvgl_version")
                        .selected_text(format!("{:?}", state.context.convert_params.lvgl_version))
                        .show_ui(ui, |ui| {
                            for &version in LvglVersion::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.lvgl_version,
                                    version,
                                    format!("{version:?}"),
                                );
                            }
                        });
                });
                param_row(ui, t!("color_format").as_ref(), |ui| {
                    egui::ComboBox::from_id_salt("color_format")
                        .selected_text(format!("{:?}", state.context.convert_params.color_format))
                        .show_ui(ui, |ui| {
                            for &format in LvglColorFormat::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.color_format,
                                    format,
                                    format!("{format:?}"),
                                );
                            }
                        });
                });
                param_row(ui, t!("compression").as_ref(), |ui| {
                    egui::ComboBox::from_id_salt("compression")
                        .selected_text(format!("{:?}", state.context.convert_params.compression))
                        .show_ui(ui, |ui| {
                            for &compression in LvglCompression::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.compression,
                                    compression,
                                    format!("{compression:?}"),
                                );
                            }
                        });
                });
                param_row(ui, t!("stride_align").as_ref(), |ui| {
                    ui.add(egui::DragValue::new(
                        &mut state.context.convert_params.stride_align,
                    ));
                });
                param_row(ui, t!("dither").as_ref(), |ui| {
                    crate::image_viewer::ui::widgets::toggle(
                        ui,
                        &mut state.context.convert_params.dither,
                    );
                });
            },
        );
    }
}

#[allow(unused_assignments)]
pub(crate) fn draw_mirx_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

    if state.context.convert_params.output_format == ImageFormat::MIRX {
        crate::image_viewer::ui::widgets::section_card(
            ui,
            t!("section_mirx_settings").as_ref(),
            |ui| {
                param_row(ui, t!("color_format").as_ref(), |ui| {
                    egui::ComboBox::from_id_salt("mirx_color_format")
                        .selected_text(format!("{:?}", state.context.convert_params.color_format))
                        .show_ui(ui, |ui| {
                            for &format in LvglColorFormat::value_variants() {
                                if matches!(
                                    format,
                                    LvglColorFormat::RGB565
                                        | LvglColorFormat::RGB565Swapped
                                        | LvglColorFormat::RGB888
                                        | LvglColorFormat::RGBA8888
                                        | LvglColorFormat::BGRA8888
                                        | LvglColorFormat::XRGB8888
                                ) {
                                    ui.selectable_value(
                                        &mut state.context.convert_params.color_format,
                                        format,
                                        format!("{format:?}"),
                                    );
                                }
                            }
                        });
                });
                param_row(ui, t!("stride_align").as_ref(), |ui| {
                    ui.add(egui::DragValue::new(
                        &mut state.context.convert_params.stride_align,
                    ));
                });
                param_row(ui, t!("dither").as_ref(), |ui| {
                    crate::image_viewer::ui::widgets::toggle(
                        ui,
                        &mut state.context.convert_params.dither,
                    );
                });

                if state.context.convert_params.dither {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(t!("dither_level"))
                                .size(11.0)
                                .color(p.subtext0),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut state.context.convert_params.dither_level,
                                1..=30,
                            )
                            .text(""),
                        );
                    });
                    ui.label(
                        egui::RichText::new(t!("neuquant_hint"))
                            .size(9.0)
                            .color(p.overlay0),
                    );
                }
            },
        );
    }
}

#[allow(unused_assignments)]
pub fn draw_convert_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

    draw_output_format_selector(ui, state);
    ui.add_space(12.0);
    draw_lvgl_options(ui, state);
    draw_mirx_options(ui, state);
    if matches!(
        state.context.convert_params.output_format,
        ImageFormat::GIF | ImageFormat::APNG
    ) && state.selected_id.is_some_and(|id| {
        state.group_members(id).is_some()
            || state
                .item(id)
                .and_then(|item| item.as_image())
                .is_some_and(|image| image.frame_count() > 1)
    }) {
        crate::image_viewer::ui::widgets::section_card(ui, &t!("section_export"), |ui| {
            param_row(ui, t!("collection_interval").as_ref(), |ui| {
                let response = ui.add(
                    egui::DragValue::new(&mut state.context.convert_params.gif_interval_ms)
                        .range(1..=60_000),
                );
                if response.changed() {
                    if let Some(id) = state.selected_id {
                        state.set_animation_interval(
                            id,
                            std::time::Duration::from_millis(
                                state.context.convert_params.gif_interval_ms.max(1) as u64,
                            ),
                        );
                    }
                }
            });
            param_row(ui, t!("collection_repeat").as_ref(), |ui| {
                let mut infinite = state.context.convert_params.gif_repeat.is_none();
                if crate::image_viewer::ui::widgets::toggle_labeled(
                    ui,
                    t!("collection_repeat_infinite"),
                    &mut infinite,
                )
                .changed()
                {
                    state.context.convert_params.gif_repeat = if infinite { None } else { Some(1) };
                }
                if let Some(repeat) = &mut state.context.convert_params.gif_repeat {
                    ui.add(egui::DragValue::new(repeat).range(1..=u16::MAX));
                }
            });
        });
    }
    ui.add_space(16.0);

    let image_items = if state.primary_target.is_some_and(|target| {
        matches!(
            target,
            crate::image_viewer::model::SelectionTarget::Frame { .. }
        )
    }) {
        state
            .selected_frame()
            .map(|(_, _, image)| vec![image.clone()])
            .unwrap_or_default()
    } else if state.selected_ids.len() > 1 {
        state.selected_image_snapshots()
    } else {
        state
            .selected_image_snapshots()
            .into_iter()
            .next()
            .or_else(|| state.current_image().cloned())
            .into_iter()
            .collect()
    };

    ui.vertical_centered(|ui| {
        if state.is_converting {
            ui.spinner();
            ui.label(
                egui::RichText::new(t!("converting").to_string())
                    .size(12.0)
                    .color(p.overlay0),
            );
        } else {
            let btn_text = if image_items.len() > 1 {
                t!("convert_all")
            } else {
                t!("convert")
            };
            if ui
                .add_sized(
                    [ui.available_width(), 32.0],
                    egui::Button::new(egui::RichText::new(btn_text).heading()),
                )
                .clicked()
            {
                state.is_converting = true;
                let target = crate::image_viewer::utils::export_target_from_selection(state);
                let is_group_or_frame = target.as_ref().is_some_and(|target| {
                    matches!(
                        target,
                        crate::image_viewer::utils::ExportTarget::Frame { .. }
                    ) || matches!(
                        target,
                        crate::image_viewer::utils::ExportTarget::Entry(id)
                            if state.group_members(*id).is_some()
                                || state
                                    .item(*id)
                                    .and_then(|item| item.as_image())
                                    .is_some_and(|image| image.frame_count() > 1)
                    )
                });
                if is_group_or_frame {
                    if let Some(plan) = target
                        .and_then(|target| crate::image_viewer::utils::export_plan(state, target))
                    {
                        crate::image_viewer::utils::save_export_plan(
                            &plan,
                            &state.context.convert_params,
                        );
                    }
                } else {
                    crate::image_viewer::utils::save_images(
                        &image_items,
                        &state.context.convert_params,
                    );
                }
                state.is_converting = false;
            }
        }
    });
}
