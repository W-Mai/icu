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

#[allow(unused_assignments)]
pub fn draw_convert_options(ui: &mut egui::Ui, state: &mut ViewerState) {
    let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

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

    ui.add_space(12.0);

    if state.context.convert_params.output_format == ImageFormat::LVGL {
        crate::image_viewer::ui::widgets::section_card(ui, "LVGL Settings", |ui| {
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
        });
    }

    if state.context.convert_params.output_format == ImageFormat::MIRX {
        crate::image_viewer::ui::widgets::section_card(ui, "MIRX Settings", |ui| {
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
                    ui.label(egui::RichText::new("Level").size(11.0).color(p.subtext0));
                    ui.add(
                        egui::Slider::new(&mut state.context.convert_params.dither_level, 1..=30)
                            .text(""),
                    );
                });
                ui.label(
                    egui::RichText::new("NeuQuant sample factor (1=best quality, 30=fastest)")
                        .size(9.0)
                        .color(p.overlay0),
                );
            }
        });
    }

    ui.add_space(16.0);

    let image_items: Vec<crate::image_viewer::model::ImageItem> = state
        .items
        .iter()
        .filter_map(|i| match i {
            crate::image_viewer::model::SidebarItem::Image(img) => Some(img.clone()),
            _ => None,
        })
        .collect();

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
                crate::image_viewer::utils::save_images(
                    &image_items,
                    &state.context.convert_params,
                );
                state.is_converting = false;
            }
        }
    });
}
