use image::{GenericImage, GenericImageView};

pub fn blend_with_fn<I, V, F>(a: &mut I, b: &V, f: F) -> Result<(), String> 
where
I: GenericImage,
V: GenericImageView,
F: Fn(u32, u32, I::Pixel, V::Pixel) -> I::Pixel
{
    if a.dimensions() != b.dimensions() {
        return Err("Dimensions of images should match".into());
    }

    // TODO: Rework this. We could replace with ImageBuffers, then zip two enumerators
    // on the pixels, allowing parallel.
    let (w, h) = a.dimensions();
    for y in 0..h {
        for x in 0..w {
            let p = f(x, y, a.get_pixel(x, y), b.get_pixel(x, y));
            a.put_pixel(x, y, p);
        }
    }

    Ok(())
}