use crate::image_viewer::model::{SidebarItem, ViewerState};
use crate::image_viewer::ui;
use crate::image_viewer::utils::process_images_with_format;
use crate::utils;
use eframe::egui;
use eframe::egui::{Color32, DroppedFile};

enum ExportKind {
    Convert,
    Png,
    None,
}

pub struct MyEguiApp {
    state: ViewerState,
}

impl MyEguiApp {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn install_web_directory_drop(&self, ctx: &egui::Context) {
        crate::image_viewer::utils::install_web_directory_drop(
            self.state.pending_dropped.clone(),
            ctx.clone(),
        );
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        files: Vec<DroppedFile>,
        input_format: crate::converter::ImageFormatCategory,
    ) -> Self {
        log::info!(
            "Starting Egui App with system language: {}",
            crate::image_viewer::utils::get_system_locale()
        );
        let mut state = ViewerState::default();
        state.context = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        state.input_format = input_format;
        state.insert_and_select_first(
            process_images_with_format(&files, state.input_format)
                .into_iter()
                .map(SidebarItem::Image),
        );
        state.context.right_tab = crate::image_viewer::model::RightTab::Info;
        state.context.diff_active = false;
        state.context.only_show_diff = false;

        rust_i18n::set_locale(&state.context.language);

        ui::theme::apply(&cc.egui_ctx);

        Self { state }
    }

    fn reset_state(state: &mut ViewerState) {
        state.clear_items();
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            const MAX_PREVIEW_FILES: usize = 3;
            let (file_count, file_names) = ctx.input(|i| {
                let names = i
                    .raw
                    .hovered_files
                    .iter()
                    .take(MAX_PREVIEW_FILES)
                    .map(|file| {
                        file.path
                            .as_deref()
                            .and_then(std::path::Path::file_name)
                            .map(|name| name.to_string_lossy().into_owned())
                            .filter(|name| !name.is_empty())
                            .unwrap_or_else(|| {
                                if file.mime.is_empty() {
                                    "???".to_owned()
                                } else {
                                    file.mime.clone()
                                }
                            })
                    })
                    .collect::<Vec<_>>();
                (i.raw.hovered_files.len(), names)
            });

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("file_drop_target"),
            ));
            let palette = crate::image_viewer::ui::theme::tokens::palette(ctx);
            let screen_rect = ctx.viewport_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(179));

            let rows = file_names.len() + usize::from(file_count > MAX_PREVIEW_FILES);
            let desired_height = 112.0 + rows as f32 * 18.0;
            let content_size = egui::vec2(
                (screen_rect.width() - 48.0).clamp(0.0, 420.0),
                (screen_rect.height() - 48.0).clamp(0.0, desired_height),
            );
            let content_rect = egui::Rect::from_center_size(screen_rect.center(), content_size);
            crate::image_viewer::ui::viewer::paint_dashed_rect(
                &painter,
                content_rect,
                egui::CornerRadius::same(12),
                palette.accent(),
                8.0,
                6.0,
            );
            painter.text(
                egui::pos2(content_rect.center().x, content_rect.top() + 32.0),
                egui::Align2::CENTER_CENTER,
                t!("dropping_files").trim(),
                egui::FontId::proportional(18.0),
                palette.accent(),
            );
            painter.text(
                egui::pos2(content_rect.center().x, content_rect.top() + 58.0),
                egui::Align2::CENTER_CENTER,
                t!("n_files", count = file_count),
                egui::FontId::proportional(11.0),
                palette.subtext0,
            );
            for (index, name) in file_names.iter().enumerate() {
                painter.text(
                    egui::pos2(
                        content_rect.center().x,
                        content_rect.top() + 82.0 + index as f32 * 18.0,
                    ),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::monospace(11.0),
                    palette.text,
                );
            }
            if file_count > MAX_PREVIEW_FILES {
                painter.text(
                    egui::pos2(
                        content_rect.center().x,
                        content_rect.top() + 82.0 + file_names.len() as f32 * 18.0,
                    ),
                    egui::Align2::CENTER_CENTER,
                    "...",
                    egui::FontId::monospace(11.0),
                    palette.overlay0,
                );
            }
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.state.dropped_files = i.raw.dropped_files.clone();
            }
        });

        if !self.state.dropped_files.is_empty() {
            let new_items: Vec<SidebarItem> =
                process_images_with_format(&self.state.dropped_files, self.state.input_format)
                    .into_iter()
                    .map(SidebarItem::Image)
                    .collect();
            let was_empty = self.state.is_empty();
            self.state.insert_and_select_first(new_items);

            if was_empty && !self.state.is_empty() {
                self.state.context.right_tab = crate::image_viewer::model::RightTab::Info;
            }
            self.state.dropped_files.clear();
        }
    }
}

