#[macro_use]
extern crate log;
mod watermarker;

fn main() {
    env_logger::init();

    let img = image::open("tests/passport.jpg").unwrap();

    let wm = watermarker::Watermarker::new(
        "Tenancy application - 12/12/2022"
    );
    wm.watermark(img);
}
