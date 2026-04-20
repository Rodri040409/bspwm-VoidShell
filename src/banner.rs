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
    let info = SystemInfo::collect(shell_path);
    let art_lines: Vec<&str> = STARTUP_ART.lines().collect();
    let art_width = art_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default();

    let info_lines = vec![
        format!(
            "{TITLE}{}{RESET} {DIM}// gengar rice shell{RESET}",
            constants::APP_NAME
        ),
        format!("{DIM}fedora • {} • {}{RESET}", info.hostname, info.shell),
        String::new(),
        detail_line("distro", &info.distro),
        detail_line("kernel", &info.kernel),
        detail_line("gnome", &info.gnome),
        detail_line("cpu", &info.cpu),
        detail_line("ram", &info.ram),
        detail_line("gpu", &info.gpu),
        detail_line("local", &info.local_ip),
        detail_line("public", &info.public_ip),
        detail_line("host", &info.hostname),
        detail_line("shell", &info.shell),
    ];

    let total_rows = art_lines.len().max(info_lines.len());
    let mut lines = vec![String::new()];

    for index in 0..total_rows {
        let art_line = art_lines.get(index).copied().unwrap_or("");
        let gap = " ".repeat(art_width.saturating_sub(art_line.chars().count()) + 5);
        let info_line = info_lines.get(index).map(String::as_str).unwrap_or("");
        lines.push(format!("{ART}{art_line}{RESET}{gap}{info_line}"));
    }

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