impl eframe::App for MyEguiApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::theme::apply(ctx);

        if self.state.context.diff_active
            && self.state.diff_image1_id.is_none()
            && self.state.diff_image2_id.is_none()
        {
            let font_ids: Vec<_> = self
                .state
                .items()
                .iter()
                .filter_map(|item| match item.content() {
                    SidebarItem::Image(img)
                        if matches!(img.midata, Some(icu_lib::midata::MiData::FONT(_))) =>
                    {
                        Some(item.id())
                    }
                    _ => None,
                })
                .collect();
            if font_ids.len() >= 2 {
                self.state.diff_image1_id = Some(font_ids[0]);
                self.state.diff_image2_id = Some(font_ids[1]);
            } else {
                let image_ids: Vec<_> = self
                    .state
                    .items()
                    .iter()
                    .filter_map(|item| match item.content() {
                        SidebarItem::Image(_) => Some(item.id()),
                        _ => None,
                    })
                    .collect();
                if image_ids.len() >= 2 {
                    self.state.diff_image1_id = Some(image_ids[0]);
                    self.state.diff_image2_id = Some(image_ids[1]);
                }
            }
        }

        if crate::image_viewer::ui::viewer::get_diff_mode(&self.state)
            == crate::image_viewer::ui::viewer::DiffMode::Image
            && let (Some(i1), Some(i2)) = (self.state.diff_image1_id, self.state.diff_image2_id)
            && i1 != i2
        {
            let img1 = match self.state.item(i1) {
                Some(SidebarItem::Image(i)) => i.clone(),
                _ => {
                    self.state.diff_result = None;
                    return;
                }
            };
            let img2 = match self.state.item(i2) {
                Some(SidebarItem::Image(i)) => i.clone(),
                _ => {
                    self.state.diff_result = None;
                    return;
                }
            };
            let diff_result = utils::diff_image(
                &img1,
                &img2,
                self.state.context.diff_blend,
                self.state.context.diff_tolerance,
                self.state.context.only_show_diff,
            );
            self.state.diff_result = diff_result.map(|(img, diff_result)| {
                self.state.context.min_diff = diff_result.min_diff() + 1.0;
                self.state.context.max_diff = diff_result.max_diff() + 1.0;
                (img, diff_result)
            });
        } else {
            self.state.diff_result = None;
        }

        if self.state.context.diff_active
            && self.state.context.fast_switch
            && !self.state.context.only_show_diff
        {
            let dt = ctx.input(|i| i.stable_dt);
            self.state.context.fast_switch_phase += dt * self.state.context.fast_switch_speed;
            if self.state.context.fast_switch_phase > 1.0 {
                self.state.context.fast_switch_phase -= 1.0;
            }
            let phase = self.state.context.fast_switch_phase;
            self.state.context.diff_blend = if phase < 0.5 { 0.0 } else { 1.0 };
        }

        if self.state.context.fast_switch && !self.state.context.only_show_diff {
            ctx.request_repaint();
        }

        let mod_down =
            ctx.input(|i| i.modifiers.mac_cmd || i.modifiers.ctrl || i.modifiers.command);
        if mod_down {
            if ctx.input(|i| i.key_pressed(egui::Key::O)) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let files: Vec<DroppedFile> = rfd::FileDialog::new()
                        .pick_files()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| DroppedFile {
                            path: Some(p),
                            ..Default::default()
                        })
                        .collect();
                    if !files.is_empty() {
                        let new_items: Vec<SidebarItem> =
                            process_images_with_format(&files, self.state.input_format)
                                .into_iter()
                                .map(SidebarItem::Image)
                                .collect();
                        self.state.insert_and_select_first(new_items);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    crate::image_viewer::utils::pick_files_web(
                        self.state.pending_dropped.clone(),
                        ctx.clone(),
                    );
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::D)) {
                use crate::image_viewer::model::RightTab;
                if self.state.context.right_tab == RightTab::Diff {
                    self.state.context.right_tab = RightTab::Info;
                    self.state.context.diff_active = false;
                } else {
                    self.state.context.right_tab = RightTab::Diff;
                    self.state.context.diff_active = true;
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::E)) {
                use icu_lib::midata::MiData;
                let kind = self
                    .state
                    .selected_id
                    .and_then(|id| self.state.item(id))
                    .and_then(|it| match it {
                        SidebarItem::Image(img) => img.midata.as_ref().map(|m| match m {
                            MiData::RGBA(_) => ExportKind::Convert,
                            MiData::PATH(_) | MiData::INDEXED(_) => ExportKind::Png,
                            _ => ExportKind::None,
                        }),
                        SidebarItem::Glyph(_) => Some(ExportKind::Convert),
                    });
                match kind {
                    Some(ExportKind::Convert) => {
                        self.state.context.right_tab =
                            crate::image_viewer::model::RightTab::Convert;
                    }
                    Some(ExportKind::Png) => {
                        crate::image_viewer::ui::panels::export_current_as_png(&self.state);
                    }
                    _ => {}
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let pending: Vec<DroppedFile> =
                std::mem::take(&mut *self.state.pending_dropped.borrow_mut());
            if !pending.is_empty() {
                let new_items: Vec<SidebarItem> =
                    process_images_with_format(&pending, self.state.input_format)
                        .into_iter()
                        .map(SidebarItem::Image)
                        .collect();
                let was_empty = self.state.is_empty();
                self.state.insert_and_select_first(new_items);
                if was_empty && !self.state.is_empty() {
                    self.state.context.right_tab = crate::image_viewer::model::RightTab::Info;
                }
            }
        }

        self.ui_file_drag_and_drop(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::draw_top_panel(ui, &mut self.state);
        ui::draw_bottom_panel(ui, &mut self.state);

        ui::draw_left_panel(ui, &mut self.state, |s| {
            Self::reset_state(s);
        });

        ui::draw_right_panel_container(ui, &mut self.state);

        ui::draw_central_panel(ui, &mut self.state);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state.context);
    }
}
