use crate::image_viewer::model::ImageItem;
use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, PointerButton};
use egui_plot::{CoordinatesFormatter, Corner, PlotImage, PlotPoint};
use std::cell::RefCell;
use std::rc::Rc;

pub struct ImagePlotter<'a> {
    id: String,
    anti_alias: bool,
    show_grid: bool,
    show_only: bool,
    background_color: Color32,
    highlight_pixel: Option<[u32; 2]>,
    on_hover: Option<&'a mut Option<[u32; 2]>>,
    badge: Option<String>,
}

impl<'a> ImagePlotter<'a> {
    pub fn new(id: impl ToString) -> ImagePlotter<'a> {
        Self {
            id: id.to_string(),
            anti_alias: false,
            show_grid: false,
            show_only: false,
            background_color: Default::default(),
            highlight_pixel: None,
            on_hover: None,
            badge: None,
        }
    }

    pub fn on_hover(mut self, on_hover: &'a mut Option<[u32; 2]>) -> Self {
        self.on_hover = Some(on_hover);
        self
    }

    pub fn badge(mut self, text: impl ToString) -> Self {
        self.badge = Some(text.to_string());
        self
    }

    pub fn highlight(self, pixel: Option<[u32; 2]>) -> Self {
        let mut s = self;
        s.highlight_pixel = pixel;
        s
    }

    pub fn anti_alias(self, sure: bool) -> Self {
        let mut s = self;
        s.anti_alias = sure;
        s
    }

    pub fn show_grid(self, show: bool) -> Self {
        let mut s = self;
        s.show_grid = show;
        s
    }

    #[allow(dead_code)]
    pub fn show_only(self, only: bool) -> Self {
        let mut s = self;
        s.show_only = only;
        s
    }

    pub fn background_color(self, color: Color32) -> Self {
        let mut s = self;
        s.background_color = color;
        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, image_item: &Option<ImageItem>) {
        let color_data: Rc<RefCell<Option<Color32>>> = Default::default();
        let cursor_pos: Rc<RefCell<Option<[f64; 2]>>> = Default::default();

        match image_item {
            None => {}
            Some(image_item) => {
                let (pixels, width_u32, height_u32) = image_item.current_pixels();
                let width = width_u32 as f32;
                let height = height_u32 as f32;

                let image = ColorImage::new([width as usize, height as usize], pixels.to_vec());
                let texture = ui.ctx().load_texture(
                    format!("showing_image_{}", self.id),
                    image,
                    if self.anti_alias {
                        egui::TextureOptions::LINEAR
                    } else {
                        egui::TextureOptions::NEAREST
                    },
                );

                let texture = SizedTexture::new(texture.id(), [width, height]);

                let img_w = width as f64;
                let img_h = height as f64;
                let copy_image_data = Rc::new(RefCell::new(pixels.to_vec()));

                let copy_image_data_1 = copy_image_data.clone();
                let color_data_1 = color_data.clone();
                let color_data_2 = color_data.clone();
                let cursor_pos_2 = cursor_pos.clone();

                let mut plot = egui_plot::Plot::new(format!("plot{}", self.id))
                    .data_aspect(1.0)
                    .y_axis_formatter(move |y, _| format!("{:.0}", -y.value))
                    .label_formatter(move |hover| {
                        let pos = match hover {
                            egui_plot::HoverPosition::NearDataPoint { position, .. }
                            | egui_plot::HoverPosition::Elsewhere { position } => *position,
                        };
                        if pos.x > 0.0 && pos.x < img_w && pos.y < 0.0 && pos.y > -img_h {
                            let row = -pos.y as usize;
                            let col = pos.x as usize;
                            let index = row * img_w as usize + col;
                            let pixel = &copy_image_data_1.borrow()[index];
                            color_data_2.borrow_mut().replace(*pixel);
                            cursor_pos_2.borrow_mut().replace([pos.x, pos.y]);

                            Some(format!(
                                "Pos: {:.0} {:.0}",
                                pos.x.floor(),
                                -pos.y.floor() - 1.0
                            ))
                        } else {
                            color_data_2.take();
                            cursor_pos_2.take();
                            None
                        }
                    })
                    .boxed_zoom_pointer_button(PointerButton::Extra2)
                    .show_grid([self.show_grid, self.show_grid])
                    .clamp_grid(true)
                    .show_axes([!self.show_only, !self.show_only])
                    .allow_scroll(!self.show_only)
                    .allow_zoom(!self.show_only)
                    .allow_drag(!self.show_only)
                    .show_x(!self.show_only)
                    .show_y(!self.show_only)
                    .show_background(self.background_color.is_additive());

                if !self.show_only {
                    plot = plot.coordinates_formatter(
                        Corner::LeftBottom,
                        CoordinatesFormatter::new(move |p, _b| {
                            let color_data = *color_data_1.borrow();
                            match color_data {
                                None => {
                                    format!("Nothing {:.0} {:.0}", p.x.floor(), p.y.ceil())
                                }
                                Some(pixel) => {
                                    format!(
                                        "RGBA: #{:02X}_{:02X}_{:02X}_{:02X}",
                                        pixel.r(),
                                        pixel.g(),
                                        pixel.b(),
                                        pixel.a(),
                                    )
                                }
                            }
                        }),
                    )
                }

                if self.background_color.a() > 0 {
                    let painter = ui.painter();
                    painter.rect_filled(ui.min_rect(), 0.0, self.background_color);
                }

                let time = ui.input(|i| i.time);

                let plot_response = plot.show(ui, |plot_ui| {
                    plot_ui.image(PlotImage::new(
                        "image",
                        texture.id,
                        PlotPoint::new(img_w / 2.0, -img_h / 2.0),
                        texture.size,
                    ));

                    let plot_bounds = plot_ui.plot_bounds();
                    let plot_size = plot_ui.response().rect;
                    let scale_fact = 1.2f64;
                    let scale = 1.0 / (plot_bounds.width() as f32 / plot_size.width());

                    if let Some([x, y]) = self.highlight_pixel {
                        plot_ui.set_plot_bounds_x(
                            x as f64 - plot_bounds.width() / 2.0
                                ..=x as f64 + plot_bounds.width() / 2.0,
                        );
                        plot_ui.set_plot_bounds_y(
                            -(y as f64 + plot_bounds.height() / 2.0)
                                ..=-(y as f64 - plot_bounds.height() / 2.0),
                        );
                        let center = [x as f64 + 0.5, -(y as f64 + 0.5)];
                        let alpha = (time * 5.0).sin().abs() as f32;
                        let color = Color32::CYAN.linear_multiply(alpha);
                        let stroke_width = 3.0;
                        let radius = 1.5 / scale as f64;

                        plot_ui.polygon(
                            egui_plot::Polygon::new(
                                "highlight",
                                vec![
                                    [
                                        center[0] - 0.5 * scale_fact * scale_fact - radius,
                                        center[1] - 0.5 * scale_fact * scale_fact - radius,
                                    ],
                                    [
                                        center[0] + 0.5 * scale_fact * scale_fact + radius,
                                        center[1] - 0.5 * scale_fact * scale_fact - radius,
                                    ],
                                    [
                                        center[0] + 0.5 * scale_fact * scale_fact + radius,
                                        center[1] + 0.5 * scale_fact * scale_fact + radius,
                                    ],
                                    [
                                        center[0] - 0.5 * scale_fact * scale_fact - radius,
                                        center[1] + 0.5 * scale_fact * scale_fact + radius,
                                    ],
                                ],
                            )
                            .fill_color(Color32::TRANSPARENT)
                            .stroke(egui::Stroke::new(stroke_width, color)),
                        );
                        plot_ui.ctx().request_repaint();
                    }

                    if let Some(pos) = plot_ui.pointer_coordinate() {
                        if !(pos.x > 0.0 && pos.x < img_w && pos.y < 0.0 && pos.y > -img_h) {
                            return;
                        }

                        let row = -pos.y as usize;
                        let col = pos.x as usize;
                        let index = row * img_w as usize + col;
                        let pixel = copy_image_data.borrow()[index];

                        let pos = [pos.x.floor() + 0.5, pos.y.floor() + 0.5];

                        plot_ui.points(
                            egui_plot::Points::new("cursor", vec![pos])
                                .shape(egui_plot::MarkerShape::Square)
                                .radius(scale)
                                .color(pixel),
                        );

                        plot_ui.polygon(
                            egui_plot::Polygon::new(
                                "cursor",
                                vec![
                                    [
                                        pos[0] - 0.5 * scale_fact * scale_fact,
                                        pos[1] - 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        pos[0] + 0.5 * scale_fact * scale_fact,
                                        pos[1] - 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        pos[0] + 0.5 * scale_fact * scale_fact,
                                        pos[1] + 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        pos[0] - 0.5 * scale_fact * scale_fact,
                                        pos[1] + 0.5 * scale_fact * scale_fact,
                                    ],
                                ],
                            )
                            .fill_color(pixel)
                            .stroke(egui::Stroke::new(1.0, Color32::BLACK)),
                        );
                    }
                });

                let plot_rect = plot_response.response.rect;
                let p = crate::image_viewer::ui::theme::tokens::palette(ui.ctx());

                if let Some(pos) = *cursor_pos.borrow() {
                    let color = *color_data.borrow();
                    let coord_text = if let Some(c) = color {
                        format!(
                            "RGBA: #{:02X}{:02X}{:02X}{:02X}  ({:.0}, {:.0})",
                            c.r(),
                            c.g(),
                            c.b(),
                            c.a(),
                            pos[0].floor(),
                            -pos[1].floor() - 1.0
                        )
                    } else {
                        format!("({:.0}, {:.0})", pos[0].floor(), -pos[1].floor() - 1.0)
                    };
                    let galley = ui.painter().layout_no_wrap(
                        coord_text,
                        egui::FontId::monospace(11.0),
                        p.subtext0,
                    );
                    let pad = egui::vec2(8.0, 4.0);
                    let coord_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            plot_rect.left() + 8.0,
                            plot_rect.bottom() - galley.size().y - pad.y - 8.0,
                        ),
                        galley.size() + pad * 2.0,
                    );
                    ui.painter().rect(
                        coord_rect,
                        egui::CornerRadius::same(4),
                        p.mantle,
                        egui::Stroke::new(1.0, p.surface1),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().galley(
                        coord_rect.center() - 0.5 * galley.size(),
                        galley,
                        p.subtext0,
                    );
                }

                if let Some(badge_text) = &self.badge {
                    let galley = ui.painter().layout_no_wrap(
                        badge_text.clone(),
                        egui::FontId::monospace(11.0),
                        p.overlay0,
                    );
                    let pad = egui::vec2(10.0, 4.0);
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            plot_rect.right() - galley.size().x - pad.x - 8.0,
                            plot_rect.top() + 8.0,
                        ),
                        galley.size() + pad * 2.0,
                    );
                    ui.painter().rect(
                        badge_rect,
                        egui::CornerRadius::same(4),
                        p.mantle,
                        egui::Stroke::new(1.0, p.surface1),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().galley(
                        badge_rect.center() - 0.5 * galley.size(),
                        galley,
                        p.overlay0,
                    );
                }

                if let Some(on_hover) = &mut self.on_hover {
                    if let Some(pos) = *cursor_pos.borrow() {
                        **on_hover = Some([pos[0].floor() as u32, (-pos[1].floor() - 1.0) as u32]);
                    } else {
                        **on_hover = None;
                    }
                }
            }
        }
    }
}
