pub mod font_panel;
pub mod indexed_panel;
pub mod path_panel;

pub use font_panel::{draw_font_panel, draw_glyph_panel};
pub use indexed_panel::draw_indexed_panel;
pub use path_panel::draw_path_panel;

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
