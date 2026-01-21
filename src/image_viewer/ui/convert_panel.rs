use crate::cus_component::toggle;
use crate::image_viewer::model::{
    ImageFormat, LvglColorFormat, LvglCompression, LvglVersion, ViewerState,
};
use clap::ValueEnum;
use eframe::egui;

/// Draws the convert panel.
pub fn draw_convert_panel(ctx: &egui::Context, state: &mut ViewerState) {
    if state.context.show_convert_panel {
        egui::SidePanel::right("ConvertPanel")
            .exact_width(280.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.heading(t!("convert_panel"));
                });
                ui.add_space(12.0);
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(12.0);
                    draw_convert_options(ui, state);
                    ui.add_space(20.0);
                });
            });
    }
}

/// Draws the options for image conversion.
fn draw_convert_options(ui: &mut egui::Ui, state: &mut ViewerState) {
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
                    ui.add(toggle("", &mut state.context.convert_params.dither));
                    ui.end_row();
                });
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
            let btn_text = if state.image_items.len() > 1 {
                t!("convert_all")
            } else {
                t!("convert")
            };

            // Slightly larger button than default, but not huge
            if ui
                .add_sized(
                    [200.0, 32.0],
                    egui::Button::new(egui::RichText::new(btn_text).heading()),
                )
                .clicked()
            {
                state.is_converting = true;
                crate::image_viewer::utils::save_images(
                    &state.image_items,
                    &state.context.convert_params,
                );
                state.is_converting = false;
            }
        }
    });
}

fn draw_section_frame(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::containers::Frame::default()
        .inner_margin(8.0)
        .corner_radius(4.0)
        .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong());
            ui.add_space(4.0);
            add_contents(ui);
        });
}
