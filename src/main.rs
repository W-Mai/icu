#![allow(
    clippy::bind_instead_of_map,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::field_reassign_with_default,
    clippy::if_same_then_else,
    clippy::map_entry,
    clippy::map_identity,
    clippy::needless_lifetimes,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    clippy::unnecessary_to_owned,
    clippy::unused_enumerate_index,
    clippy::upper_case_acronyms
)]

#[cfg(not(target_arch = "wasm32"))]
mod arguments;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
pub mod converter;
mod cus_component;
mod image_viewer;
mod utils;

#[macro_use]
extern crate rust_i18n;

i18n!("locales");

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    image_viewer::show_image(vec![], converter::ImageFormatCategory::Auto);
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let res = cli::process();

    if let Err(e) = res {
        log::error!("{e}");
        std::process::exit(1);
    }
}
