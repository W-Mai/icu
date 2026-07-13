use crate::image_viewer::model::{SidebarItem, ViewerState};
use crate::image_viewer::ui;
use crate::image_viewer::utils::process_images;
use crate::utils;
use eframe::egui;
use eframe::egui::{Color32, DroppedFile};

pub struct MyEguiApp {
    state: ViewerState,
}

impl MyEguiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<DroppedFile>) -> Self {
        log::info!(
            "Starting Egui App with system language: {}",
            crate::image_viewer::utils::get_system_locale()
        );
        let mut state = ViewerState {
            items: process_images(&files)
                .into_iter()
                .map(SidebarItem::Image)
                .collect(),
            context: cc
                .storage
                .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
                .unwrap_or_default(),
            ..Default::default()
        };

        if let Some(SidebarItem::Image(first)) = state.items.first().cloned() {
            state.current_image = Some(first.clone());
            state.selected_index = Some(0);
        }
        rust_i18n::set_locale(&state.context.language);

        ui::theme::apply(&cc.egui_ctx);

        Self { state }
    }

    fn reset_state(state: &mut ViewerState) {
        state.current_image = None;
        state.selected_index = None;
        state.hovered_index = None;
        state.diff_image1_index = None;
        state.diff_image2_index = None;
        state.diff_result = None;
        state.selected_diff_pixel = None;
        state.hovered_diff_pixel = None;
    }

    fn ui_file_drag_and_drop(&mut self, ctx: &egui::Context) {
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("file_drop_target"),
            ));

            let screen_rect = ctx.viewport_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::TextStyle::Heading.resolve(&ctx.global_style()),
                Color32::WHITE,
            );
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.state.dropped_files = i.raw.dropped_files.clone();
            }
        });

        if !self.state.dropped_files.is_empty() {
            let new_items: Vec<SidebarItem> = process_images(&self.state.dropped_files)
                .into_iter()
                .map(SidebarItem::Image)
                .collect();
            self.state.items.extend(new_items);

            if self.state.items.len() == 1 {
                self.state.context.right_tab = crate::image_viewer::model::RightTab::Convert;
            }

            if let Some(SidebarItem::Image(image)) = self.state.items.first().cloned() {
                self.state.current_image = Some(image);
                self.state.selected_index = Some(0);
            }
            self.state.dropped_files.clear();
        }
    }
}

impl eframe::App for MyEguiApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let image_count = self
            .state
            .items
            .iter()
            .filter(|i| matches!(i, SidebarItem::Image(_)))
            .count();
        if self.state.context.diff_active
            && image_count == 2
            && (self.state.diff_image1_index.is_none() && self.state.diff_image2_index.is_none())
        {
            self.state.diff_image1_index = Some(0);
            self.state.diff_image2_index = Some(1);
        }

        if let (Some(i1), Some(i2)) = (self.state.diff_image1_index, self.state.diff_image2_index)
            && i1 != i2
        {
            let img1 = match self.state.items.get(i1) {
                Some(SidebarItem::Image(i)) => i.clone(),
                _ => {
                    self.state.diff_result = None;
                    return;
                }
            };
            let img2 = match self.state.items.get(i2) {
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

        self.ui_file_drag_and_drop(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::draw_top_panel(ui, &mut self.state);
        ui::draw_bottom_panel(ui, &mut self.state);

        ui::draw_left_panel(ui, &mut self.state, |s| {
            Self::reset_state(s);
        });

        ui::draw_right_panel(ui, &mut self.state);
        ui::draw_convert_panel(ui, &mut self.state);

        ui::draw_central_panel(ui, &mut self.state);
        ui::draw_image_info(ui.ctx(), &mut self.state);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state.context);
    }
}
