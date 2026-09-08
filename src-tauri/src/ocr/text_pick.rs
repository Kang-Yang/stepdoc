pub fn reject_reason(text: &str) -> Option<&'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some("empty");
    }
    if trimmed.chars().count() > 24 {
        return Some("too_long");
    }
    if trimmed.contains('\u{FFFD}') {
        return Some("replacement_char");
    }
    if is_digit_only_label(trimmed) {
        return Some("digit_only");
    }
    if is_repeated_glyph_noise(trimmed) {
        return Some("repeated_glyph");
    }
    if trimmed.chars().count() == 1 {
        let ch = trimmed.chars().next().unwrap_or(' ');
        if !is_cjk(ch) {
            return Some("single_non_cjk");
        }
        if is_unlikely_single_cjk(ch) {
            return Some("unlikely_single_cjk");
        }
    }
    if !trimmed
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric() || is_cjk(ch))
    {
        return Some("no_alnum_or_cjk");
    }
    if is_suspicious_latin_gibberish(trimmed) {
        return Some("latin_gibberish");
    }
    None
}

pub fn is_plausible_ui_label(text: &str) -> bool {
    reject_reason(text).is_none() && {
        let trimmed = text.trim();
        let mut score = 0i32;
        let mut penalty = 0i32;

        for ch in trimmed.chars() {
            if ch.is_control() {
                return false;
            }
            if is_cjk(ch) {
                score += 3;
            } else if ch.is_ascii_alphanumeric() {
                score += 2;
            } else if is_common_ui_punct(ch) {
                score += 1;
            } else if is_private_use(ch) {
                return false;
            } else if !ch.is_whitespace() {
                penalty += 2;
            }
        }

        score > 0 && penalty * 2 <= score
    }
}

pub fn label_quality_score(text: &str) -> i32 {
    let trimmed = text.trim();
    if !is_plausible_ui_label(trimmed) {
        return i32::MIN / 4;
    }

    let mut score = 0i32;
    let char_count = trimmed.chars().count();
    let cjk_count = trimmed.chars().filter(|ch| is_cjk(*ch)).count();
    let latin_count = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .count();
    let digit_count = trimmed.chars().filter(|ch| ch.is_ascii_digit()).count();

    score += (cjk_count as i32) * 12;
    score += (latin_count as i32) * 4;
    score -= (digit_count as i32) * 8;

    if cjk_count >= 4 {
        score += 36;
    } else if cjk_count >= 2 {
        score += 28;
    } else if cjk_count == 1 {
        score -= 24;
    }

    if char_count >= 2 && char_count <= 12 {
        score += 14;
    }

    score
}

pub fn is_confident_click_label(text: &str) -> bool {
    if !is_plausible_ui_label(text) {
        return false;
    }
    let cjk = text.chars().filter(|ch| is_cjk(*ch)).count();
    let alnum = text.chars().filter(|ch| ch.is_ascii_alphanumeric()).count();
    cjk >= 2 || alnum >= 2
}

/// 当不存在任何评分候选项时的旧式回退。优先采用以点击为中心的裁剪。
pub fn pick_fallback_label<'a, I>(picks: I) -> Option<String>
where
    I: IntoIterator<Item = (&'a str, Option<&'a str>)>,
{
    let picks: Vec<_> = picks.into_iter().collect();
    const ORDER: [&str; 4] = ["center", "right", "above", "below"];

    for name in ORDER {
        if let Some(text) = label_for_region(&picks, name) {
            if is_confident_click_label(text) {
                return Some(text.to_string());
            }
        }
    }

    for name in ORDER {
        if let Some(text) = label_for_region(&picks, name) {
            if is_plausible_ui_label(text) {
                return Some(text.to_string());
            }
        }
    }

    None
}

fn label_for_region<'a>(picks: &[(&'a str, Option<&'a str>)], name: &str) -> Option<&'a str> {
    picks
        .iter()
        .find(|(region, _)| *region == name)
        .and_then(|(_, label)| *label)
}

