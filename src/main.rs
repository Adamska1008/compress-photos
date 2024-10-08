use std::str::FromStr;

use anyhow::{anyhow, Result};
use clap::{arg, command, Command};
use even_bigger_s::S;
use image::{GenericImageView, ImageReader};
use simple_logger::SimpleLogger;
use std::path::PathBuf;

// The upper limit of the width and height of the image
#[derive(Debug, Clone, Copy)]
struct Bound(Option<u32>, Option<u32>);

impl FromStr for Bound {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(',').collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(S!("Invalid bound format"));
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

fn encode_with_quality(
    img: &image::DynamicImage,
    path: impl Into<PathBuf>,
    quality: &str,
    format: image::ImageFormat,
) -> Result<()> {
    let writer = std::fs::File::create(path.into())?;
    match format {
        image::ImageFormat::Jpeg => {
            let quality = match quality {
                "low" => 50,
                "medium" => 75,
                "high" => 90,
                _ => return Err(anyhow!("Invalid quality: {}", quality)),
            };
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
            img.write_with_encoder(encoder)?;
            Ok(())
        }
        image::ImageFormat::Png => {
            use image::codecs::png;
            let compression = match quality {
                "low" => png::CompressionType::Fast,
                "medium" => png::CompressionType::Default,
                "high" => png::CompressionType::Best,
                _ => return Err(anyhow!("Invalid quality: {}", quality)),
            };
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                writer,
                compression,
                png::FilterType::Adaptive,
            );
            img.write_with_encoder(encoder)?;
            Ok(())
        }
        _ => Err(anyhow!("Unsupported format: {:?}", format)),
    }
}

fn cli() -> Command {
    command!()
        .arg(arg!([filename] "Specify the filename to compress.\nIf not specified, all files in the current directory will be compressed"))
        .arg(
            arg!(-b --bound <BOUND> "Specify the upper limit of two dimensions of image.\nAlways keep the aspect ratio of the image.")
                .value_parser(Bound::from_str)
                .default_value("1600,1600"),
        )
        .arg(
            arg!(-q --quality <QUALITY> "")
                .value_parser(["low", "medium", "high"])
                .default_value("high"),
        )
}

fn init_logger() -> Result<()> {
    SimpleLogger::new()
        .init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logger: {}", e))?;
    Ok(())
}

fn main() -> Result<()> {
    init_logger()?;
    const EXT_TO_CHECK: [&str; 3] = ["jpg", "png", "jpeg"];
    let matches = cli().get_matches();
    let bound = matches.get_one::<Bound>("bound").unwrap();
    let quality = matches.get_one::<String>("quality").unwrap();

    // collect all file path
    let mut paths = vec![];
    if let Some(filename) = matches.get_one::<String>("filename") {
        paths.push(PathBuf::from(filename));
    } else {
        // compress all file in the dir
        let files = std::fs::read_dir("./")?;
        for file in files {
            let file = file?;
            if file.path().is_file() {
                paths.push(file.path());
            }
        }
    }

    if !std::path::Path::new("compacted").exists() {
        std::fs::create_dir("compacted")?;
    }

    let compacted_path = PathBuf::from("compacted");

    for path in paths {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        if !EXT_TO_CHECK.contains(&ext) {
            continue;
        }
        let format = if ext == "jpg" || ext == "jpeg" {
            image::ImageFormat::Jpeg
        } else {
            image::ImageFormat::Png
        };
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let save_path = compacted_path.join(file_name);
        let img = ImageReader::open(&path)?.decode()?;
        let resized = Strategy::Bound(*bound).apply(&img);
        encode_with_quality(&resized, save_path, quality, format)?;
    }
    Ok(())
}
