use image::{ImageBuffer, Rgba};

use crate::core::{
    hangul::{all_hangul_syllables, decompose_hangul, get_jamo_char},
    render::{RenderContext, compose_pixels},
};
use crate::io::{adobe_kr_9, ks_x_1001};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(all(not(target_arch = "wasm32"), feature = "cli"), derive(clap::ValueEnum))]
pub enum FileNameFormat {
    #[default]
    /// 가.png
    Char,
    /// AC00.png
    Hex,
    /// UAC00.png
    UHex,
    /// U+AC00.png
    UPlusHex,
}

impl FileNameFormat {
    pub fn format(&self, ch: char) -> String {
        match self {
            FileNameFormat::Char => format!("{}.png", ch),
            FileNameFormat::Hex => format!("{:04X}.png", ch as u32),
            FileNameFormat::UHex => format!("U{:04X}.png", ch as u32),
            FileNameFormat::UPlusHex => format!("U+{:04X}.png", ch as u32),
        }
    }
}

pub struct ExportConfig {
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub color_text: [u8; 4],
    pub color_bg: [u8; 4],
    pub columns: u32,
    pub name_format: FileNameFormat,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum CharScope {
    #[default]
    All,
    KsX1001,
    AdobeKr9,
    Custom,
}

pub fn get_char_list(scope: &CharScope, custom_text: &str) -> Vec<char> {
    match scope {
        CharScope::All => all_hangul_syllables().collect(),
        CharScope::KsX1001 => ks_x_1001::CHARS.chars().filter(|c| c.is_alphabetic()).collect(),
        CharScope::AdobeKr9 => adobe_kr_9::CHARS.chars().filter(|c| c.is_alphabetic()).collect(),
        CharScope::Custom => {
            let mut out = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for ch in custom_text.chars() {
                if (0xAC00..=0xD7A3).contains(&(ch as u32)) && seen.insert(ch) {
                    out.push(ch);
                }
            }
            out
        }
    }
}

pub fn render_single_char(ctx: &RenderContext<'_>, ch: char, cfg: &ExportConfig) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::from_pixel(cfg.canvas_w, cfg.canvas_h, Rgba(cfg.color_bg));

    if let Some(res) = ctx.engine.layout_char(ctx.store, ch, decompose_hangul, get_jamo_char) {
        let pixels = compose_pixels(ctx.store, &res, ctx.canvas_w, ctx.canvas_h);
        for px in pixels {
            if px.x >= 0 && px.y >= 0 && px.x < cfg.canvas_w as i32 && px.y < cfg.canvas_h as i32 {
                img.put_pixel(px.x as u32, px.y as u32, Rgba(cfg.color_text));
            }
        }
    }
    img
}

pub fn build_sheet_image(ctx: &RenderContext<'_>, chars: &[char], cfg: &ExportConfig) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    if chars.is_empty() {
        return ImageBuffer::from_pixel(1, 1, Rgba(cfg.color_bg));
    }

    let count = chars.len() as u32;
    let cols = cfg.columns.max(1);
    let rows = count.div_ceil(cols);
    let mut sheet = ImageBuffer::from_pixel(cols * cfg.canvas_w, rows * cfg.canvas_h, Rgba(cfg.color_bg));

    for (i, &ch) in chars.iter().enumerate() {
        let char_img = render_single_char(ctx, ch, cfg);
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x_off = col * cfg.canvas_w;
        let y_off = row * cfg.canvas_h;

        for y in 0..cfg.canvas_h {
            for x in 0..cfg.canvas_w {
                let px = char_img.get_pixel(x, y);
                if px.0[3] != 0 {
                    sheet.put_pixel(x_off + x, y_off + y, *px);
                }
            }
        }
    }
    sheet
}

#[cfg(target_arch = "wasm32")]
pub fn encode_png(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap_or(());
    bytes
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_sheet_to_path(ctx: &RenderContext<'_>, chars: &[char], cfg: &ExportConfig, path: &std::path::Path) {
    let image = build_sheet_image(ctx, chars, cfg);
    if let Err(e) = image.save(path) {
        log::error!("Save failed: {:?}", e);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_individual_to_dir(ctx: &RenderContext<'_>, chars: &[char], cfg: &ExportConfig, dir: &std::path::Path) {
    for &ch in chars {
        let img = render_single_char(ctx, ch, cfg);
        let _ = img.save(dir.join(cfg.name_format.format(ch)));
    }
    log::info!("Export done: {} files", chars.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_char_ga() {
        assert_eq!(FileNameFormat::Char.format('가'), "가.png");
    }

    #[test]
    fn format_hex_ga() {
        assert_eq!(FileNameFormat::Hex.format('가'), "AC00.png");
    }

    #[test]
    fn format_uhex_ga() {
        assert_eq!(FileNameFormat::UHex.format('가'), "UAC00.png");
    }

    #[test]
    fn format_uplushex_ga() {
        assert_eq!(FileNameFormat::UPlusHex.format('가'), "U+AC00.png");
    }

    #[test]
    fn format_hex_hih() {
        assert_eq!(FileNameFormat::Hex.format('힣'), "D7A3.png");
    }

    #[test]
    fn format_hex_ascii_a_zero_padded() {
        assert_eq!(FileNameFormat::Hex.format('A'), "0041.png");
    }

    #[test]
    fn format_char_ascii() {
        assert_eq!(FileNameFormat::Char.format('A'), "A.png");
    }

    #[test]
    fn get_char_list_all_hangul() {
        let list = get_char_list(&CharScope::All, "");
        assert_eq!(list.len(), 11172);
        assert_eq!(list[0], '가');
        assert_eq!(*list.last().unwrap(), '힣');
    }

    #[test]
    fn get_char_list_ks1001_nonempty_and_alphabetic() {
        let list = get_char_list(&CharScope::KsX1001, "");
        assert!(!list.is_empty());
        assert!(list.iter().all(|c| c.is_alphabetic()));
    }

    #[test]
    fn get_char_list_custom_dedup_and_order() {
        let list = get_char_list(&CharScope::Custom, "가나가다나");
        assert_eq!(list, vec!['가', '나', '다']);
    }

    #[test]
    fn get_char_list_custom_ignores_non_hangul() {
        let list = get_char_list(&CharScope::Custom, "Hello가World나");
        assert_eq!(list, vec!['가', '나']);
    }
}
