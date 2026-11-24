//! Renders the logos in `logo/` to the images the game and the studio use (docs/logo.md):
//!
//! - from `logo/star-racer.svg`, the game's: `site/favicon.png` (64 px), the site's and the game
//!   page's tab icon; `client/web/icon.png` (512) and `client/web/icon-192.png`, the page's home
//!   screen icons; `logo/star-racer.png` (1024); and `logo/star-racer.ico`, the Windows program's,
//!   its shortcut's and its installer's, holding 16 to 256 px so Windows never scales one itself;
//! - from `logo/studio.svg`, the studio's: `logo/studio.png` (1024).

use std::fs;

use resvg::{tiny_skia, usvg};

fn main() {
    let game = load("logo/star-racer.svg");
    for (path, size) in [
        ("site/favicon.png", 64),
        ("client/web/icon.png", 512),
        ("client/web/icon-192.png", 192),
        ("logo/star-racer.png", 1024),
    ] {
        write_png(&game, path, size);
    }
    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16, 24, 32, 48, 64, 128, 256] {
        let image = ico::IconImage::from_rgba_data(size, size, straight(&render(&game, size)));
        icon.add_entry(ico::IconDirEntry::encode(&image).expect("encodes the size"));
    }
    icon.write(fs::File::create("logo/star-racer.ico").expect("creates the icon"))
        .expect("writes the icon");
    println!("logo/star-racer.ico (16 to 256 px)");

    write_png(&load("logo/studio.svg"), "logo/studio.png", 1024);
}

fn load(path: &str) -> usvg::Tree {
    let svg = fs::read(path)
        .unwrap_or_else(|error| panic!("cannot read {path} ({error}): run from the repository's root"));
    usvg::Tree::from_data(&svg, &usvg::Options::default()).expect("the logo parses")
}

fn render(tree: &usvg::Tree, size: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a size above zero");
    let scale = size as f32 / tree.size().width();
    resvg::render(tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
}

/// tiny-skia keeps its colors multiplied by their alpha; files want them straight.
fn straight(pixmap: &tiny_skia::Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect()
}

fn write_png(tree: &usvg::Tree, path: &str, size: u32) {
    let pixmap = render(tree, size);
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("writes the header");
    writer.write_image_data(&straight(&pixmap)).expect("writes the pixels");
    drop(writer);
    fs::write(path, out).expect("writes the image");
    println!("{path} ({size} px)");
}
