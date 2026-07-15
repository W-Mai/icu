use crate::image_viewer::ui::theme::{self, RADIUS, RADIUS_SM};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Margin, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};
use egui::epaint::Pos2;

pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let desired_size = Vec2::new(28.0, 16.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let p = theme::tokens::palette(ui.ctx());
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let bg = if *on {
            p.accent()
        } else if response.hovered() {
            p.surface1
        } else {
            p.surface0
        };
        let radius = rect.height() * 0.5;
        ui.painter().rect(
            rect,
            CornerRadius::same(radius.round() as u8),
            bg,
            Stroke::new(1.0, p.surface1),
            StrokeKind::Inside,
        );
        let knob_r = 6.0;
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        let center = Pos2::new(x, rect.center().y);
        let knob_color = if *on { p.base } else { p.overlay0 };
        ui.painter().circle_filled(center, knob_r, knob_color);
    }

    response
}

pub fn toggle_labeled(ui: &mut Ui, label: impl Into<egui::RichText>, on: &mut bool) -> Response {
    ui.horizontal(|ui| {
        let r = toggle(ui, on);
        let label_response = ui.add(egui::Label::new(label.into()).sense(Sense::click()));
        if label_response.clicked() {
            *on = !*on;
        }
        r | label_response
    })
    .inner
}

pub fn mode_tabs<T: Copy + PartialEq>(
    ui: &mut Ui,
    selected: &mut T,
    tabs: &[(T, &str)],
) -> Response {
    let p = theme::tokens::palette(ui.ctx());
    let height = 24.0f32;
    let padding_x = 12.0f32;
    let gap = 0.0f32;

    let text_widths: Vec<f32> = tabs
        .iter()
        .map(|(_, label)| {
            ui.painter()
                .layout_no_wrap(
                    label.to_string(),
                    FontId::proportional(12.0),
                    Color32::TRANSPARENT,
                )
                .size()
                .x
        })
        .collect();
    let total_width: f32 = text_widths.iter().map(|w| w + 2.0 * padding_x).sum::<f32>()
        + gap * (tabs.len().saturating_sub(1)) as f32;
    let desired_size = Vec2::new(total_width, height);
    let (outer, mut response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(outer) {
        ui.painter()
            .rect(outer, RADIUS, p.surface0, Stroke::NONE, StrokeKind::Inside);
    }

    let mut x = outer.left();
    let mut any_clicked = false;
    for (i, (value, label)) in tabs.iter().enumerate() {
        let tab_w = text_widths[i] + 2.0 * padding_x;
        let tab_rect =
            egui::Rect::from_min_size(Pos2::new(x, outer.top()), Vec2::new(tab_w, height));
        let id = ui.make_persistent_id(("mode-tabs", i));
        let tab_response = ui.interact(tab_rect, id, Sense::click());

        if tab_response.clicked() {
            *selected = *value;
            any_clicked = true;
        }

        let is_selected = *selected == *value;
        if ui.is_rect_visible(tab_rect) {
            let fill = if is_selected {
                p.mantle
            } else {
                Color32::TRANSPARENT
            };
            if fill != Color32::TRANSPARENT {
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        Pos2::new(tab_rect.left() + 2.0, tab_rect.top() + 2.0),
                        Vec2::new(tab_w - 4.0, height - 4.0),
                    ),
                    RADIUS_SM,
                    fill,
                );
            }
            let text_color = if is_selected { p.accent() } else { p.overlay0 };
            ui.painter().text(
                tab_rect.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(12.0),
                text_color,
            );
        }

        x += tab_w + gap;
    }

    if any_clicked {
        response.mark_changed();
    }
    response
}

pub fn button(ui: &mut Ui, label: impl Into<egui::RichText>) -> Response {
    button_opts(ui, label, ButtonOpts::default())
}

#[derive(Default)]
pub struct ButtonOpts {
    pub active: bool,
    pub primary: bool,
    pub small: bool,
    pub full_width: bool,
}

