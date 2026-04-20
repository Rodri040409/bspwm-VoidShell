use crate::config::BannerInfoLayout;
use crate::constants;
use crate::system_info::SystemInfo;

const DEFAULT_BANNER_COLUMNS: usize = 60;
const SIDE_GAP: usize = 3;
const DETAIL_PREFIX_WIDTH: usize = 9;
const MIN_SIDE_INFO_WIDTH: usize = 14;
const SIDE_BY_SIDE_INFO_CAP: usize = 32;
const STACKED_WIDTH_CAP: usize = 54;

pub const STARTUP_ART: &str = r#"⠀⠀⠀⠀⠀⠀⣀⡀⠀⠀⣀⣤⣶⣾⣿⣿⣷⣶⣤⣀⠀⠀⣀⣀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠜⠉⣿⡆⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⢰⣿⠉⠃⠀⠀⠀⠀⠀
⠀⢀⣤⣴⣦⣄⣴⠟⣸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡎⢻⣦⣠⣴⣦⣄⠀⠀
⠀⡞⠁⣠⣾⢿⣧⠀⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠀⣽⡿⣷⣄⠈⢷⠀
⠀⣠⣾⠟⠁⢸⣿⠀⠘⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠁⠀⣿⡇⠈⠻⣷⣄⠀
⣰⡿⠁⠀⢀⣾⣏⣾⣄⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣇⣰⣷⣹⣷⠀⠀⠈⢿⣆
⣿⡇⠀⢠⣾⠏⢸⣿⣿⣿⣿⠋⢻⣿⣿⣿⣿⡟⠙⣿⣿⣿⣿⡇⠹⣷⡀⠀⢸⣿
⠹⣿⣴⡿⠋⠀⠈⠛⠉⣹⣿⣦⣄⡹⣿⣿⣋⣠⣶⣿⣏⠉⠛⠁⠀⠙⢿⣦⣿⠏
⠀⣸⣿⠿⠿⣿⣾⣿⡿⠿⣿⣿⣿⣿⡆⢰⣿⣿⣿⣿⠿⢿⣿⣶⣿⠿⠿⣻⣇⠀
⠀⣿⡇⢀⣴⣶⣤⣀⣴⣿⠿⣻⡿⣿⣧⣾⣿⢿⣟⠿⣿⣦⣀⣤⣶⣦⠀⢸⣿⠀
⠀⢿⣧⠈⠃⢀⣵⣿⡋⠁⢀⣿⡷⣿⡇⢻⣿⣿⣿⡀⠈⢛⣿⣮⡀⠘⠀⣼⡟⠀
⠀⠈⠻⣷⣤⣟⣋⣿⣧⣴⡿⠋⠀⣿⡇⢸⣿⠀⠙⢿⣦⣼⣿⣙⣻⣤⣾⠟⠁⠀
⠀⠀⠀⠈⢽⣿⠛⢻⣏⢉⣤⣶⣶⣿⠁⠈⣿⣶⣶⣤⡉⣽⡟⠛⣿⡏⠁⠀⠀⠀
⠀⠀⠀⠀⠈⠿⣷⣾⣾⣟⣉⣠⣿⢿⡇⢸⠿⣿⣄⣙⣻⣷⣷⣾⠿⠁⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠙⠻⠿⠛⢁⡼⠃⠘⢦⡈⠛⠿⠟⠃⠀⠀⠀⠀⠀⠀⠀⠀"#;

const RESET: &str = "\x1b[0m";
const TITLE: &str = "\x1b[1;38;2;217;169;255m";
const ART: &str = "\x1b[38;2;185;140;255m";
const LABEL: &str = "\x1b[38;2;160;128;210m";
const VALUE: &str = "\x1b[38;2;241;235;255m";
const DIVIDER: &str = "\x1b[38;2;110;93;150m";
const DIM: &str = "\x1b[38;2;151;137;179m";

pub fn startup_payload_for_columns(
    shell_path: &str,
    columns: Option<usize>,
    layout: BannerInfoLayout,
) -> String {
    let info = SystemInfo::collect(shell_path);
    let art_lines: Vec<&str> = STARTUP_ART.lines().collect();
    let art_width = art_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default();

    let max_columns = columns
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_BANNER_COLUMNS);
    let side_by_side_width = art_width + SIDE_GAP + MIN_SIDE_INFO_WIDTH;

    if matches!(layout, BannerInfoLayout::Right) && max_columns >= side_by_side_width {
        let info_width = max_columns
            .saturating_sub(art_width + SIDE_GAP)
            .min(SIDE_BY_SIDE_INFO_CAP);
        let info_lines = build_info_lines(&info, Some(info_width), true);
        let total_rows = art_lines.len().max(info_lines.len());
        let mut lines = vec![String::new()];

        for index in 0..total_rows {
            let art_line = art_lines.get(index).copied().unwrap_or("");
            let gap = " ".repeat(art_width.saturating_sub(art_line.chars().count()) + SIDE_GAP);
            let info_line = info_lines.get(index).map(String::as_str).unwrap_or("");
            lines.push(format!("{ART}{art_line}{RESET}{gap}{info_line}"));
        }

        lines.push(String::new());
        return lines.join("\n");
    }

    let stacked_width = max_columns.saturating_sub(2).min(STACKED_WIDTH_CAP);
    let info_lines = build_info_lines(&info, Some(stacked_width), true);
    let mut lines = vec![String::new()];

    if max_columns >= art_width.saturating_add(2) {
        lines.extend(art_lines.iter().map(|line| format!("{ART}{line}{RESET}")));
        lines.push(String::new());
    }

    lines.extend(info_lines);
    lines.push(String::new());
    lines.join("\n")
}

