use crate::image_viewer::ui::widgets;
use crate::image_viewer::model::{
    ImageFormat, LvglColorFormat, LvglCompression, LvglVersion, ViewerState,
};
use clap::ValueEnum;
use eframe::egui;

#[allow(unused_assignments)]
pub fn draw_convert_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    // General Settings Group
    draw_section_frame(ui, &t!("output_format"), |ui| {
        egui::Grid::new("general_settings_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(false)
            .show(ui, |ui| {
                ui.label(t!("format"));
                egui::ComboBox::from_id_salt("output_format")
                    .selected_text(format!("{:?}", state.context.convert_params.output_format))
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for &format in ImageFormat::value_variants() {
                            ui.selectable_value(
                                &mut state.context.convert_params.output_format,
                                format,
                                format!("{format:?}"),
                            );
                        }
                    });
                ui.end_row();
            });
    });

    ui.add_space(16.0);

    // LVGL Specific Options Group
    if state.context.convert_params.output_format == ImageFormat::LVGL {
        draw_section_frame(ui, "LVGL Settings", |ui| {
            egui::Grid::new("lvgl_settings_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(false)
                .show(ui, |ui| {
                    ui.label(t!("lvgl_version"));
                    egui::ComboBox::from_id_salt("lvgl_version")
                        .selected_text(format!("{:?}", state.context.convert_params.lvgl_version))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for &version in LvglVersion::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.lvgl_version,
                                    version,
                                    format!("{version:?}"),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label(t!("color_format"));
                    egui::ComboBox::from_id_salt("color_format")
                        .selected_text(format!("{:?}", state.context.convert_params.color_format))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for &format in LvglColorFormat::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.color_format,
                                    format,
                                    format!("{format:?}"),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label(t!("compression"));
                    egui::ComboBox::from_id_salt("compression")
                        .selected_text(format!("{:?}", state.context.convert_params.compression))
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for &compression in LvglCompression::value_variants() {
                                ui.selectable_value(
                                    &mut state.context.convert_params.compression,
                                    compression,
                                    format!("{compression:?}"),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label(t!("stride_align"));
                    ui.add(egui::DragValue::new(
                        &mut state.context.convert_params.stride_align,
                    ));
                    ui.end_row();

                    ui.label(t!("dither"));
                    widgets::toggle(ui, &mut state.context.convert_params.dither);
                    ui.end_row();
                });
        });
    }

    // MIRX Specific Options Group
    if state.context.convert_params.output_format == ImageFormat::MIRX {
        draw_section_frame(ui, "MIRX Settings", |ui| {
            egui::Grid::new("mirx_settings_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(false)
                .show(ui, |ui| {
                    ui.label(t!("color_format"));
                    egui::ComboBox::from_id_salt("mirx_color_format")
                        .selected_text(format!("{:?}", state.context.convert_params.color_format))
                        .width(160.0)
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
                    ui.end_row();

                    ui.label(t!("stride_align"));
                    ui.add(egui::DragValue::new(
                        &mut state.context.convert_params.stride_align,
                    ));
                    ui.end_row();

                    ui.label(t!("dither"));
                    widgets::toggle(ui, &mut state.context.convert_params.dither);
                    ui.end_row();
                });

            if state.context.convert_params.dither {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Level");
                    ui.add(
                        egui::Slider::new(
                            &mut state.context.convert_params.dither_level,
                            1..=30,
                        )
                        .text(""),
                    );
                });
                ui.label(
                    egui::RichText::new("NeuQuant sample factor (1=best quality, 30=fastest)")
                        .size(9.0)
                        .color(ui.style().visuals.weak_text_color()),
                );
            }
        });
    }

    ui.add_space(24.0);

    // Convert Action
    ui.vertical_centered(|ui| {
        if state.is_converting {
            ui.spinner();
            ui.add_space(4.0);
            ui.label(t!("converting"));
        } else {
            let image_items: Vec<crate::image_viewer::model::ImageItem> = state
                .items
                .iter()
                .filter_map(|i| match i {
                    crate::image_viewer::model::SidebarItem::Image(img) => Some(img.clone()),
                    _ => None,
                })
                .collect();
            let btn_text = if image_items.len() > 1 {
                t!("convert_all")
            } else {
                t!("convert")
            };

            if ui
                .add_sized(
                    [200.0, 32.0],
                    egui::Button::new(egui::RichText::new(btn_text).heading()),
                )
                .clicked()
            {
                state.is_converting = true;
                crate::image_viewer::utils::save_images(
                    &image_items,
                    &state.context.convert_params,
                );
                state.is_converting = false;
            }
        }
    });
}

fn draw_section_frame(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    widgets::section_card(ui, title, add_contents);
}
