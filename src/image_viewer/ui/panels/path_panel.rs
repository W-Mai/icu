use crate::image_viewer::plotter::ImagePlotter;
use eframe::egui;
use icu_lib::midata::MiData;

pub fn draw_path_panel(ui: &mut egui::Ui, state: &mut crate::image_viewer::model::ViewerState) {
    let Some(image) = state.current_image.clone() else {
        return;
    };
    let Some(MiData::PATH(scene_data)) = &image.midata else {
        return;
    };
    let scene_data = scene_data.clone();

    egui::Panel::left("path_left").show(ui, |ui| {
        ui.heading("Scene");
        ui.label(format!("ops: {}", scene_data.scene.ops.len()));
        ui.separator();
        if ui.button("Export PNG").clicked() {
            let (w, h) =
                icu_lib::endecoder::mirui::scene_render::scene_dimensions(&scene_data.scene)
                    .unwrap_or((256, 256));
            let img =
                icu_lib::endecoder::mirui::scene_render::render_scene(&scene_data.scene, w, h);
            if let Some(path) = super::pick_save_file(&[("PNG", &["png"])], "scene.png") {
                let _ = img.save(&path);
            }
        }
        if ui.button("Export SVG").clicked() {
            let svg = icu_lib::endecoder::svg::export::scene_to_svg(&scene_data.scene, 0, 0);
            if let Some(path) = super::pick_save_file(&[("SVG", &["svg"])], &"scene.svg") {
                let _ = std::fs::write(&path, svg);
            }
        }
        if ui.button("Export mirx").clicked() {
            let payload = scene_data.scene.encode().unwrap_or_default();
            let bytes = icu_lib::mirx::encode_chunk_generic(
                icu_lib::mirx::chunk_type::VECTOR,
                icu_lib::mirx::ChunkEntry::FLAG_CRITICAL,
                &payload,
            );
            if let Some(path) = super::pick_save_file(&[("mirx", &["mirx"])], &"scene.mirx") {
                let _ = std::fs::write(&path, bytes);
            }
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, op) in scene_data.scene.ops.iter().enumerate() {
                let label = op_label(op);
                if ui
                    .selectable_label(
                        state.selected_op == Some(i),
                        format!("{}. {}", i, label),
                    )
                    .clicked()
                {
                    state.selected_op = Some(i);
                }
            }
        });
    });

    egui::Panel::right("path_right").show(ui, |ui| {
        if let Some(idx) = state.selected_op {
            if let Some(op) = scene_data.scene.ops.get(idx) {
                ui.heading(format!("Op #{}: {}", idx, op_label(op)));
                ui.separator();
                op_inspector(ui, op);
            }
        }
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.horizontal(|ui| {
            crate::image_viewer::ui::widgets::mode_tabs(
                ui,
                &mut state.path_mode,
                &[(crate::image_viewer::model::PathMode::Preview, "Preview")],
            );
        });
        ui.separator();

        let highlight = if let Some(idx) = state.selected_op {
            if let Some(op) = scene_data.scene.ops.get(idx) {
                op_center(op)
            } else {
                None
            }
        } else {
            None
        };
        let mut plotter = ImagePlotter::new("path_preview")
            .anti_alias(state.context.anti_alias)
            .show_grid(state.context.show_grid)
            .highlight(highlight);
        plotter.show(ui, &Some(image.clone()));
    });
}

