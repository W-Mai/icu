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
    image_viewer::show_image(vec![]);
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