fn detail_line(label: &str, value: &str) -> String {
    format!("{LABEL}{label:<6}{RESET} {DIVIDER}│{RESET} {VALUE}{value}{RESET}")
}

fn detail_continuation_line(value: &str) -> String {
    format!("       {DIVIDER}│{RESET} {VALUE}{value}{RESET}")
}

fn build_info_lines(
    info: &SystemInfo,
    max_width: Option<usize>,
    wrap_details: bool,
) -> Vec<String> {
    let title_plain = format!("{} // gengar rice shell", constants::APP_NAME);
    let subtitle_plain = format!("fedora • {} • {}", info.hostname, info.shell);

    let detail_width = max_width
        .unwrap_or(usize::MAX)
        .saturating_sub(DETAIL_PREFIX_WIDTH)
        .max(1);
    let mut lines = vec![
        color_title_line(&truncate_visible(&title_plain, max_width)),
        color_subtitle_line(&truncate_visible(&subtitle_plain, max_width)),
        String::new(),
    ];

    append_detail_lines(
        &mut lines,
        "distro",
        &info.distro,
        detail_width,
        wrap_details,
    );
    append_detail_lines(
        &mut lines,
        "kernel",
        &info.kernel,
        detail_width,
        wrap_details,
    );
    append_detail_lines(
        &mut lines,
        "gnome",
        &info.gnome,
        detail_width,
        wrap_details,
    );
    append_detail_lines(&mut lines, "cpu", &info.cpu, detail_width, wrap_details);
    append_detail_lines(&mut lines, "ram", &info.ram, detail_width, wrap_details);
    append_detail_lines(&mut lines, "gpu", &info.gpu, detail_width, wrap_details);
    append_detail_lines(
        &mut lines,
        "local",
        &info.local_ip,
        detail_width,
        wrap_details,
    );
    append_detail_lines(
        &mut lines,
        "public",
        &info.public_ip,
        detail_width,
        wrap_details,
    );
    append_detail_lines(
        &mut lines,
        "host",
        &info.hostname,
        detail_width,
        wrap_details,
    );
    append_detail_lines(
        &mut lines,
        "shell",
        &info.shell,
        detail_width,
        wrap_details,
    );

    lines
}

fn append_detail_lines(
    lines: &mut Vec<String>,
    label: &str,
    value: &str,
    detail_width: usize,
    wrap_details: bool,
) {
    if !wrap_details {
        lines.push(detail_line(label, &truncate_visible(value, Some(detail_width))));
        return;
    }

    let wrapped = wrap_visible(value, detail_width);
    let mut iter = wrapped.iter();
    let first = iter.next().map(String::as_str).unwrap_or("");
    lines.push(detail_line(label, first));
    for chunk in iter {
        lines.push(detail_continuation_line(chunk));
    }
}

fn color_title_line(value: &str) -> String {
    if let Some((app_name, suffix)) = value.split_once(" // ") {
        return format!("{TITLE}{app_name}{RESET} {DIM}// {suffix}{RESET}");
    }
    format!("{TITLE}{value}{RESET}")
}

fn color_subtitle_line(value: &str) -> String {
    format!("{DIM}{value}{RESET}")
}

fn truncate_visible(value: &str, max_width: Option<usize>) -> String {
    let Some(max_width) = max_width.filter(|value| *value > 0) else {
        return value.to_string();
    };

    if value.chars().count() <= max_width {
        return value.to_string();
    }

    if max_width <= 3 {
        return value.chars().take(max_width).collect();
    }

    let visible = max_width.saturating_sub(3);
    let mut truncated: String = value.chars().take(visible).collect();
    truncated.push_str("...");
    truncated
}

fn wrap_visible(value: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![String::new()];
    }

    let normalized = value.trim();
    if normalized.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in normalized.split_whitespace() {
        if word.chars().count() > max_width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            push_word_chunks(word, max_width, &mut lines);
            continue;
        }

        let current_len = current.chars().count();
        let word_len = word.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if current_len + 1 + word_len <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

fn push_word_chunks(word: &str, max_width: usize, output: &mut Vec<String>) {
    let mut chunk = String::new();
    for ch in word.chars() {
        chunk.push(ch);
        if chunk.chars().count() >= max_width {
            output.push(chunk);
            chunk = String::new();
        }
    }

    if !chunk.is_empty() {
        output.push(chunk);
    }
}
