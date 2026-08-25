pub mod font_panel;
pub mod indexed_panel;
pub mod path_panel;

#[allow(unused_imports)]
use crate::image_viewer::model::{SidebarItem, ViewerState};
#[allow(unused_imports)]
use icu_lib::midata::MiData;
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn pick_file(filters: &[(&str, &[&str])]) -> Option<std::path::PathBuf> {
    let mut fd = rfd::FileDialog::new();
    for (name, exts) in filters {
        fd = fd.add_filter(*name, exts);
    }
    fd.pick_file()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn pick_file(_filters: &[(&str, &[&str])]) -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn pick_save_file(
    filters: &[(&str, &[&str])],
    file_name: &str,
) -> Option<std::path::PathBuf> {
    let mut fd = rfd::FileDialog::new();
    for (name, exts) in filters {
        fd = fd.add_filter(*name, exts);
    }
    fd.set_file_name(file_name).save_file()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn pick_save_file(
    _filters: &[(&str, &[&str])],
    _file_name: &str,
) -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_current_as_png(state: &ViewerState) {
    let Some(item) = state.selected_item() else {
        return;
    };
    let SidebarItem::Image(img) = item else {
        return;
    };
    let Some(midata) = &img.midata else {
        return;
    };
    match midata {
        MiData::PATH(scene_data) => {
            let (w, h) =
                icu_lib::endecoder::mirui::scene_render::scene_dimensions(&scene_data.scene)
                    .unwrap_or((256, 256));
            let png =
                icu_lib::endecoder::mirui::scene_render::render_scene(&scene_data.scene, w, h);
            if let Some(path) = pick_save_file(&[("PNG", &["png"])], "scene.png") {
                let _ = png.save(&path);
            }
        }
        MiData::INDEXED(indexed) => {
            let png = indexed.rgba.clone();
            if let Some(path) = pick_save_file(&[("PNG", &["png"])], "indexed.png") {
                let _ = png.save(&path);
            }
        }
        _ => {}
    }
}

#[cfg(target_arch = "wasm32")]
pub fn export_current_as_png(_state: &ViewerState) {}