pub fn button_opts(ui: &mut Ui, label: impl Into<egui::RichText>, opts: ButtonOpts) -> Response {
    let p = theme::tokens::palette(ui.ctx());
    let label_text: egui::RichText = label.into();
    let font_size = if opts.small { 11.0 } else { 12.0 };
    let pad_x = if opts.small { 6.0 } else { 10.0 };
    let pad_y = if opts.small { 2.0 } else { 4.0 };

    let galley = ui.painter().layout_no_wrap(
        label_text.text().to_string(),
        FontId::proportional(font_size),
        Color32::TRANSPARENT,
    );
    let desired = Vec2::new(galley.size().x + 2.0 * pad_x, galley.size().y + 2.0 * pad_y);
    let desired = if opts.full_width {
        Vec2::new(ui.available_width(), desired.y.max(28.0))
    } else {
        desired
    };
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let (fill, text_color) = if opts.primary {
            if response.hovered() || response.is_pointer_button_down_on() {
                (p.lavender, p.base)
            } else {
                (p.accent(), p.base)
            }
        } else if opts.active {
            (p.accent_dim(), p.accent())
        } else if response.is_pointer_button_down_on() {
            (p.accent_dim(), p.accent())
        } else if response.hovered() {
            (p.surface1, p.text)
        } else {
            (Color32::TRANSPARENT, p.subtext0)
        };

        ui.painter().rect(
            rect,
            RADIUS,
            fill,
            if opts.active || opts.primary {
                Stroke::new(1.0, p.accent())
            } else {
                Stroke::NONE
            },
            StrokeKind::Inside,
        );
        let label_rich = label_text.color(text_color).size(font_size);
        let galley = ui.painter().layout_no_wrap(
            label_rich.text().to_string(),
            FontId::proportional(font_size),
            text_color,
        );
        ui.painter()
            .galley(rect.center() - 0.5 * galley.size(), galley, text_color);
    }

    response
}

pub fn section_card(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui)) {
    let p = theme::tokens::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface0)
        .stroke(Stroke::new(1.0, p.surface0))
        .corner_radius(RADIUS)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            if !title.is_empty() {
                ui.label(
                    egui::RichText::new(title.to_uppercase())
                        .size(10.0)
                        .color(p.overlay0)
                        .strong(),
                );
                ui.add_space(6.0);
            }
            add_contents(ui);
        });
}

pub fn section_header(ui: &mut Ui, title: &str) {
    let p = theme::tokens::palette(ui.ctx());
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .size(10.0)
            .color(p.overlay0)
            .strong(),
    );
}

pub fn chip(ui: &mut Ui, text: &str, color: Color32) {
    let p = theme::tokens::palette(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(9.0), p.base);
    let pad_x = 5.0;
    let pad_y = 1.0;
    let desired = galley.size() + Vec2::new(2.0 * pad_x, 2.0 * pad_y);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::same(3), color);
        ui.painter()
            .galley(rect.center() - 0.5 * galley.size(), galley, p.base);
    }
}

pub fn info_row(ui: &mut Ui, label: &str, value: &str) {
    let p = theme::tokens::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(12.0).color(p.subtext0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .size(12.0)
                    .color(p.text)
                    .family(egui::FontFamily::Monospace),
            );
        });
    });
}

pub fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> Response {
    let p = theme::tokens::palette(ui.ctx());
    let mut slider_resp = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(11.0).color(p.subtext0));
        slider_resp = Some(ui.add(egui::Slider::new(value, range)));
    });
    slider_resp.unwrap_or_else(|| ui.allocate_response(Vec2::ZERO, Sense::hover()))
}

pub fn kbd(ui: &mut Ui, key: &str) {
    let p = theme::tokens::palette(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(key.to_string(), FontId::monospace(10.0), p.overlay0);
    let pad_x = 5.0;
    let pad_y = 1.0;
    let desired = galley.size() + Vec2::new(2.0 * pad_x, 2.0 * pad_y);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            CornerRadius::same(3),
            p.surface0,
            Stroke::new(1.0, p.surface1),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(rect.center() - 0.5 * galley.size(), galley, p.overlay0);
    }
}
