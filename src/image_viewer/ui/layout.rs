use crate::image_viewer::model::ViewerState;
use eframe::egui;
use eframe::egui::color_picker::Alpha;

/// Draws the top panel containing global settings like theme, grid, anti-aliasing, and background color.
pub fn draw_top_panel(ctx: &egui::Context, state: &mut ViewerState) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_height(30.0);
            egui::widgets::global_theme_preference_switch(ui);

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
                    if ui
                        .toggle_value(&mut state.context.image_diff, t!("image_diff"))
                        .clicked()
                        && state.context.image_diff
                    {
                        state.context.show_convert_panel = false;
                    }
                    if ui
                        .toggle_value(&mut state.context.show_convert_panel, t!("convert_panel"))
                        .clicked()
                        && state.context.show_convert_panel
                    {
                        state.context.image_diff = false;
                    }
                },
            );
        });
    });
}

/// Draws the bottom panel containing version info, language selection, and links.
pub fn draw_bottom_panel(ctx: &egui::Context, state: &mut ViewerState) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let show_lesser = ui.ctx().viewport_rect().width() <= 450.0;
        use egui::special_emojis::GITHUB;

        ui.horizontal_wrapped(|ui| {
            if show_lesser {
                ui.heading("ICU");
            } else {
                ui.heading("Image Converter Ultra");
            }

            ui.separator();

            egui::ComboBox::from_id_salt("Language")
                .selected_text(t!("language"))
                .show_ui(ui, |ui| {
                    let lang_choices = [("en-US", "English"), ("zh-CN", "简体中文")];
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

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                draw_footer_links(ui, VERSION, show_lesser, GITHUB);
            });
        });
    });
}

/// Helper to draw footer links
fn draw_footer_links(ui: &mut egui::Ui, version: &str, show_lesser: bool, github_icon: char) {
    ui.horizontal_wrapped(|ui| {
        ui.hyperlink_to(
            format!("v{version}"),
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
            str_source_code = format!("{github_icon}");
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                str_web_version = format!("🌐 {}", t!("web_version"));
            }
            str_cli_version = format!(">_ {}", t!("cli_version"));
            str_source_code = format!("{github_icon} {}", t!("source_code"));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();
            ui.hyperlink_to(str_web_version, format!("{}i", env!("CARGO_PKG_HOMEPAGE")));
        }
        ui.separator();
        ui.hyperlink_to(str_cli_version, env!("CARGO_PKG_HOMEPAGE"));
        ui.separator();
        ui.hyperlink_to(str_source_code, env!("CARGO_PKG_REPOSITORY"));
    });
}
