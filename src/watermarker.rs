extern crate font_loader as fonts;

use std::{f32::consts::{PI, TAU}};
use image::{Pixel, GrayImage, Luma, Rgba, ImageBuffer, RgbImage, GenericImageView};
use imageproc::{drawing::{draw_text_mut, text_size}, geometric_transformations::{warp_with, rotate_about_center, Interpolation, Projection}};
use rusttype::{Font, Scale};
use fonts::system_fonts;
use super::blender;


pub struct Watermarker {
    text: String,
    font: Font<'static>,
    size: u32,
    /// Period of vertical oscillations in x, as fraction of total width
    warp_period_x: f32,
    /// Period of horizontal oscillations in y, as fraction of total width
    warp_period_y: f32,
    /// Amplitude of horizontal oscillations in x, in pixels
    warp_amplitude_x: f32,
    /// Amplitude of vertical oscillations in y, in pixels
    warp_amplitude_y: f32,
    /// Period of blending on the x axis, as fraction of total_width
    blend_period_x: f32,
    /// Period of blending on the y axis, as fraction of total_width
    blend_period_y: f32,
    /// Rotation of general watermark in degrees
    rotation: i32,
    /// Interplation mode for operations
    interpolation: Interpolation,
    /// Blend ratio: Between 0.0 and 1.0. 0.0 won't have any watermark, 1.0 will be full watermark
    blend_ratio: f32,
}

impl Watermarker {
    pub fn new(text: &str) -> Watermarker {
        let fp = system_fonts::FontPropertyBuilder::new().family("Arial").bold().build();
        let (bytes, index) = system_fonts::get(&fp).unwrap();
        let font = Font::try_from_vec_and_index(bytes, index as u32).unwrap();
        Watermarker {
            text: text.to_string(),
            font: font,
            size: 16,
            warp_period_x: 0.35,
            warp_period_y: 0.15,
            warp_amplitude_x: 7.0,
            warp_amplitude_y: 5.0,
            blend_period_x: 0.4,
            blend_period_y: 0.3,
            rotation: 10,
            interpolation: Interpolation::Bicubic,
            blend_ratio: 0.3,
        }
    }

    pub fn blend_max(&self, img: &mut RgbImage, wm: &GrayImage) {
        blender::blend_with_fn(img, wm, |x, y, a, b| {
            a.map(|u| b.0[0].max(u))
        }).unwrap();
    }
    pub fn blend_min_invert(&self, img: &mut RgbImage, wm: &GrayImage) {
        blender::blend_with_fn(img, wm, |x, y, a, b| {
            let mut b = b.clone();
            b.invert();
            a.map(|u| b.0[0].min(u))
        }).unwrap();
    }

    pub fn watermark(&self, img: &mut RgbImage)
    {
        let (w, h) = img.dimensions();
        let wm = self.create_watermark((w, h));

        let mut with_max_text = img.clone();
        self.blend_max(&mut with_max_text, &wm);

        let mut with_min_invert_text = img.clone();
        self.blend_min_invert(&mut with_min_invert_text, &wm);
        self.blend_min_invert(img, &wm);

        return;
        
        let blend_alpha = (self.blend_ratio * 255.0).clamp(0.0, 255.0) as u8;

        let w_x = TAU / ((w as f32) * self.blend_period_x);
        let w_y = TAU / ((h as f32) * self.blend_period_y);
        blender::blend_with_fn(img, &wm, |x, y, a, b| {
            // We'll blend by using the watermark value (normalized to 1) as the absolute value of change,
            // and multiplying with a sinusoid to add/remove from current value.
            if x < 50 { return a; }
            
            // We'll look at the luminosity of this.
            let Luma([p]) = b;

            let xp = (w_x * (x as f32)).sin();
            let yp = (w_y * (y as f32)).sin();

            // Base brightness. Will then oscillate between this and 1.0
            let low_color_base = 0.4;
            let high_color_base = 1.0;

            // Lum_change rescaled to 0..1
            //let lum = (p as f32 / 255.0);
            let lum_change = xp * yp * 0.5 + 0.5;
            let lum_change = low_color_base + (high_color_base - low_color_base) * lum_change;
            let new_color = (p as f32 * lum_change).clamp(0.0, 255.0) as u8;

            //let blend_base = 0.5;
            //let blend_amplitude = 0.5;
            //let blend_alpha = blend_base + blend_amplitude * xp * yp;
            //let blend_alpha = (blend_alpha * 255.0).clamp(0.0, 255.0) as u8;

            // TODO: Instead of blending on img and watermark
            // create watermark on copy of img, then blend between both

            let blend_alpha = 0.3 + lum_change * 0.3;
            let blend_alpha = (blend_alpha * 255.0).clamp(0.0, 255.0) as u8;
            let m = Rgba([p, p, p, blend_alpha]);
            //let m = Rgba([new_color, new_color, new_color, blend_alpha]);

            let mut out = a.to_rgba();
            out.blend(&m);
            out.to_rgb()

        }).unwrap();
    }

    pub fn create_watermark(&self, (w, h): (u32, u32)) -> GrayImage {
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
        let buf = self.create_text_pattern((buf_w, buf_h), Luma([0]), Luma([255]));

        let w_x = TAU / ((w as f32) * self.warp_period_x);
        let w_y = TAU / ((h as f32) * self.warp_period_y);
        let buf = self.warp(&buf, self.warp_amplitude_x, self.warp_amplitude_y, w_x, w_y);

        // Now rotate
        let mut buf = rotate_about_center(&buf, rot_rad, self.interpolation, Luma([0]));
        // Now crop to get the right size
        let x = buf_w/2 - w/2;
        let y = buf_h/2 - h/2;
        let buf = image::imageops::crop(&mut buf, x, y, w, h).to_image();
        buf
    }

    fn create_text_pattern(&self, (w, h): (u32, u32), bg: Luma<u8>, fg: Luma<u8>) -> GrayImage
    {
        let mut buf: GrayImage = ImageBuffer::new(w, h);
        buf.fill(bg.0[0]);

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
        let mut start_x;

        while line * vert_shift < h {
            let start_y: i32 = (line * vert_shift) as i32;
            start_x = (line as i32) * line_shift;
            while start_x < (w as i32) {
                draw_text_mut( &mut buf, fg, start_x, start_y as i32, scale, &self.font, &self.text);
                start_x += horizontal_shift as i32;
            }
            line += 1;
        }
        buf
    }

    fn warp(&self, buf: &GrayImage, scale_x: f32, scale_y: f32, w_x: f32, w_y: f32) -> GrayImage {
        warp_with(
            buf,
            |x, y| {
                let xp = (w_x * x).sin();
                let yp = (w_y * y).sin();
                (x + 0.5 * scale_x * xp * yp, y + 0.5 * scale_y * xp * yp)
            },
            self.interpolation,
            Luma([0u8])
        )
    }


}