use crate::image_shower::ImageItem;
use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, PointerButton};
use egui_plot::{CoordinatesFormatter, Corner, PlotImage, PlotPoint};
use std::cell::RefCell;
use std::rc::Rc;

pub struct ImagePlotter {
    id: String,
    anti_alias: bool,
    show_grid: bool,
    show_only: bool,
    background_color: Color32,
    highlight_pixel: Option<[u32; 2]>,
}

impl ImagePlotter {
    pub fn new(id: impl ToString) -> ImagePlotter {
        Self {
            id: id.to_string(),
            anti_alias: false,
            show_grid: false,
            show_only: false,
            background_color: Default::default(),
            highlight_pixel: None,
        }
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
            Some(image_data) => {
                let width = image_data.width as f32;
                let height = image_data.height as f32;

                let image = ColorImage::new(
                    [width as usize, height as usize],
                    image_data.image_data.clone(),
                );
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
                let copy_image_data = Rc::new(RefCell::new(image_data.image_data.clone()));

                let copy_image_data_1 = copy_image_data.clone();
                let color_data_1 = color_data.clone();
                let color_data_2 = color_data.clone();
                let cursor_pos_2 = cursor_pos.clone();

                let mut plot = egui_plot::Plot::new(format!("plot{}", self.id))
                    .data_aspect(1.0)
                    .y_axis_formatter(move |y, _| format!("{:.0}", -y.value))
                    .label_formatter(move |_text, pos| {
                        if pos.x > 0.0 && pos.x < img_w && pos.y < 0.0 && pos.y > -img_h {
                            let row = -pos.y as usize;
                            let col = pos.x as usize;
                            let index = row * img_w as usize + col;
                            let pixel = &copy_image_data_1.borrow()[index];
                            color_data_2.borrow_mut().replace(*pixel);
                            cursor_pos_2.borrow_mut().replace([pos.x, pos.y]);

                            format!("Pos: {:.0} {:.0}", pos.x.floor(), -pos.y.floor() - 1.0)
                        } else {
                            color_data_2.take();
                            cursor_pos_2.take();
                            "".into()
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

                if let Some([x, y]) = self.highlight_pixel {
                    plot = plot.include_x(x as f64 + 0.5).include_y(-(y as f64 + 0.5));
                }

                if !self.show_only {
                    plot = plot.coordinates_formatter(
                        Corner::LeftBottom,
                        CoordinatesFormatter::new(|p, _b| {
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

                plot.show(ui, |plot_ui| {
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
                        let center = [x as f64 + 0.5, -(y as f64 + 0.5)];
                        let alpha = (time * 5.0).sin().abs() as f32;
                        let color = Color32::RED.linear_multiply(alpha);
                        let stroke_width = if scale < 1.0 { 2.0 } else { 2.0 / scale };

                        plot_ui.polygon(
                            egui_plot::Polygon::new(
                                "highlight",
                                vec![
                                    [
                                        center[0] - 0.5 * scale_fact * scale_fact,
                                        center[1] - 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        center[0] + 0.5 * scale_fact * scale_fact,
                                        center[1] - 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        center[0] + 0.5 * scale_fact * scale_fact,
                                        center[1] + 0.5 * scale_fact * scale_fact,
                                    ],
                                    [
                                        center[0] - 0.5 * scale_fact * scale_fact,
                                        center[1] + 0.5 * scale_fact * scale_fact,
                                    ],
                                ],
                            )
                            .fill_color(Color32::TRANSPARENT)
                            .stroke(egui::Stroke::new(stroke_width, color)),
                        );
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
            }
        }
    }
}
