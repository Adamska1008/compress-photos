use clap::{arg, command, value_parser};
use image::{GenericImageView, ImageReader};

fn compress_one(filename: &str, max_min: u32, quality: u8) -> Result<(), image::ImageError> {
    println!("Compressing {}", filename);
    let img = ImageReader::open(filename)?.decode()?;
    let (w, h) = img.dimensions();
    if w <= max_min && h <= max_min {
        img.save(format!("./compacted/{}", filename))?;
        return Ok(());
    }
    let (w, h) = if w > h {
        (w * max_min / h, max_min)
    } else {
        (max_min, h * max_min / w)
    };
    let resized = img.resize(w, h, image::imageops::FilterType::Lanczos3);
    // if no dir, create it
    if !std::path::Path::new("./compacted").exists() {
        std::fs::create_dir("./compacted")?;
    }
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        std::fs::File::create(format!("./compacted/{}", filename))?,
        quality,
    );
    encoder.encode_image(&resized)?;
    println!(
        "Compressed {} from {:?} to {:?}",
        filename,
        img.dimensions(),
        resized.dimensions()
    );
    Ok(())
}

fn main() -> Result<(), image::ImageError> {
    let matches = command!()
        .arg(arg!([filename] "Specify the filename to compress"))
        .arg(
            arg!(-m --max_min <MAX_MIN> "Specify the max min")
                .value_parser(value_parser!(u32))
                .default_value("1200"),
        )
        .arg(
            arg!(-q --quality <QUALITY> "Specify the quality")
                .value_parser(value_parser!(u8))
                .default_value("75"),
        )
        .get_matches();
    let max_min = matches.get_one::<u32>("max_min").unwrap();
    let quality = matches.get_one::<u8>("quality").unwrap();
    if let Some(filename) = matches.get_one::<String>("filename") {
        compress_one(filename, *max_min, *quality)?;
    } else {
        // compress all file in the dir
        let files = std::fs::read_dir("./")?;
        for file in files {
            let file = file?;
            let path = file.path();
            if path.is_file() && path.extension().unwrap_or_default() == "jpg" {
                compress_one(path.to_str().unwrap(), *max_min, *quality)?;
            }
        }
    }
    Ok(())
}
