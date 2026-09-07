//! Dev probe: re-run the same RapidOCR engine used by the app over saved OCR debug crops and
//! print each detected line's rect, so click-containment rules can be calibrated on real data.
//!
//! Usage: cargo run --example ocr_rects -- <crop.png> [more.png ...]

use std::path::PathBuf;

use image::GenericImageView;
use rapidocr_core::config::PipelineConfig;
use rapidocr_core::model::{ModelCache, ModelDownloadMode, PPOCRV5_CH_MOBILE};
use rapidocr_core::RapidOcr;

fn main() {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if paths.is_empty() {
        eprintln!("usage: ocr_rects <crop.png> [...]");
        std::process::exit(2);
    }

    let model_dir = dirs_ocr_models();
    let cache = ModelCache::new(&model_dir);
    let pipeline = PipelineConfig {
        use_det: true,
        use_cls: false,
        use_rec: true,
    };
    cache
        .ensure_model_set_for_pipeline(&PPOCRV5_CH_MOBILE, pipeline, ModelDownloadMode::Missing)
        .expect("model set");
    let cfg = cache.config_for(&PPOCRV5_CH_MOBILE).with_pipeline(pipeline);
    let mut engine = RapidOcr::new(cfg).expect("engine");

    for path in &paths {
        let image = image::open(path).expect("open crop");
        let (w, h) = image.dimensions();
        println!("=== {} ({}x{}) ===", path.display(), w, h);
        let temp = std::env::temp_dir().join("stepdoc-ocr-probe.png");
        image.save(&temp).expect("save temp");
        let output = engine.run_path(&temp).expect("run ocr");
        let _ = std::fs::remove_file(&temp);
        for line in &output.lines {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for [x, y] in &line.bbox.points {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            println!(
                "  text={:?} score={:.3} rect=({:.0},{:.0},{:.0},{:.0}) center=({:.0},{:.0})",
                line.text,
                line.score,
                min_x,
                min_y,
                max_x - min_x,
                max_y - min_y,
                (min_x + max_x) / 2.0,
                (min_y + max_y) / 2.0
            );
        }
    }
}

fn dirs_ocr_models() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join("stepdoc").join("ocr-models"))
            .expect("LOCALAPPDATA")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".local/share/stepdoc/ocr-models"))
            .expect("HOME")
    }
}
