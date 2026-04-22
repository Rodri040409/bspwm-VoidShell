use crate::util;
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static SYSTEM_INFO_CACHE: RefCell<BTreeMap<String, (u64, SystemInfo)>> =
        const { RefCell::new(BTreeMap::new()) };
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub distro: String,
    pub kernel: String,
    pub gnome: String,
    pub cpu: String,
    pub ram: String,
    pub gpu: String,
    pub local_ip: String,
    pub public_ip: String,
    pub hostname: String,
    pub shell: String,
}

impl SystemInfo {
    pub fn collect(shell_path: &str) -> Self {
        Self::collect_with_mode(shell_path, SystemInfoCollectionMode::Full)
    }

    pub fn collect_startup(shell_path: &str) -> Self {
        Self::collect_with_mode(shell_path, SystemInfoCollectionMode::Startup)
    }

    fn collect_with_mode(shell_path: &str, mode: SystemInfoCollectionMode) -> Self {
        let cache_key = util::shell_name(shell_path);
        let now = util::now_epoch_seconds();
        let max_age_seconds = match mode {
            SystemInfoCollectionMode::Startup => 12,
            SystemInfoCollectionMode::Full => 45,
        };

        if let Some(cached) = SYSTEM_INFO_CACHE.with(|cache| {
            cache
                .borrow()
                .get(&cache_key)
                .and_then(|(timestamp, value)| {
                    (now.saturating_sub(*timestamp) <= max_age_seconds).then(|| value.clone())
                })
        }) {
            return cached;
        }

        let mut collected = base_system_info(shell_path);
        collected.gnome = match mode {
            SystemInfoCollectionMode::Startup => detect_desktop_environment_label()
                .unwrap_or_else(|| "Entorno no detectado".to_string()),
            SystemInfoCollectionMode::Full => detect_gnome_version()
                .or_else(detect_desktop_environment_label)
                .unwrap_or_else(|| "Versión de GNOME no disponible".to_string()),
        };
        collected.gpu = match mode {
            SystemInfoCollectionMode::Startup => "GPU en segundo plano".to_string(),
            SystemInfoCollectionMode::Full => {
                detect_gpu().unwrap_or_else(|| "GPU no detectada".to_string())
            }
        };
        collected.public_ip = match mode {
            SystemInfoCollectionMode::Startup => util::cached_public_ip_cached_only(900)
                .unwrap_or_else(|| "No disponible".to_string()),
            SystemInfoCollectionMode::Full => {
                util::cached_public_ip(900).unwrap_or_else(|| "No disponible".to_string())
            }
        };

        SYSTEM_INFO_CACHE.with(|cache| {
            cache
                .borrow_mut()
                .insert(cache_key, (now, collected.clone()));
        });

        collected
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemInfoCollectionMode {
    Startup,
    Full,
}

fn base_system_info(shell_path: &str) -> SystemInfo {
    SystemInfo {
        distro: util::platform_display_name().unwrap_or_else(|| "Distro desconocida".to_string()),
        kernel: util::kernel_release().unwrap_or_else(|| "Kernel desconocido".to_string()),
        gnome: String::new(),
        cpu: util::cpu_description().unwrap_or_else(|| "CPU desconocida".to_string()),
        ram: util::mem_total_gib_portable().unwrap_or_else(|| "RAM desconocida".to_string()),
        gpu: String::new(),
        local_ip: util::primary_local_ip().unwrap_or_else(|| "No disponible".to_string()),
        public_ip: String::new(),
        hostname: util::hostname(),
        shell: util::shell_name(shell_path),
    }
}

fn detect_gnome_version() -> Option<String> {
    util::command_output("gnome-shell", &["--version"])
        .or_else(|| util::command_output("gnome-session", &["--version"]))
}

fn detect_desktop_environment_label() -> Option<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .or_else(|| std::env::var("DESKTOP_SESSION").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn detect_gpu() -> Option<String> {
    let mut discrete: Option<String> = None;
    let mut integrated: Option<String> = None;
    let mut fallback: Vec<String> = Vec::new();

    if let Some(nvidia) =
        util::command_output("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"])
    {
        for line in nvidia.lines() {
            register_gpu(line.trim(), &mut discrete, &mut integrated, &mut fallback);
        }
    }

    if let Some(lspci) = util::command_output("sh", &["-lc", "lspci | grep -E 'VGA|3D|Display'"]) {
        for line in lspci.lines() {
            let raw = line
                .split_once(": ")
                .map(|(_, value)| value.trim())
                .unwrap_or_else(|| line.trim());
            register_gpu(raw, &mut discrete, &mut integrated, &mut fallback);
        }
    }

    let mut labels = Vec::new();
    if let Some(label) = discrete {
        labels.push(label);
    }
    if let Some(label) = integrated {
        labels.push(label);
    }

    for label in fallback {
        if labels.len() >= 2 {
            break;
        }
        if !labels.iter().any(|existing| existing == &label) {
            labels.push(label);
        }
    }

    (!labels.is_empty()).then(|| labels.join(" + "))
}

fn register_gpu(
    raw: &str,
    discrete: &mut Option<String>,
    integrated: &mut Option<String>,
    fallback: &mut Vec<String>,
) {
    let Some(label) = summarize_gpu_label(raw) else {
        return;
    };

    let lowered = label.to_ascii_lowercase();
    if is_discrete_gpu(&lowered) {
        update_preferred_label(discrete, label);
        return;
    }

    if is_integrated_gpu(&lowered) {
        update_preferred_label(integrated, label);
        return;
    }

    if !fallback.iter().any(|entry| same_gpu(entry, &label)) {
        fallback.push(label);
    }
}

fn summarize_gpu_label(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_string();
    if value.is_empty() {
        return None;
    }

    if let Some((prefix, _)) = value.rsplit_once(" (rev ") {
        value = prefix.trim().to_string();
    }

    value = value.replace("Corporation ", "");
    value = value.replace("Advanced Micro Devices, Inc. [AMD/ATI]", "AMD");

    if value.to_ascii_lowercase().contains("intel") && value.contains("UHD Graphics") {
        return Some("Intel UHD Graphics".to_string());
    }

    if value.to_ascii_lowercase().contains("nvidia") {
        if let Some(bracketed) = extract_bracketed_name(&value) {
            let normalized = bracketed
                .replace("Max-Q / Mobile", "Laptop GPU")
                .replace("Max-Q", "Laptop GPU")
                .trim()
                .to_string();
            if normalized.contains("RTX")
                || normalized.contains("GTX")
                || normalized.contains("GeForce")
            {
                return Some(format!("NVIDIA {normalized}"));
            }
        }

        if value.contains("GeForce") {
            return Some(clean_gpu_spacing(&value));
        }
    }

    if value.to_ascii_lowercase().contains("amd")
        && let Some(bracketed) = extract_bracketed_name(&value)
    {
        return Some(format!("AMD {}", clean_gpu_spacing(&bracketed)));
    }

    Some(clean_gpu_spacing(&value))
}

fn extract_bracketed_name(value: &str) -> Option<String> {
    let start = value.find('[')? + 1;
    let end = value[start..].find(']')?;
    Some(value[start..start + end].trim().to_string())
}

fn clean_gpu_spacing(value: &str) -> String {
    value
        .replace("Max-Q / Mobile", "Laptop GPU")
        .replace("Max-Q", "Laptop GPU")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_discrete_gpu(value: &str) -> bool {
    value.contains("nvidia")
        || value.contains("geforce")
        || value.contains("rtx")
        || value.contains("gtx")
        || value.contains("radeon")
        || (value.contains("amd") && value.contains("graphics"))
}

fn is_integrated_gpu(value: &str) -> bool {
    value.contains("intel")
        || value.contains("uhd graphics")
        || value.contains("iris xe")
        || value.contains("integrated")
}

fn update_preferred_label(slot: &mut Option<String>, candidate: String) {
    match slot {
        Some(current) if score_gpu_label(&candidate) <= score_gpu_label(current) => {}
        Some(current) if same_gpu(current, &candidate) => *current = candidate,
        Some(current) if !same_gpu(current, &candidate) => {
            if candidate.len() < current.len() {
                *current = candidate;
            }
        }
        None => *slot = Some(candidate),
        _ => {}
    }
}

fn same_gpu(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();

    if left == right {
        return true;
    }

    let families = [
        "rtx 4090",
        "rtx 4080",
        "rtx 4070",
        "rtx 4060",
        "rtx 4050",
        "gtx",
        "uhd graphics",
        "iris xe",
        "radeon",
    ];
    for family in families {
        if left.contains(family) && right.contains(family) {
            return true;
        }
    }

    false
}

fn score_gpu_label(value: &str) -> i32 {
    let lowered = value.to_ascii_lowercase();
    let mut score = 0;

    if lowered.contains("rtx") || lowered.contains("gtx") {
        score += 5;
    }
    if lowered.contains("geforce") {
        score += 4;
    }
    if lowered.contains("laptop gpu") {
        score += 3;
    }
    if lowered.contains("uhd graphics") || lowered.contains("iris xe") {
        score += 3;
    }

    score * 10 - value.len() as i32
}
