#[macro_use]
extern crate log;
mod watermarker;

mod blender;

fn main() {
    env_logger::init();

    let mut img = image::open("tests/passport.jpg").unwrap().into_rgb8();
    let dims = img.dimensions();
    info!("Found image with dimensions {:?}", dims);

    let wm = watermarker::Watermarker::new(
        "Tenancy application - 12/12/2022"
    );

    info!("Creating watermark");
    wm.watermark(&mut img);
    img.save("buf.jpg").unwrap();
}