fn op_center(op: &icu_lib::mirx::SceneOp) -> Option<[u32; 2]> {
    use icu_lib::mirx::SceneOp;
    match op {
        SceneOp::FillPath { path, .. } | SceneOp::StrokePath { path, .. } => {
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for cmd in &path.cmds {
                let p = match cmd {
                    icu_lib::mirx::PathCmd::MoveTo(p) | icu_lib::mirx::PathCmd::LineTo(p) => *p,
                    icu_lib::mirx::PathCmd::QuadTo { end, .. } => *end,
                    icu_lib::mirx::PathCmd::CubicTo { end, .. } => *end,
                    icu_lib::mirx::PathCmd::Close => continue,
                };
                let x = p.x.to_int();
                let y = p.y.to_int();
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            if min_x <= max_x && min_y <= max_y {
                Some([((min_x + max_x) / 2) as u32, ((min_y + max_y) / 2) as u32])
            } else {
                None
            }
        }
        SceneOp::FillRect { area, .. } | SceneOp::Border { area, .. } => {
            let cx = (area.x.to_int() + area.w.to_int()) / 2;
            let cy = (area.y.to_int() + area.h.to_int()) / 2;
            Some([cx as u32, cy as u32])
        }
        SceneOp::Line { p1, p2, .. } => {
            let cx = (p1.x.to_int() + p2.x.to_int()) / 2;
            let cy = (p1.y.to_int() + p2.y.to_int()) / 2;
            Some([cx as u32, cy as u32])
        }
        SceneOp::Arc { center, .. } => Some([center.x.to_int() as u32, center.y.to_int() as u32]),
        _ => None,
    }
}

fn op_label(op: &icu_lib::mirx::SceneOp) -> &'static str {
    match op {
        icu_lib::mirx::SceneOp::GroupBegin { .. } => "GroupBegin",
        icu_lib::mirx::SceneOp::GroupEnd => "GroupEnd",
        icu_lib::mirx::SceneOp::FillPath { .. } => "FillPath",
        icu_lib::mirx::SceneOp::StrokePath { .. } => "StrokePath",
        icu_lib::mirx::SceneOp::FillRect { .. } => "FillRect",
        icu_lib::mirx::SceneOp::Border { .. } => "Border",
        icu_lib::mirx::SceneOp::Line { .. } => "Line",
        icu_lib::mirx::SceneOp::Arc { .. } => "Arc",
        icu_lib::mirx::SceneOp::Label { .. } => "Label",
        icu_lib::mirx::SceneOp::Blit { .. } => "Blit",
        icu_lib::mirx::SceneOp::PushClip { .. } => "PushClip",
        icu_lib::mirx::SceneOp::PopClip => "PopClip",
    }
}

fn op_inspector(ui: &mut egui::Ui, op: &icu_lib::mirx::SceneOp) {
    use icu_lib::mirx::SceneOp;
    match op {
        SceneOp::FillPath {
            paint,
            opa,
            fill_rule,
            ..
        } => {
            ui.label(format!("paint: {:?}", paint));
            ui.label(format!("opa: {}", opa));
            ui.label(format!("fill_rule: {:?}", fill_rule));
        }
        SceneOp::StrokePath {
            paint,
            width,
            opa,
            line_cap,
            line_join,
            miter_limit,
            dash,
            ..
        } => {
            ui.label(format!("paint: {:?}", paint));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
            ui.label(format!("cap: {:?}", line_cap));
            ui.label(format!("join: {:?}", line_join));
            ui.label(format!("miter_limit: {}", miter_limit.to_f32()));
            if !dash.is_empty() {
                let s: Vec<String> = dash.iter().map(|d| d.to_f32().to_string()).collect();
                ui.label(format!("dash: [{}]", s.join(", ")));
            }
        }
        SceneOp::FillRect {
            area,
            color,
            radius,
            opa,
            ..
        } => {
            ui.label(format!(
                "area: ({},{},{},{})",
                area.x.to_f32(),
                area.y.to_f32(),
                area.w.to_f32(),
                area.h.to_f32()
            ));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Border {
            area,
            color,
            width,
            radius,
            opa,
            ..
        } => {
            ui.label(format!(
                "area: ({},{},{},{})",
                area.x.to_f32(),
                area.y.to_f32(),
                area.w.to_f32(),
                area.h.to_f32()
            ));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Line {
            p1,
            p2,
            color,
            width,
            opa,
            ..
        } => {
            ui.label(format!("p1: ({},{})", p1.x.to_f32(), p1.y.to_f32()));
            ui.label(format!("p2: ({},{})", p2.x.to_f32(), p2.y.to_f32()));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            color,
            width,
            opa,
            ..
        } => {
            ui.label(format!(
                "center: ({},{})",
                center.x.to_f32(),
                center.y.to_f32()
            ));
            ui.label(format!("radius: {}", radius.to_f32()));
            ui.label(format!(
                "angles: {}° - {}°",
                start_angle.to_f32(),
                end_angle.to_f32()
            ));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("width: {}", width.to_f32()));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::GroupBegin {
            transform, opacity, ..
        } => {
            if let Some(t) = transform {
                ui.label(format!(
                    "transform: [{},{},{}/{},{},{}]",
                    t.m00.to_f32(),
                    t.m01.to_f32(),
                    t.tx.to_f32(),
                    t.m10.to_f32(),
                    t.m11.to_f32(),
                    t.ty.to_f32()
                ));
            } else {
                ui.label("transform: identity");
            }
            ui.label(format!("opacity: {:?}", opacity));
        }
        SceneOp::Label {
            text, color, opa, ..
        } => {
            ui.label(format!("text: {:?}", text));
            ui.label(format!("color: {:?}", color));
            ui.label(format!("opa: {}", opa));
        }
        SceneOp::PushClip { fill_rule, .. } => {
            ui.label(format!("fill_rule: {:?}", fill_rule));
        }
        _ => {}
    }
}
