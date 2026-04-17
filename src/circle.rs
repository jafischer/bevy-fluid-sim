use bevy::prelude::Vec2;
use image::ImageResult;

/// This is a function I wrote because I couldn't figure out how to do this in the world's most intuitive software, GIMP.
/// It generates an image of a circle that is blurred (transparent) at the edge.
/// Pixels within the circle will be assigned a white color, and an alpha value of
///  1 - d ^ exponent
///  where d is the distance from the center, scaled to [0..1].
pub fn make_blurred_circle(exponent: f32) -> ImageResult<()> {
    const CIRCLE_RADIUS: f32 = 32.0;
    let mut imgbuf = image::ImageBuffer::new(CIRCLE_RADIUS as u32 * 2, CIRCLE_RADIUS as u32 * 2);
    let center = Vec2::new(CIRCLE_RADIUS, CIRCLE_RADIUS);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let distance = (Vec2::new(x as f32, y as f32) - center).length();
        if distance < CIRCLE_RADIUS {
            let alpha = 1.0 - (distance / CIRCLE_RADIUS).powf(exponent);
            *pixel = image::Rgba([255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    imgbuf.save(&format!("assets/blurred-circle-pow-{:.1}.png", exponent))
}
