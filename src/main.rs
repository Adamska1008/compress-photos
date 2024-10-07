use std::{
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use clap::{arg, command, value_parser, Command};
use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    ColorType, GenericImageView, ImageEncoder, ImageReader,
};
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

fn compress_one(
    filename: &str,
    strategy: &Strategy,
) -> Result<image::DynamicImage, image::ImageError> {
    let img = ImageReader::open(filename)?.decode()?;
    info!("Compressing {} size: {:?}", filename, img.dimensions());
    let resized = strategy.apply(&img);
    // If dir not exists, create it
    info!("Compressed {} to size {:?}", filename, resized.dimensions());
    Ok(resized)
}

fn cli() -> Command {
    command!()
        .arg(arg!([filename] "Specify the filename to compress"))
        .arg(
            arg!(-b --bound <BOUND> "Specify the bound of image")
                .value_parser(Bound::from_str)
                .default_value("1600,1600"),
        )
}

fn init_logger() {
    SimpleLogger::new()
        .init()
        .map_err(|e| eprintln!("Failed to initialize logger: {}", e))
        .unwrap();
}

fn main() -> Result<(), image::ImageError> {
    init_logger();
    let ext_to_check = ["jpg", "png", "jpeg"];
    let matches = cli().get_matches();
    let bound = matches.get_one::<Bound>("bound").unwrap();

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
        std::fs::create_dir("compacted").unwrap();
    }

    let compacted_path = PathBuf::from("compacted");

    for path in paths {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        if !ext_to_check.contains(&ext) {
            continue;
        }

        let file_name = path.file_name().unwrap().to_str().unwrap();
        let save_path = compacted_path.join(file_name);
        let resized = compress_one(path.to_str().unwrap(), &Strategy::Bound(*bound))?;
        resized.save_with_format(
            save_path,
            if ext == "jpg" || ext == "jpeg" {
                image::ImageFormat::Jpeg
            } else {
                image::ImageFormat::Png
            },
        )?;
    }
    Ok(())
}
