use crate::constants;
use crate::system_info::SystemInfo;

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

pub fn startup_payload(shell_path: &str) -> String {
    startup_payload_for_columns(shell_path, None)
}

pub fn startup_payload_for_columns(shell_path: &str, columns: Option<usize>) -> String {
    let info = SystemInfo::collect(shell_path);
    let art_lines: Vec<&str> = STARTUP_ART.lines().collect();
    let art_width = art_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default();

    let max_columns = columns.filter(|value| *value > 0).unwrap_or(usize::MAX);
    let side_gap = if max_columns >= 120 { 5 } else { 3 };
    let min_info_width = 42;

    if max_columns >= art_width + side_gap + min_info_width {
        let info_width = max_columns.saturating_sub(art_width + side_gap);
        let info_lines = build_info_lines(&info, Some(info_width));
        let total_rows = art_lines.len().max(info_lines.len());
        let mut lines = vec![String::new()];

        for index in 0..total_rows {
            let art_line = art_lines.get(index).copied().unwrap_or("");
            let gap = " ".repeat(art_width.saturating_sub(art_line.chars().count()) + side_gap);
            let info_line = info_lines.get(index).map(String::as_str).unwrap_or("");
            lines.push(format!("{ART}{art_line}{RESET}{gap}{info_line}"));
        }

        lines.push(String::new());
        return lines.join("\n");
    }

    let stacked_width = max_columns.saturating_sub(2);
    let info_lines = build_info_lines(&info, Some(stacked_width));
    let mut lines = vec![String::new()];

    if max_columns >= art_width.saturating_add(2) {
        lines.extend(art_lines.iter().map(|line| format!("{ART}{line}{RESET}")));
        lines.push(String::new());
    }

    lines.extend(info_lines);
    lines.push(String::new());
    lines.join("\n")
}

pub fn shell_wrapper_script(shell_path: &str) -> String {
    let payload = startup_payload(shell_path);
    format!("cat <<'TERMVOID_BANNER'\n{payload}\nTERMVOID_BANNER\nexec \"$0\" \"$@\"")
}

fn detail_line(label: &str, value: &str) -> String {
    format!("{LABEL}{label:<6}{RESET} {DIVIDER}│{RESET} {VALUE}{value}{RESET}")
}

fn build_info_lines(info: &SystemInfo, max_width: Option<usize>) -> Vec<String> {
    let title_plain = format!("{} // gengar rice shell", constants::APP_NAME);
    let subtitle_plain = format!("fedora • {} • {}", info.hostname, info.shell);

    let detail_width = max_width.unwrap_or(usize::MAX).saturating_sub(9);

    vec![
        color_title_line(&truncate_visible(&title_plain, max_width)),
        color_subtitle_line(&truncate_visible(&subtitle_plain, max_width)),
        String::new(),
        detail_line(
            "distro",
            &truncate_visible(&info.distro, Some(detail_width)),
        ),
        detail_line(
            "kernel",
            &truncate_visible(&info.kernel, Some(detail_width)),
        ),
        detail_line("gnome", &truncate_visible(&info.gnome, Some(detail_width))),
        detail_line("cpu", &truncate_visible(&info.cpu, Some(detail_width))),
        detail_line("ram", &truncate_visible(&info.ram, Some(detail_width))),
        detail_line("gpu", &truncate_visible(&info.gpu, Some(detail_width))),
        detail_line(
            "local",
            &truncate_visible(&info.local_ip, Some(detail_width)),
        ),
        detail_line(
            "public",
            &truncate_visible(&info.public_ip, Some(detail_width)),
        ),
        detail_line(
            "host",
            &truncate_visible(&info.hostname, Some(detail_width)),
        ),
        detail_line("shell", &truncate_visible(&info.shell, Some(detail_width))),
    ]
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
