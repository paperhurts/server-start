//! Renders icon assets from `assets/server-start-icon.svg` at build time:
//!
//! 1. A 256x256 raw-RGBA blob (`tray-icon.rgba`) in OUT_DIR, embedded by
//!    `src/main.rs` via `include_bytes!` for the system-tray icon.
//! 2. A multi-resolution `.ico` (16/32/48/64/128/256) in OUT_DIR, embedded
//!    as a Windows resource (icon ID 1) so the .exe shows our icon in
//!    File Explorer, Alt-Tab, the taskbar, and Task Manager.
//!
//! The SVG is the single source of truth. Each raster output is rendered
//! directly from the vector at its target size — no downsampling — so the
//! 16x16 and 32x32 ICO frames stay crisp instead of going to mush.

use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use resvg::tiny_skia;
use resvg::usvg;

const SOURCE_SVG: &str = "assets/server-start-icon.svg";

/// Side length of the tray-icon RGBA blob. Larger than the 16-32px Windows
/// tray displays so high-DPI scaling stays crisp; tray-icon downsamples
/// internally.
const TRAY_SIZE: u32 = 256;

/// Resolutions packed into the multi-resolution .ico. Windows picks the
/// closest match for each context (Alt-Tab uses 32, File Explorer thumbnails
/// use 256, etc.).
const ICO_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

fn main() {
    println!("cargo:rerun-if-changed={}", SOURCE_SVG);
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    let svg_data = std::fs::read(SOURCE_SVG)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", SOURCE_SVG, e));
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .expect("Failed to parse SVG");

    write_tray_rgba(&tree, &out_dir);
    let ico_path = write_ico(&tree, &out_dir);
    embed_exe_icon(&ico_path);
}

fn write_tray_rgba(tree: &usvg::Tree, out_dir: &PathBuf) {
    let rgba = render_svg(tree, TRAY_SIZE);
    std::fs::write(out_dir.join("tray-icon.rgba"), rgba)
        .expect("Failed to write tray-icon.rgba");
}

fn write_ico(tree: &usvg::Tree, out_dir: &PathBuf) -> PathBuf {
    let ico_path = out_dir.join("server-start.ico");
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &size in ICO_SIZES {
        let rgba = render_svg(tree, size);
        let image_data = ico::IconImage::from_rgba_data(size, size, rgba);
        let entry = ico::IconDirEntry::encode(&image_data)
            .expect("Failed to encode .ico entry");
        icon_dir.add_entry(entry);
    }

    let writer = BufWriter::new(
        File::create(&ico_path).expect("Failed to create .ico file"),
    );
    icon_dir.write(writer).expect("Failed to write .ico");
    ico_path
}

/// Embed the .ico as a Windows resource. `winresource::compile()` errors out
/// when the target env isn't `gnu` or `msvc`, so we skip it on non-Windows
/// targets (e.g. CI running `cargo check` on Linux).
fn embed_exe_icon(ico_path: &PathBuf) {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("ico path not UTF-8"));
    res.compile().expect("Failed to compile Windows resource");
}

/// Render the SVG into a `size`x`size` square buffer of straight (un-
/// premultiplied) RGBA bytes. Preserves the SVG's aspect ratio by scaling
/// to fit and centering with transparent padding — non-issue for the
/// current square viewBox but keeps the pipeline robust to art changes.
fn render_svg(tree: &usvg::Tree, size: u32) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .expect("Failed to allocate pixmap");

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let tx = (size as f32 - svg_size.width() * scale) / 2.0;
    let ty = (size as f32 - svg_size.height() * scale) / 2.0;
    let transform = tiny_skia::Transform::from_scale(scale, scale).post_translate(tx, ty);

    resvg::render(tree, transform, &mut pixmap.as_mut());

    // tiny-skia stores premultiplied RGBA; the ico crate and tray-icon
    // both expect straight (un-premultiplied) RGBA. Convert pixel by pixel.
    let mut rgba = vec![0u8; (size as usize) * (size as usize) * 4];
    for (i, pixel) in pixmap.pixels().iter().enumerate() {
        let c = pixel.demultiply();
        let idx = i * 4;
        rgba[idx] = c.red();
        rgba[idx + 1] = c.green();
        rgba[idx + 2] = c.blue();
        rgba[idx + 3] = c.alpha();
    }
    rgba
}
