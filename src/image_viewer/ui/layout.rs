use crate::image_viewer::model::ViewerState;
use eframe::egui;
use eframe::egui::color_picker::Alpha;

pub fn draw_top_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    let frame = crate::image_viewer::ui::theme::top_panel_frame(ui.ctx());
    egui::Panel::top("top_panel").frame(frame).show(ui, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_height(30.0);

            let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
            ui.label(
                egui::RichText::new(t!("app_title_short"))
                    .size(14.0)
                    .color(p.accent())
                    .strong(),
            );
            ui.separator();

            if ui.button(t!("btn_open")).clicked() {
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
                        use crate::image_viewer::model::SidebarItem;
                        let new_items: Vec<SidebarItem> =
                            crate::image_viewer::utils::process_images(&files)
                                .into_iter()
                                .map(SidebarItem::Image)
                                .collect();
                        state.items.extend(new_items);
                        if let Some(SidebarItem::Image(img)) = state.items.first().cloned() {
                            state.current_image = Some(img);
                            state.selected_index = Some(0);
                        }
                    }
                }
            }

            egui::widgets::global_theme_preference_switch(ui);

            ui.separator();
            crate::image_viewer::ui::widgets::toggle_labeled(
                ui,
                t!("show_grid"),
                &mut state.context.show_grid,
            );
            crate::image_viewer::ui::widgets::toggle_labeled(
                ui,
                t!("anti_aliasing"),
                &mut state.context.anti_alias,
            );

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
        });
    });
}

pub fn draw_bottom_panel(ui: &mut egui::Ui, state: &mut ViewerState) {
    let frame = crate::image_viewer::ui::theme::top_panel_frame(ui.ctx());
    egui::Panel::bottom("bottom_panel")
        .frame(frame)
        .show(ui, |ui| {
            const VERSION: &str = env!("CARGO_PKG_VERSION");
            let show_lesser = ui.ctx().viewport_rect().width() <= 450.0;
            use egui::special_emojis::GITHUB;

            ui.horizontal_wrapped(|ui| {
                let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());
                let small = |text: &str| egui::RichText::new(text).size(11.0).color(p.overlay0);

                ui.label(small(&format!("v{VERSION}")));
                ui.separator();
                let file_count = state
                    .items
                    .iter()
                    .filter(|i| matches!(i, crate::image_viewer::model::SidebarItem::Image(_)))
                    .count();
                ui.label(small(&t!("n_files", count = file_count).to_string()));
                if let Some(idx) = state.selected_index {
                    if let Some(item) = state.items.get(idx) {
                        let name = match item {
                            crate::image_viewer::model::SidebarItem::Image(i) => {
                                std::path::Path::new(&i.path)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| i.path.clone())
                            }
                            crate::image_viewer::model::SidebarItem::Glyph(g) => g.name.clone(),
                        };
                        ui.separator();
                        ui.label(small(&name));
                    }
                }
                ui.separator();
                crate::image_viewer::ui::widgets::kbd(ui, "⌘O");
                ui.label(small(t!("kbd_open").as_ref()));
                crate::image_viewer::ui::widgets::kbd(ui, "⌘D");
                ui.label(small(t!("kbd_diff").as_ref()));
                crate::image_viewer::ui::widgets::kbd(ui, "⌘E");
                ui.label(small(t!("kbd_export").as_ref()));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("Language")
                        .selected_text(t!("language"))
                        .show_ui(ui, |ui| {
                            let lang_choices = [("en-US", "English"), ("zh-CN", "简体中文")];
                            for (code, label) in lang_choices {
                                if ui
                                    .selectable_value(
                                        &mut state.context.language,
                                        code.to_owned(),
                                        label,
                                    )
                                    .clicked()
                                {
                                    rust_i18n::set_locale(code);
                                }
                            }
                        });
                    ui.separator();
                    draw_footer_links(ui, VERSION, show_lesser, GITHUB);
                });
            });
        });
}

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
