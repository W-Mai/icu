use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

pub mod palette {
    use super::Color32;

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    pub struct Theme {
        pub rosewater: Color32,
        pub flamingo: Color32,
        pub pink: Color32,
        pub mauve: Color32,
        pub red: Color32,
        pub maroon: Color32,
        pub peach: Color32,
        pub yellow: Color32,
        pub green: Color32,
        pub teal: Color32,
        pub sky: Color32,
        pub sapphire: Color32,
        pub blue: Color32,
        pub lavender: Color32,
        pub text: Color32,
        pub subtext1: Color32,
        pub subtext0: Color32,
        pub overlay2: Color32,
        pub overlay1: Color32,
        pub overlay0: Color32,
        pub surface2: Color32,
        pub surface1: Color32,
        pub surface0: Color32,
        pub base: Color32,
        pub mantle: Color32,
        pub crust: Color32,
    }

    pub const MOCHA: Theme = Theme {
        rosewater: Color32::from_rgb(245, 224, 220),
        flamingo: Color32::from_rgb(242, 205, 205),
        pink: Color32::from_rgb(245, 194, 231),
        mauve: Color32::from_rgb(203, 166, 247),
        red: Color32::from_rgb(243, 139, 168),
        maroon: Color32::from_rgb(235, 160, 172),
        peach: Color32::from_rgb(250, 179, 135),
        yellow: Color32::from_rgb(249, 226, 175),
        green: Color32::from_rgb(166, 227, 161),
        teal: Color32::from_rgb(148, 226, 213),
        sky: Color32::from_rgb(137, 220, 235),
        sapphire: Color32::from_rgb(116, 199, 236),
        blue: Color32::from_rgb(137, 180, 250),
        lavender: Color32::from_rgb(180, 190, 254),
        text: Color32::from_rgb(205, 214, 244),
        subtext1: Color32::from_rgb(186, 194, 222),
        subtext0: Color32::from_rgb(166, 173, 200),
        overlay2: Color32::from_rgb(147, 154, 183),
        overlay1: Color32::from_rgb(127, 132, 156),
        overlay0: Color32::from_rgb(108, 112, 134),
        surface2: Color32::from_rgb(88, 91, 112),
        surface1: Color32::from_rgb(69, 71, 90),
        surface0: Color32::from_rgb(49, 50, 68),
        base: Color32::from_rgb(30, 30, 46),
        mantle: Color32::from_rgb(24, 24, 37),
        crust: Color32::from_rgb(17, 17, 27),
    };

    pub const LATTE: Theme = Theme {
        rosewater: Color32::from_rgb(220, 138, 120),
        flamingo: Color32::from_rgb(221, 120, 120),
        pink: Color32::from_rgb(234, 118, 203),
        mauve: Color32::from_rgb(136, 57, 239),
        red: Color32::from_rgb(210, 15, 57),
        maroon: Color32::from_rgb(230, 69, 83),
        peach: Color32::from_rgb(254, 100, 11),
        yellow: Color32::from_rgb(223, 142, 29),
        green: Color32::from_rgb(64, 160, 43),
        teal: Color32::from_rgb(23, 146, 153),
        sky: Color32::from_rgb(4, 165, 229),
        sapphire: Color32::from_rgb(32, 159, 181),
        blue: Color32::from_rgb(30, 102, 245),
        lavender: Color32::from_rgb(114, 135, 253),
        text: Color32::from_rgb(76, 79, 105),
        subtext1: Color32::from_rgb(92, 95, 119),
        subtext0: Color32::from_rgb(108, 111, 133),
        overlay2: Color32::from_rgb(124, 127, 147),
        overlay1: Color32::from_rgb(140, 143, 161),
        overlay0: Color32::from_rgb(156, 160, 176),
        surface2: Color32::from_rgb(172, 176, 190),
        surface1: Color32::from_rgb(188, 192, 204),
        surface0: Color32::from_rgb(204, 208, 218),
        base: Color32::from_rgb(239, 241, 245),
        mantle: Color32::from_rgb(230, 233, 239),
        crust: Color32::from_rgb(220, 224, 232),
    };

    #[allow(dead_code)]
    impl Theme {
        pub fn accent(self) -> Color32 {
            self.blue
        }
        pub fn accent_2(self) -> Color32 {
            self.lavender
        }
        pub fn accent_dim(self) -> Color32 {
            let a = self.blue;
            Color32::from_rgba_unmultiplied(a.r(), a.g(), a.b(), 38)
        }
        pub fn accent_dim_of(self, accent: Color32) -> Color32 {
            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 38)
        }
    }
}

pub mod tokens {
    use super::palette::{LATTE, MOCHA};

    #[allow(dead_code)]
    pub fn is_dark(ctx: &eframe::egui::Context) -> bool {
        ctx.global_style().visuals.dark_mode
    }

