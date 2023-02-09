extern crate font_loader as fonts;

use std::{f32::consts::{PI, TAU}, cmp::min};
use image::{GenericImageView, GrayImage, Luma, ImageBuffer};
use imageproc::{drawing::{draw_text_mut, text_size}, geometric_transformations::{warp_with, rotate_about_center, Interpolation, Projection}};
use rusttype::{Font, Scale};
use fonts::system_fonts;

pub struct Watermarker {
    text: String,
    font: Font<'static>,
    size: u32,
    /// Period of vertical oscillations in x, as fraction of total width
    period_x: f32,
    /// Amplitude of vertical oscillations in y, in pixels
    amplitude_y: f32,
    /// Rotation of general watermark in degrees
    rotation: i32,
    /// Interplation mode for operations
    interpolation: Interpolation
}

impl Watermarker {
    pub fn new(text: &str) -> Watermarker {
        let fp = system_fonts::FontPropertyBuilder::new().family("DejaVu").bold().build();
        let (bytes, index) = system_fonts::get(&fp).unwrap();
        let font = Font::try_from_vec_and_index(bytes, index as u32).unwrap();
        Watermarker {
            text: text.to_string(),
            font: font,
            size: 16,
            period_x: 0.2,
            amplitude_y: 6.0,
            rotation: 10,
            interpolation: Interpolation::Bicubic
        }
    }
    pub fn watermark(&self, img: impl GenericImageView) {
        info!("Found image with dimensions {:?}", img.dimensions());

        let (w, h) = img.dimensions();
        info!("Creating watermark");
        let buf = self.create_watermark((w, h));

        buf.save("buf.jpg");
    }

    fn create_watermark(&self, (w, h): (u32, u32)) -> GrayImage {
        // We'll want to rotate our buffer eventually.
        // So we need to make sure that the buffer, once rotated, will be able to cover the original image.

        let rot_rad = PI * (self.rotation as f32) / 180.0;

        // Find the center
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let rotation = Projection::translate(cx, cy) * Projection::rotate(rot_rad) * Projection::translate(-cx, -cy);

        // Rotation would keep the top-left as-is
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for x in [0.0, w as f32] {
            for y in [0.0, h as f32] {
                let point = rotation * (x, y);
                min_x = min_x.min(point.0);
                min_y = min_y.min(point.1);
                max_x = max_x.max(point.0);
                max_y = max_y.max(point.1);
            }
        }
        let (buf_w, buf_h) = ((max_x - min_x) as u32, (max_y - min_y) as u32);
        info!("buf_w={}, buf_h={}", buf_w, buf_h);
        let buf = self.create_text_pattern((buf_w, buf_h), Luma([255]));

        let w_x = TAU / ((w as f32) * self.period_x);
        let buf = self.warp(&buf, self.amplitude_y, w_x);

        // Now rotate
        let mut buf = rotate_about_center(&buf, rot_rad, self.interpolation, Luma([0]));
        // Now crop to get the right size
        let x = (buf_w/2 - w/2);
        let y = (buf_h/2 - h/2);
        let buf = image::imageops::crop(&mut buf, x, y, w, h).to_image();
        buf
    }

    fn create_text_pattern(&self, (w, h): (u32, u32), color: Luma<u8>) -> GrayImage
    {
        let mut buf: GrayImage = ImageBuffer::new(w, h);

        let scale = Scale::uniform(self.size as f32);
        let (text_w, text_h) = text_size(scale, &self.font, &self.text);
        info!("text_w={}, text_h={}", text_w, text_h);

        // Shift between two texts on two different lines
        let vert_shift = f32::round((text_h as f32) * 1.1) as u32;
        // Shift between two texts on the same line
        let horizontal_shift = f32::round((text_w as f32) * 1.1) as u32;
        // How much we shift each line
        let line_shift = -text_w / 10;

        let mut line = 0;
        let mut start_x = 0;

        while line * vert_shift < h {
            let start_y: i32 = (line * vert_shift) as i32;
            start_x = (line as i32) * line_shift;
            while start_x < (w as i32) {
                draw_text_mut( &mut buf, color, start_x, start_y as i32, scale, &self.font, &self.text);
                start_x += horizontal_shift as i32;
            }
            line += 1;
        }
        buf
    }

    fn warp(&self, buf: &GrayImage, scale_y: f32, w_x: f32) -> GrayImage {
        let (w, h) = buf.dimensions();

        warp_with(
            buf,
            |x, y| (x, y + 0.5 * scale_y * (w_x * x).sin()),
            self.interpolation,
            Luma([0u8])
        )
    }


}