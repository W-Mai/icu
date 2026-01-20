use crate::image_viewer::model::{
    ImageFormat, LvglColorFormat, LvglCompression, LvglVersion, ViewerState,
};
use clap::ValueEnum;
use eframe::egui;
use crate::cus_component::toggle;

/// Draws the convert panel.
pub fn draw_convert_panel(ctx: &egui::Context, state: &mut ViewerState) {
    if state.context.show_convert_panel {
        egui::SidePanel::right("ConvertPanel")
            .exact_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                draw_convert_options(ui, state);
            });
    }
}

/// Draws the options for image conversion.
fn draw_convert_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    egui::Grid::new("convert_grid")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            // Output Format
            ui.label(t!("output_format"));
            egui::ComboBox::from_id_salt("output_format")
                .selected_text(format!("{:?}", state.context.convert_params.output_format))
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

            // LVGL Specific Options
            if state.context.convert_params.output_format == ImageFormat::LVGL {
                ui.label(t!("lvgl_version"));
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
                ui.end_row();

                ui.label(t!("color_format"));
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
                ui.end_row();

                ui.label(t!("compression"));
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
                ui.end_row();

                ui.label(t!("stride_align"));
                ui.add(egui::DragValue::new(
                    &mut state.context.convert_params.stride_align,
                ));
                ui.end_row();

                ui.label(t!("dither"));
                ui.add(toggle("", &mut state.context.convert_params.dither));
                ui.end_row();
            }
        });

    ui.add_space(20.0);

    if state.is_converting {
        ui.centered_and_justified(|ui| {
            ui.label(t!("converting"));
        });
    } else {
        let btn_text = if state.image_items.len() > 1 {
            t!("convert_all")
        } else {
            t!("convert")
        };
        if ui.button(btn_text).clicked() {
            state.is_converting = true;
            crate::image_viewer::utils::save_images(
                &state.image_items,
                &state.context.convert_params,
            );
            state.is_converting = false;
        }
    }
}