    pub fn palette(ctx: &eframe::egui::Context) -> super::palette::Theme {
        if is_dark(ctx) {
            MOCHA
        } else {
            LATTE
        }
    }
}

pub const RADIUS: CornerRadius = CornerRadius::same(4);
#[allow(dead_code)]
pub const RADIUS_SM: CornerRadius = CornerRadius::same(3);
#[allow(dead_code)]
pub const RADIUS_LG: CornerRadius = CornerRadius::same(8);

pub fn apply(ctx: &egui::Context) {
    let is_dark = ctx.global_style().visuals.dark_mode;
    let theme = if is_dark {
        palette::MOCHA
    } else {
        palette::LATTE
    };
    apply_theme(ctx, &theme, is_dark);
}

fn apply_theme(ctx: &egui::Context, t: &palette::Theme, is_dark: bool) {
    let shadow_color = if is_dark {
        Color32::from_black_alpha(96)
    } else {
        Color32::from_black_alpha(25)
    };

    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(12.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(12.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::new(11.0, FontFamily::Monospace));

        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.indent = 12.0;
        style.spacing.interact_size = egui::vec2(28.0, 22.0);
        style.spacing.icon_width = 14.0;
        style.spacing.icon_width_inner = 10.0;
        style.spacing.icon_spacing = 6.0;
        style.spacing.slider_rail_height = 4.0;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.floating_width = 4.0;

        let v = &mut style.visuals;
        v.dark_mode = is_dark;
        v.hyperlink_color = t.rosewater;
        v.faint_bg_color = t.surface0;
        v.extreme_bg_color = t.crust;
        v.code_bg_color = t.mantle;
        v.warn_fg_color = t.peach;
        v.error_fg_color = t.maroon;
        v.window_fill = t.base;
        v.panel_fill = t.base;
        v.window_stroke = Stroke::new(1.0, t.overlay1);
        v.window_corner_radius = RADIUS;
        v.menu_corner_radius = RADIUS;
        v.window_shadow = egui::Shadow {
            color: shadow_color,
            ..v.window_shadow
        };
        v.popup_shadow = egui::Shadow {
            color: shadow_color,
            ..v.popup_shadow
        };
        v.selection.bg_fill = t.blue.linear_multiply(if is_dark { 0.2 } else { 0.4 });
        v.selection.stroke = Stroke::new(1.0, t.text);

        v.widgets.noninteractive.bg_fill = t.base;
        v.widgets.noninteractive.weak_bg_fill = t.base;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.overlay1);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
        v.widgets.noninteractive.corner_radius = RADIUS;

        v.widgets.inactive.bg_fill = t.surface0;
        v.widgets.inactive.weak_bg_fill = t.surface0;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.overlay1);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
        v.widgets.inactive.corner_radius = RADIUS;

        v.widgets.hovered.bg_fill = t.surface2;
        v.widgets.hovered.weak_bg_fill = t.surface2;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.overlay1);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text);
        v.widgets.hovered.corner_radius = RADIUS;

        v.widgets.active.bg_fill = t.surface1;
        v.widgets.active.weak_bg_fill = t.surface1;
        v.widgets.active.bg_stroke = Stroke::new(1.0, t.overlay1);
        v.widgets.active.fg_stroke = Stroke::new(1.0, t.text);
        v.widgets.active.corner_radius = RADIUS;

        v.widgets.open.bg_fill = t.surface0;
        v.widgets.open.weak_bg_fill = t.surface0;
        v.widgets.open.bg_stroke = Stroke::new(1.0, t.overlay1);
        v.widgets.open.fg_stroke = Stroke::new(1.0, t.text);
        v.widgets.open.corner_radius = RADIUS;
    });
}

pub fn side_panel_frame(ctx: &egui::Context) -> egui::Frame {
    let p = tokens::palette(ctx);
    egui::Frame::new()
        .fill(p.mantle)
        .stroke(Stroke::new(1.0, p.surface1))
        .inner_margin(Margin::same(8))
}

pub fn top_panel_frame(ctx: &egui::Context) -> egui::Frame {
    let p = tokens::palette(ctx);
    egui::Frame::new()
        .fill(p.mantle)
        .stroke(Stroke::new(1.0, p.surface1))
        .inner_margin(Margin {
            left: 12,
            right: 12,
            top: 4,
            bottom: 4,
        })
}

#[allow(dead_code)]
pub fn panel_frame(ctx: &egui::Context) -> egui::Frame {
    side_panel_frame(ctx)
}

#[allow(dead_code)]
pub fn section_card(ctx: &egui::Context) -> egui::Frame {
    let p = tokens::palette(ctx);
    egui::Frame::new()
        .fill(p.surface0)
        .stroke(Stroke::new(1.0, p.surface0))
        .corner_radius(RADIUS)
        .inner_margin(Margin::same(10))
}
