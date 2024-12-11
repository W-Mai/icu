use crate::image_shower::ImageItem;
use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, PointerButton};
use egui_plot::{CoordinatesFormatter, Corner, PlotImage, PlotPoint};
use rand::random;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ImagePlotter {
    anti_alias: bool,
    show_grid: bool,
    show_only: bool,
}

impl ImagePlotter {
    pub fn new() -> ImagePlotter {
        Self {
            anti_alias: false,
            show_grid: false,
            show_only: false,
        }
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

    pub fn show(&mut self, ui: &mut egui::Ui, image_item: &Option<ImageItem>) {
        let color_data: Rc<RefCell<Option<Color32>>> = Default::default();
        let cursor_pos: Rc<RefCell<Option<[f64; 2]>>> = Default::default();

        match image_item {
            None => {}
            Some(image_data) => {
                let width = image_data.width as f32;
                let height = image_data.height as f32;

                let image = ColorImage {
                    size: [width as usize, height as usize],
                    pixels: image_data.image_data.clone(),
                };
                let texture = ui.ctx().load_texture(
                    format!("showing_image_{}", random::<u32>()),
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
                let copy_image_data = image_data.image_data.clone();

                let color_data_1 = color_data.clone();
                let color_data_2 = color_data.clone();
                let cursor_pos_2 = cursor_pos.clone();

                let plot = egui_plot::Plot::new(format!("plot{}", random::<u32>()))
                    .data_aspect(1.0)
                    .y_axis_formatter(move |y, _, _| format!("{:.0}", -y.value))
                    .coordinates_formatter(
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
                    .label_formatter(move |_text, pos| {
                        if pos.x > 0.0 && pos.x < img_w && pos.y < 0.0 && pos.y > -img_h {
                            let row = -pos.y as usize;
                            let col = pos.x as usize;
                            let index = row * img_w as usize + col;
                            let pixel = copy_image_data[index];
                            color_data_2.borrow_mut().replace(pixel);
                            cursor_pos_2.borrow_mut().replace([pos.x, pos.y]);

                            format!("Pos: {:.2} {:.2}", pos.x, pos.y)
                        } else {
                            color_data_2.take();
                            cursor_pos_2.take();
                            "".into()
                        }
                    })
                    .boxed_zoom_pointer_button(PointerButton::Extra2)
                    .show_grid([self.show_grid, self.show_grid])
                    .clamp_grid(true)
                    .sharp_grid_lines(false)
                    .show_axes([!self.show_only, !self.show_only])
                    .allow_scroll(!self.show_only)
                    .allow_zoom(!self.show_only)
                    .allow_drag(!self.show_only)
                    .show_x(!self.show_only)
                    .show_y(!self.show_only);

                plot.show(ui, |plot_ui| {
                    plot_ui.image(PlotImage::new(
                        texture.id,
                        PlotPoint::new(img_w / 2.0, -img_h / 2.0),
                        texture.size,
                    ));

                    let plot_bounds = plot_ui.plot_bounds();
                    let plot_size = plot_ui.response().rect;
                    let scale = 1.0 / (plot_bounds.width() as f32 / plot_size.width());

                    let color_data = *color_data.clone().borrow();
                    let cursor_pos = *cursor_pos.clone().borrow();

                    if let Some(pos) = cursor_pos {
                        if let Some(pixel) = color_data {
                            let pos = [pos[0].floor() + 0.5, pos[1].floor() + 0.5];

                            plot_ui.points(
                                egui_plot::Points::new(vec![pos])
                                    .shape(egui_plot::MarkerShape::Square)
                                    .radius(scale)
                                    .color(pixel),
                            );
                        }
                    }
                });
            }
        }
    }
}
