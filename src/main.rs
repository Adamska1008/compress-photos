use std::str::FromStr;

use clap::{arg, command, value_parser, Command};
use image::{GenericImageView, ImageReader};
use log::info;
use simple_logger::SimpleLogger;

// The upper limit of the width and height of the image
#[derive(Debug, Clone, Copy)]
struct Bound(Option<u32>, Option<u32>);

impl FromStr for Bound {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(',').collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err("Invalid bound format".to_string());
        }
        let w = parts[0].parse::<u32>().ok();
        let h = parts[1].parse::<u32>().ok();
        Ok(Bound(w, h))
    }
}

fn compress_with_bound(img: &image::DynamicImage, bound: &Bound) -> image::DynamicImage {
    let (w, h) = img.dimensions();
    let w = if let Some(w) = bound.0 { w } else { w };
    let h = if let Some(h) = bound.1 { h } else { h };
    img.resize(w, h, image::imageops::FilterType::CatmullRom)
}

enum Strategy {
    Bound(Bound),
}

impl Strategy {
    fn apply(&self, img: &image::DynamicImage) -> image::DynamicImage {
        match self {
            Strategy::Bound(bound) => compress_with_bound(img, bound),
        }
    }
}

fn compress_one(filename: &str, strategy: &Strategy, quality: u8) -> Result<(), image::ImageError> {
    let img = ImageReader::open(filename)?.decode()?;
    info!("Compressing {} size: {:?}", filename, img.dimensions());
    let resized = strategy.apply(&img);
    // If dir not exists, create it
    if !std::path::Path::new("./compacted").exists() {
        std::fs::create_dir("./compacted")?;
    }
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        std::fs::File::create(format!("./compacted/{}", filename))?,
        quality,
    );
    encoder.encode_image(&resized)?;
    info!("Compressed {} to size {:?}", filename, resized.dimensions());
    Ok(())
}

fn cli() -> Command {
    command!()
        .arg(arg!([filename] "Specify the filename to compress"))
        .arg(
            arg!(-b --bound <BOUND> "Specify the bound of image")
                .value_parser(Bound::from_str)
                .default_value("1600,1600"),
        )
        .arg(
            arg!(-q --quality <QUALITY> "Specify the quality")
                .value_parser(value_parser!(u8))
                .default_value("75"),
        )
}

fn init_logger() {
    SimpleLogger::new().init().unwrap();
}

fn main() -> Result<(), image::ImageError> {
    init_logger();
    let matches = cli().get_matches();
    let bound = matches.get_one::<Bound>("bound").unwrap();
    let quality = matches.get_one::<u8>("quality").unwrap();
    if let Some(filename) = matches.get_one::<String>("filename") {
        compress_one(filename, &Strategy::Bound(*bound), *quality)?;
    } else {
        // compress all file in the dir
        let files = std::fs::read_dir("./")?;
        for file in files {
            let file = file?;
            let path = file.path();
            if path.is_file() && path.extension().unwrap_or_default() == "jpg" {
                compress_one(path.to_str().unwrap(), &Strategy::Bound(*bound), *quality)?;
            }
        }
    }
    Ok(())
}
