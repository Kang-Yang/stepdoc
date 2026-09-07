use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD, Engine};
use docx_rs::{
    AlignmentType, Docx, LineSpacing, LineSpacingType, Paragraph, Pic, Run, RunFonts, Style,
    StyleType,
};
use image::GenericImageView;

use crate::models::RecordedStep;

/// Max display width in the DOCX body (~A4 content area with margins).
const MAX_IMAGE_WIDTH_PX: u32 = 520;
const EMU_PER_PX: u32 = 9525;

const FONT_SONG: &str = "宋体";
const CREATOR_XML: &[u8] = b"<dc:creator>stepdoc</dc:creator>";
const LAST_MODIFIED_BY_XML: &[u8] = b"<cp:lastModifiedBy>stepdoc</cp:lastModifiedBy>";
/// DOCX font size is measured in half-points.
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
    // docx-rs stores core.xml uncompressed; keep replacements the same length.
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
    Some(
        Pic::new(&raw).size(
            display_w.saturating_mul(EMU_PER_PX),
            display_h.saturating_mul(EMU_PER_PX),
        ),
    )
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