/// 为跨区域挑选候选打分。`click_distance_sq` 是候选项到真实点击点（截图坐标空间）的平方像素
/// 距离；`reference_px` 是一个不依赖区域的长度（中心裁剪的高度），使相同的绝对距离
/// 在每一个裁剪里都能归一化成同样的程度——若按各裁剪自身尺寸归一化，较远的标签会在更宽的
/// 相邻裁剪里越过“临近加分”阈值而抢走选择。与真实点击相距较远的候选项——例如中心落在某个
/// 相邻裁剪里的多字符窗口标题——会被大幅降权，因此光标下的真实按钮能胜过它所在局部裁剪的排名。
pub fn candidate_pick_score(
    text: &str,
    source: &str,
    click_distance_sq: f32,
    reference_px: f32,
) -> i32 {
    if reject_reason(text).is_some() {
        return i32::MIN / 4;
    }

    let quality = label_quality_score(text);
    let reference = reference_px.max(1.0);
    let pixel_dist = if click_distance_sq.is_finite() {
        click_distance_sq.sqrt()
    } else {
        reference * 2.0
    };
    let relative = (pixel_dist / reference).clamp(0.0, 2.0);
    // 把与点击的接近程度看得比文本长度更重，使更短却正对点击的按钮标签，胜过一段
    // 只是在其所在裁剪区中居中的较长标题。
    let pen = (relative * 140.0).round() as i32;
    let mut score = quality.saturating_sub(pen);

    if source == "line" || source == "segment" {
        score = score.saturating_add(16);
    } else if source == "word" && text.chars().count() == 1 {
        score = score.saturating_sub(40);
    }

    if relative < 0.12 {
        score = score.saturating_add(40);
    }

    score
}

pub fn is_digit_only_label(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit())
}

