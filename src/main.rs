use std::env;
use std::path::{Path, PathBuf};

unsafe fn process(menu_path: &Path, output_dir: &Path, sprites: bool, format: ImageFormat) -> Result<(), Ferrisouls::errors::FerrisoulsError> {
    use Ferrisouls::{binary::IO, binders::Binder};
    use Ferrisouls::formats::{tpf::{TPF, TPFTexture}, sblyt::{Layout, LayoutBinder}};
    use Ferrisouls::textures::{image, dds::DDS};
    use Ferrisouls::files;

    unsafe { Ferrisouls::oodle::auto_oodle().unwrap(); }

    std::fs::create_dir_all(output_dir)?;

    let tpfs = files::get_files_by(
        menu_path.to_path_buf(),
        |s| s.ends_with(".tpf") || s.ends_with(".tpf.dcx")
    );
    let sblyts = files::get_files_by(
        menu_path.to_path_buf(),
        |s| s.contains(".sblytbnd")
    );

    let mut textures: Vec<TPFTexture> = Vec::new();
    let mut layouts: Vec<Layout> = Vec::new();

    {
        for tpf_path in tpfs {
            let (tpf, _) = unsafe { TPF::read(&tpf_path)? };
            println!("Unpacked TPF: {:?} with {} entries", tpf_path.file_name(), tpf.count());
            textures.extend(tpf.textures);
        }

        for sblyt_path in sblyts {
            let (sblyt, _) = unsafe { LayoutBinder::read(&sblyt_path)? };
            println!("Unpacked Layout Binder: {:?} with {} entries", sblyt_path.file_name(), sblyt.count());
            layouts.extend(sblyt.entries);
        }
    }

    for tex in textures {
        let img = if matches!(format, ImageFormat::DDS(_)) {
            tex.to_dds()?.to_bytes()?
        } else {
            tex.to(format)?
        };

        let ext = extension(&format);

        std::fs::write(
            output_dir.join(format!("{}.{}", tex.name, ext)),
            &img,
        )?;
        println!("Wrote TPFTexture: {:?}.", tex.name);

        if !sprites {
            continue;
        }

        let layout = layouts
            .iter()
            .find(|l| l.stem() == tex.name);

        if let Some(layout) = layout {
            println!("Found layout for {}! Cropping {} subtextures...", tex.name, layout.entries.len());

            for sub in layout.iter() {
                let sprite = match format {
                    ImageFormat::DDS(_) => {
                        let dds = DDS::from_bytes(&img)?;

                        if (dds.header.width % 4 != 0)
                        || (dds.header.height % 4 != 0) {
                            eprintln!("Non-4 divisible dimensions for DDS: {}. Skipping.", tex.name);
                            continue;
                        }
                        image::crop_dds(&dds, sub.x, sub.y, sub.width, sub.height)?.to_bytes()?
                    },
                    ImageFormat::PNG => { image::crop_png(&img, sub.x, sub.y, sub.width, sub.height)? },
                    ImageFormat::TGA => { image::crop_tga(&img, sub.x, sub.y, sub.width, sub.height)? },
                    _=> {
                        eprintln!("Invalid ImageFormat: {:?}", format);
                        continue;
                    }
                };
                std::fs::write(
                    output_dir.join(format!("{}.{}", sub.name.replace(".png", ""), ext)),
                    &sprite,
                )?;
            }
        }
    }
    Ok(())
}

use Ferrisouls::textures::dxgi::DxgiFormat;
use Ferrisouls::textures::image::ImageFormat;

fn parse_format(s: &str) -> Option<ImageFormat> {
    match s.to_ascii_lowercase().as_str() {
        "tga" => Some(ImageFormat::TGA),
        "png" => Some(ImageFormat::PNG),
        "dds" => Some(ImageFormat::DDS(DxgiFormat::BC7_UNORM)),
        _ => None,
    }
}

fn extension(format: &ImageFormat) -> &'static str {
    match format {
        ImageFormat::TGA => "tga",
        ImageFormat::PNG => "png",
        ImageFormat::DDS(_) => "dds",
        _ => "bin"
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let mut sprites = false;
    let mut format = ImageFormat::TGA;
    let mut positional: Vec<PathBuf> = Vec::new();

    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--sprites=") {
            sprites = match value {
                "yes" => true,
                "no" => false,
                _ => {
                    eprintln!("Invalid mode: {value}. Expected one of: [yes|no]");
                    std::process::exit(1);
                }
            };
        } else if let Some(value) = arg.strip_prefix("--format=") {
            format = match parse_format(value) {
                Some(f) => f,
                None => {
                    eprintln!("Invalid format: {value}. Expected one of: [tga|png|dds]");
                    std::process::exit(1);
                }
            };
        } else if arg.starts_with('-') {
            eprintln!("Unknown argument: {arg}");
            std::process::exit(1);
        } else {
            positional.push(PathBuf::from(arg));
        }
    }

    if positional.len() == 0 { //no paths
        eprintln!("Usage: program <input> [output] [--sprites=yes|no] [--format=tga|png|dds]");
        std::process::exit(1);
    }

    let input_path = &positional[0];
    let output_path = if positional.len() < 2 {
        &env::current_exe().unwrap()
            .parent().unwrap()
            .join("images")
    } else {
        &positional[1]
    };

    println!("-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-\nBegin Processing...\n-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-.-");

    unsafe {
        process(
            input_path,
            output_path,
            sprites,
            format
        ).unwrap()
    }

}
