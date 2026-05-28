use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use hangul_syllable::core::render::RenderContext;
use hangul_syllable::io::export::{CharScope, ExportConfig, FileNameFormat, export_individual_to_dir, export_sheet_to_path, get_char_list};
use hangul_syllable::{LayoutEngine, load_project_from_path};

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ExportMode {
    /// 모든 글자를 한 장의 스프라이트 시트로 내보내기
    Sheet,
    /// 글자마다 개별 PNG 파일로 내보내기
    Individual,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, ValueEnum)]
pub enum ExportTarget {
    /// 전체 한글 11,172자
    #[default]
    All,
    /// KS X 1001 표준 2,350자
    KsX1001,
    /// Adobe-KR-9
    AdobeKr9,
    /// --chars 로 직접 지정한 글자만
    Custom,
}

#[derive(Parser)]
#[command(author, version, about = "Hangul Syllable Editor CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 프로젝트 파일을 읽어 이미지로 내보냅니다.
    Export {
        /// 렌더링할 프로젝트 파일 (.hangul)
        #[arg(short, long)]
        project: PathBuf,

        /// 결과물 폴더 (sheet → <out>/font_sheet.png, individual → <out>/<글자>.png)
        #[arg(short, long, default_value = "./output")]
        out: PathBuf,

        /// 내보내기 모드 [sheet, individual]
        #[arg(short, long, default_value = "sheet")]
        mode: ExportMode,

        /// 내보낼 글자 범위 [all, ks-x-1001, adobe-kr-9, custom]
        #[arg(short, long, default_value = "all")]
        target: ExportTarget,

        /// 직접 입력할 한글 음절 (--target custom 또는 단독 사용 시)
        #[arg(long)]
        chars: Option<String>,

        /// 스프라이트 시트의 열(Column) 개수 (sheet 모드 전용)
        #[arg(short, long, default_value_t = 32)]
        columns: u32,

        /// 글자 색상 RRGGBB hex (기본값: ffffff)
        #[arg(long, default_value = "ffffff")]
        text_color: String,

        /// 배경 색상 RRGGBB hex (생략 시 투명)
        #[arg(long)]
        bg_color: Option<String>,

        /// 파일 이름 형식 (individual 모드 전용) [char, hex, u-hex, u-plus-hex]
        #[arg(long, default_value = "hex")]
        name_format: FileNameFormat,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Export {
            project,
            out,
            mode,
            target,
            chars,
            columns,
            text_color,
            bg_color,
            name_format,
        } => {
            let (scope, custom_text) = if let Some(text) = chars {
                (CharScope::Custom, text)
            } else {
                let scope = match target {
                    ExportTarget::All => CharScope::All,
                    ExportTarget::KsX1001 => CharScope::KsX1001,
                    ExportTarget::AdobeKr9 => CharScope::AdobeKr9,
                    ExportTarget::Custom => {
                        eprintln!("오류: --target custom은 --chars 옵션과 함께 사용해야 합니다.");
                        std::process::exit(1);
                    }
                };
                (scope, String::new())
            };

            let char_list = get_char_list(&scope, &custom_text);
            if char_list.is_empty() {
                eprintln!("오류: 내보낼 글자가 없습니다.");
                std::process::exit(1);
            }

            let cfg = ExportConfig {
                canvas_w: 0,
                canvas_h: 0,
                color_text: parse_hex_color(&text_color),
                color_bg: bg_color.as_deref().map(parse_hex_color).unwrap_or([0, 0, 0, 0]),
                columns,
                name_format,
            };

            run_export(&project, &out, mode, &char_list, cfg)?;
        }
    }

    Ok(())
}

fn run_export(
    project_path: &Path,
    out: &Path,
    mode: ExportMode,
    chars: &[char],
    mut cfg: ExportConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("프로젝트 로딩 중: {}", project_path.display());

    let data = load_project_from_path(project_path)?;
    cfg.canvas_w = data.canvas_w as u32;
    cfg.canvas_h = data.canvas_h as u32;

    let engine = LayoutEngine { rules: data.rules };
    let ctx = RenderContext {
        store: &data.store,
        engine: &engine,
        canvas_w: data.canvas_w,
        canvas_h: data.canvas_h,
    };

    println!("{} 모드, {}개 글자 내보내기...", format_mode(mode), chars.len());

    std::fs::create_dir_all(out)?;

    match mode {
        ExportMode::Sheet => {
            let path = out.join("font_sheet.png");
            export_sheet_to_path(&ctx, chars, &cfg, &path);
            println!("완료: {}", path.display());
        }
        ExportMode::Individual => {
            export_individual_to_dir(&ctx, chars, &cfg, out);
            println!("완료: {}개 파일 → {}", chars.len(), out.display());
        }
    }

    Ok(())
}

fn format_mode(mode: ExportMode) -> &'static str {
    match mode {
        ExportMode::Sheet => "시트",
        ExportMode::Individual => "개별",
    }
}

fn parse_hex_color(hex: &str) -> [u8; 4] {
    let h = hex.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0xFF_FF_FF);
    [((n >> 16) & 0xFF) as u8, ((n >> 8) & 0xFF) as u8, (n & 0xFF) as u8, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_white() {
        assert_eq!(parse_hex_color("ffffff"), [255, 255, 255, 255]);
    }

    #[test]
    fn parse_hex_black() {
        assert_eq!(parse_hex_color("000000"), [0, 0, 0, 255]);
    }

    #[test]
    fn parse_hex_with_hash_prefix() {
        assert_eq!(parse_hex_color("#ff0000"), [255, 0, 0, 255]);
    }

    #[test]
    fn parse_hex_uppercase() {
        assert_eq!(parse_hex_color("FF8800"), [255, 136, 0, 255]);
    }

    #[test]
    fn parse_hex_invalid_falls_back_to_white() {
        assert_eq!(parse_hex_color("xyz"), [255, 255, 255, 255]);
    }

    #[test]
    fn parse_hex_alpha_always_255() {
        let c = parse_hex_color("aabbcc");
        assert_eq!(c[3], 255);
    }
}