/// 折叠 Windows OCR 产生的 CJK 字符间空格："企 业 定 制" → "企业定制"。
pub fn normalize_label(text: &str) -> String {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut out = String::with_capacity(chars.len());

    for (index, ch) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            let prev = out.chars().last();
            let next = chars[index + 1..]
                .iter()
                .copied()
                .find(|c| !c.is_whitespace());
            if prev.map(is_cjk).unwrap_or(false) && next.map(is_cjk).unwrap_or(false) {
                continue;
            }
            if prev.is_some() && !out.ends_with(' ') {
                out.push(' ');
            }
            continue;
        }
        out.push(*ch);
    }

    strip_icon_prefix(&out.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub fn truncate_label(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    format!("{}…", text.chars().take(max_chars).collect::<String>())
}

fn strip_icon_prefix(text: &str) -> String {
    let trimmed = text.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    let Some(cjk_at) = chars.iter().position(|ch| is_cjk(*ch)) else {
        return trimmed.to_string();
    };
    if cjk_at == 0 {
        return strip_trailing_radical(trimmed);
    }

    let prefix: String = chars[..cjk_at]
        .iter()
        .copied()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    let prefix_ok =
        !prefix.is_empty() && prefix.chars().count() <= 3 && prefix.chars().all(|ch| !is_cjk(ch));
    let rest: String = chars[cjk_at..].iter().collect();
    if prefix_ok {
        strip_trailing_radical(rest.trim())
    } else {
        strip_trailing_radical(trimmed)
    }
}

fn strip_trailing_radical(text: &str) -> String {
    let trimmed = text.trim();
    let Some(last) = trimmed.chars().last() else {
        return trimmed.to_string();
    };
    if is_unlikely_single_cjk(last) && trimmed.chars().filter(|ch| is_cjk(*ch)).count() >= 2 {
        let without: String = trimmed.chars().take(trimmed.chars().count() - 1).collect();
        return without.trim().to_string();
    }
    trimmed.to_string()
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

fn is_common_ui_punct(ch: char) -> bool {
    matches!(
        ch,
        '·' | '•' | '-' | '_' | '/' | '\\' | ':' | '：' | '（' | '）' | '(' | ')' | '…'
    )
}

fn is_private_use(ch: char) -> bool {
    matches!(ch as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD)
}

fn is_suspicious_latin_gibberish(text: &str) -> bool {
    let latin: String = text.chars().filter(|ch| ch.is_ascii_alphabetic()).collect();
    if latin.len() < 5 {
        return false;
    }
    let vowels = latin
        .chars()
        .filter(|ch| {
            matches!(
                *ch,
                'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
            )
        })
        .count();
    vowels * 4 < latin.len()
}

fn is_repeated_glyph_noise(text: &str) -> bool {
    let chars: Vec<char> = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    chars.len() >= 2
        && chars.len() <= 3
        && chars.iter().all(|ch| *ch == chars[0])
        && is_cjk(chars[0])
}

/// 单字符的汉字，通常是 OCR 残片（偏旁 / 部件），而不是 UI 标签。
fn is_unlikely_single_cjk(ch: char) -> bool {
    matches!(
        ch,
        '讠' | '钅'
            | '饣'
            | '纟'
            | '车'
            | '门'
            | '口'
            | '日'
            | '月'
            | '木'
            | '水'
            | '火'
            | '土'
            | '扌'
            | '氵'
            | '亻'
            | '忄'
            | '刂'
            | '辶'
            | '犭'
            | '礻'
            | '衤'
            | '攵'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_chinese_label() {
        assert!(is_plausible_ui_label("确定"));
        assert!(is_plausible_ui_label("设置"));
        assert!(is_plausible_ui_label("主页"));
    }

    #[test]
    fn rejects_symbol_only_text() {
        assert!(!is_plausible_ui_label("---"));
    }

    #[test]
    fn rejects_single_digit() {
        assert!(!is_plausible_ui_label("0"));
        assert!(!is_plausible_ui_label("9"));
    }

    #[test]
    fn rejects_digit_only_text() {
        assert!(!is_plausible_ui_label("12"));
    }

    #[test]
    fn prefers_settings_over_digit() {
        assert!(label_quality_score("设置") > label_quality_score("0"));
    }

    #[test]
    fn rejects_latin_gibberish() {
        assert!(!is_plausible_ui_label("Hrnqk"));
    }

    #[test]
    fn collapses_cjk_spaces() {
        assert_eq!(normalize_label("企 业 定 制"), "企业定制");
        assert_eq!(normalize_label("控 制 台 讠"), "控制台");
        assert_eq!(normalize_label("Save  As"), "Save As");
    }

    #[test]
    fn strips_icon_ocr_prefix() {
        assert_eq!(normalize_label("A\\ 用户旧管理"), "用户旧管理");
        assert_eq!(normalize_label("A\\\\ 用户旧管理"), "用户旧管理");
        assert_eq!(normalize_label("] 授权管理"), "授权管理");
    }

    #[test]
    fn rejects_koukou_noise() {
        assert_eq!(reject_reason("口 口"), Some("repeated_glyph"));
        assert_eq!(reject_reason("口口"), Some("repeated_glyph"));
    }

    #[test]
    fn rejects_unlikely_single_radical() {
        assert_eq!(reject_reason("讠"), Some("unlikely_single_cjk"));
        assert_eq!(reject_reason("口"), Some("unlikely_single_cjk"));
    }

    #[test]
    fn prefers_full_line_over_near_single_char() {
        let line = candidate_pick_score("企业定制", "line", 66930.25, 330.0);
        let word = candidate_pick_score("制", "word", 42573.25, 330.0);
        assert!(line > word, "line={line} word={word}");
    }

    /// 来自真实录制的回归测试：点击落在「新建文本文件」菜单行（距行中心 37px），
    /// 而「新建文件.」位于其下方 33px。旧有的按每个裁剪归一化的逻辑曾让更宽右侧裁剪里的
    /// 新建文件获得“临近加分”并抢走选择。改用与区域无关的参考后，被点击的行必须胜过下方项。
    #[test]
    fn on_click_row_beats_lower_item_at_similar_distance() {
        let on_click_row = candidate_pick_score("新建文本文件", "line", 1370.0, 72.0);
        let lower_item = candidate_pick_score("新建文件.", "line", 1105.0, 72.0);
        assert!(on_click_row > lower_item, "{on_click_row} vs {lower_item}");
    }

    /// 距点击 33px 不得获得“临近加分”（reference 72 -> 阈值 8.6px）。
    #[test]
    fn close_bonus_does_not_apply_at_menu_item_distance() {
        let score = candidate_pick_score("新建文件", "line", 1105.0, 72.0);
        let quality = label_quality_score("新建文件");
        let rel = 1105.0_f32.sqrt() / 72.0;
        let expected = quality - (rel * 140.0).round() as i32 + 16;
        assert_eq!(score, expected);
    }

    #[test]
    fn prefers_clicked_home_over_next_menu_item() {
        let picked = pick_fallback_label([
            ("center", Some("主页")),
            ("right", Some("常规配置")),
            ("above", None),
            ("below", Some("用户旧管理")),
        ]);
        assert_eq!(picked.as_deref(), Some("主页"));
    }

    #[test]
    fn falls_back_to_right_when_center_is_icon_only() {
        let picked = pick_fallback_label([
            ("center", None),
            ("right", Some("主页")),
            ("above", None),
            ("below", Some("用户ID管理")),
        ]);
        assert_eq!(picked.as_deref(), Some("主页"));
    }

    #[test]
    fn prefers_multi_char_quality() {
        assert!(label_quality_score("企业定制") > label_quality_score("制"));
        assert!(label_quality_score("产品配置") > label_quality_score("制"));
    }
}
