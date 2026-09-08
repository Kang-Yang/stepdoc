use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD, Engine};
use docx_rs::{
    AlignmentType, Docx, LineSpacing, LineSpacingType, Paragraph, Pic, Run, RunFonts, Style,
    StyleType,
};
use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, GenericImageView, ImageEncoder};

use crate::models::RecordedStep;

/// DOCX 正文中的最大显示宽度（约 A4 内容区加页边距的宽度）。
const MAX_IMAGE_WIDTH_PX: u32 = 520;
const EMU_PER_PX: u32 = 9525;

const FONT_SONG: &str = "宋体";
const CREATOR_XML: &[u8] = b"<dc:creator>stepdoc</dc:creator>";
const LAST_MODIFIED_BY_XML: &[u8] = b"<cp:lastModifiedBy>stepdoc</cp:lastModifiedBy>";
/// DOCX 字号以半磅为单位。
const SIZE_BODY: usize = 24;
const SIZE_STEP: usize = 28;
const SIZE_TITLE: usize = 44;

fn song_fonts() -> RunFonts {
    RunFonts::new()
        .ascii(FONT_SONG)
        .hi_ansi(FONT_SONG)
        .east_asia(FONT_SONG)
        .cs(FONT_SONG)
}

fn text_run(text: impl Into<String>) -> Run {
    Run::new().add_text(text).fonts(song_fonts())
}

fn build_docx(title: &str) -> Docx {
    let fonts = song_fonts();
    let body_line_spacing = LineSpacing::new()
        .line_rule(LineSpacingType::Auto)
        .line(360);

    Docx::new()
        .default_fonts(fonts.clone())
        .default_size(SIZE_BODY)
        .default_line_spacing(body_line_spacing)
        .add_style(
            Style::new("Heading1", StyleType::Paragraph)
                .name("标题 1")
                .fonts(fonts.clone())
                .size(SIZE_TITLE)
                .bold()
                .align(AlignmentType::Center)
                .line_spacing(
                    LineSpacing::new()
                        .before(240)
                        .after(240)
                        .line_rule(LineSpacingType::Auto)
                        .line(360),
                )
                .outline_lvl(0),
        )
        .add_style(
            Style::new("Heading2", StyleType::Paragraph)
                .name("标题 2")
                .based_on("Normal")
                .fonts(fonts)
                .size(SIZE_STEP)
                .bold()
                .line_spacing(
                    LineSpacing::new()
                        .before(160)
                        .after(80)
                        .line_rule(LineSpacingType::Auto)
                        .line(360),
                )
                .outline_lvl(1),
        )
        .add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(text_run(title)),
        )
}

pub fn render_docx(title: &str, steps: &[RecordedStep]) -> Result<Vec<u8>, String> {
    let mut docx = build_docx(title);

    for (index, step) in steps.iter().enumerate() {
        let step_no = index + 1;
        docx = docx.add_paragraph(
            Paragraph::new()
                .style("Heading2")
                .add_run(text_run(format!("{step_no}. {}", step.description))),
        );

        match step.image_base64.as_deref().and_then(prepare_image_for_docx) {
            Some(pic) => {
                docx = docx.add_paragraph(
                    Paragraph::new()
                        .align(AlignmentType::Center)
                        .add_run(Run::new().add_image(pic)),
                );
            }
            None => {
                docx = docx.add_paragraph(
                    Paragraph::new().add_run(text_run("（本步骤无截图）").size(SIZE_BODY)),
                );
            }
        }
    }

    let mut buffer = Vec::new();
    docx.build()
        .pack(Cursor::new(&mut buffer))
        .map_err(|error| error.to_string())?;

    apply_doc_author(&mut buffer);

    Ok(buffer)
}

fn apply_doc_author(buffer: &mut Vec<u8>) {
    // docx-rs 以未压缩方式存储 core.xml；替换时需保持原长度。
    replace_bytes(
        buffer,
        b"<dc:creator>unknown</dc:creator>",
        CREATOR_XML,
    );
    replace_bytes(
        buffer,
        b"<cp:lastModifiedBy>unknown</cp:lastModifiedBy>",
        LAST_MODIFIED_BY_XML,
    );
}

fn replace_bytes(buffer: &mut Vec<u8>, from: &[u8], to: &[u8]) {
    debug_assert_eq!(from.len(), to.len());
    let Some(start) = buffer
        .windows(from.len())
        .position(|window| window == from)
    else {
        return;
    };
    buffer.splice(start..start + from.len(), to.iter().copied());
}

fn prepare_image_for_docx(base64: &str) -> Option<Pic> {
    let raw = STANDARD.decode(base64.trim()).ok()?;
    if raw.is_empty() {
        return None;
    }

    let image = image::load_from_memory(&raw).ok()?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let (display_w, display_h) = display_size(width, height);

    // 超过正文宽度上限的高分屏/4K 截图，缩小后再重新编码嵌入，避免把悬殊的原图整张塞进文档；
    // 未超限的小图原样嵌入即可。
    let embedded = if width > MAX_IMAGE_WIDTH_PX {
        let resized = image.resize_exact(
            display_w,
            display_h,
            image::imageops::FilterType::Lanczos3,
        );
        encode_jpeg(&resized, 90)?
    } else {
        raw
    };

    Some(
        Pic::new(&embedded).size(
            display_w.saturating_mul(EMU_PER_PX),
            display_h.saturating_mul(EMU_PER_PX),
        ),
    )
}

/// 把图片按指定 JPEG 质量编码为字节。
fn encode_jpeg(image: &image::DynamicImage, quality: u8) -> Option<Vec<u8>> {
    let rgb = image.to_rgb8();
    let mut out = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut out, quality);
    encoder
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8,
        )
        .ok()?;
    Some(out.into_inner())
}

fn display_size(width: u32, height: u32) -> (u32, u32) {
    if width <= MAX_IMAGE_WIDTH_PX {
        return (width, height);
    }

    let display_w = MAX_IMAGE_WIDTH_PX;
    let display_h = ((height as f64) * (display_w as f64 / width as f64))
        .round()
        .max(1.0) as u32;
    (display_w, display_h)
}
