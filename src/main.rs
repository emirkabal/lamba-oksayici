use std::{io::ErrorKind, time::Duration};
use std::collections::HashMap;
use scrap::{Capturer, Display};
use image::{DynamicImage, Rgb};
use log::{LevelFilter, info};
use tapo::ApiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_timed_builder()
    .filter_level(LevelFilter::Info)
    .init();

    info!("Starting...");

    let tapo_username = "****";
    let tapo_password = "****";
    let ip_address = "192.168.1.108";

    let device = ApiClient::new(tapo_username, tapo_password)
        .l530(ip_address)
        .await?;

    info!("Turning device on...");
    device.on().await?;


    let display = Display::primary()?;
    let mut capturer = Capturer::new(display)?;

    tokio::time::sleep(Duration::from_millis(100)).await;
    loop {
        let width = capturer.width() as u32;
        let height = capturer.height() as u32;

        let buffer = loop {
            match capturer.frame() {
                Ok(buffer) => break buffer,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        };

        let img = DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(width, height, buffer.to_vec())
                .ok_or("Görüntü oluşturulamadı")?,
        );

        let rgb = dominant_color(&img);
        let (hue, saturation, brightness) = rgb_to_hsv(rgb[0], rgb[1], rgb[2]);

        device
            .set()
            .hue_saturation(hue, saturation)
            .brightness(brightness)
            .send(&device)
            .await?;
        info!(
            "Lamba güncellendi: Hue={}, Saturation={}, Brightness={}, RGB=({}, {}, {})",
            hue, saturation, brightness, rgb[0], rgb[1], rgb[2]
        );

        // tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

fn dominant_color(img: &DynamicImage) -> Rgb<u8> {
    let rgb_img = img.to_rgb8();
    let mut color_counts: HashMap<(u8, u8, u8), u32> = HashMap::new();

    for pixel in rgb_img.pixels() {
        let r = (pixel[0] >> 4) << 4;
        let g = (pixel[1] >> 4) << 4;
        let b = (pixel[2] >> 4) << 4;
        *color_counts.entry((r, g, b)).or_insert(0) += 1;
    }

    let dominant = color_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|((r, g, b), _)| Rgb([r, g, b]))
        .unwrap_or(Rgb([0, 0, 0]));

    dominant
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    let saturation = if max == 0.0 {
        0.0
    } else {
        (delta / max) * 100.0
    };

    let brightness = max * 100.0;

    (
        hue.round() as u16,
        saturation.round().max(1.0).min(100.0) as u8,
        brightness.round().max(1.0).min(100.0) as u8,
    )
}