#[cfg(not(target_arch = "wasm32"))]
mod arguments;
#[cfg(not(target_arch = "wasm32"))]
mod cli;
mod image_plotter;
mod image_shower;
mod utils;

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    image_shower::show_image(vec![]);
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let res = cli::process();

    if let Err(e) = res {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
