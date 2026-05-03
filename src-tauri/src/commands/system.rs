use serde::{Serialize, Deserialize};
use sysinfo::{System, Disks, CpuRefreshKind, MemoryRefreshKind, RefreshKind};
use regex::Regex;
use tauri::AppHandle;
use crate::state::get_resource_dir;
use std::process::Command;
use std::sync::OnceLock;
use std::collections::HashMap;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// --- Constantes y Caché ---
const CREATE_NO_WINDOW: u32 = 0x08000000;
static RE_INTEL: OnceLock<Regex> = OnceLock::new();
static RE_INTEL_CORE: OnceLock<Regex> = OnceLock::new();
static RE_RYZEN: OnceLock<Regex> = OnceLock::new();
static CACHED_SPECS: OnceLock<SystemSpecs> = OnceLock::new();
static NVIDIA_POWER_LIMIT: OnceLock<Option<String>> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemSpecs {
    pub brand: String,
    pub model: String,
    pub processor: String,
    pub cores: usize,
    pub threads: usize,
    pub gen: String,
    pub vendor: String,
    pub ram: String,
    #[serde(rename = "ramType")]
    pub ram_type: String,
    pub gpu: String,
    pub storage: String,
    pub display: String,
    pub os: String,
}

// --- COMANDO PRINCIPAL ---

#[tauri::command]
pub async fn get_system_specs() -> Result<SystemSpecs, String> {
    if let Some(cached) = CACHED_SPECS.get() {
        return Ok(cached.clone());
    }

    let sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
    );

    // 1. CPU
    let cpu = sys.cpus().first().ok_or("No se detectó CPU")?;
    let proc_name = cpu.brand().trim().replace("(R)", "").replace("(TM)", "").replace("  ", " ");
    let vendor = if proc_name.contains("Intel") { "Intel" } else if proc_name.contains("AMD") { "AMD" } else { "Generic" };
    let gen = detect_generation(&proc_name);

    // 2. RAM & Storage
    let ram_display = get_ram_display(sys.total_memory());
    let storage_display = get_storage_info();

    // 3. WMI Details (Síncrono para evitar errores de Send/Sync con COM)
    let wmi = get_wmi_details().unwrap_or_else(|_| default_wmi_fallback());

    let specs = SystemSpecs {
        brand: wmi.brand,
        model: wmi.model,
        processor: proc_name,
        cores: sys.physical_core_count().unwrap_or(0),
        threads: sys.cpus().len(),
        gen,
        vendor: vendor.to_string(),
        ram: ram_display,
        ram_type: wmi.ram_type,
        gpu: wmi.gpu,
        storage: storage_display,
        display: wmi.display,
        os: wmi.os,
    };

    let _ = CACHED_SPECS.set(specs.clone());
    Ok(specs)
}

// --- ESTRUCTURAS INTERNAS ---

struct WmiData {
    brand: String,
    model: String,
    gpu: String,
    display: String,
    ram_type: String,
    os: String,
}

fn default_wmi_fallback() -> WmiData {
    WmiData {
        brand: "PC Generico".to_string(),
        model: "PC Desktop".to_string(),
        gpu: "Gráficos Integrados".to_string(),
        display: "1920 x 1080".to_string(),
        ram_type: "DDR4".to_string(),
        os: System::name().unwrap_or_else(|| "Windows".to_string()).replace("Microsoft ", ""),
    }
}

// --- FUNCIONES DE DETECCIÓN (Síncronas para evitar errores de hilos) ---

#[cfg(windows)]
fn get_wmi_details() -> Result<WmiData, Box<dyn std::error::Error>> {
    use wmi::{COMLibrary, WMIConnection};
    let com_con = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_con)?;

    let (brand, model) = detect_brand_and_model(&wmi_con)?;
    
    let video_results: Vec<HashMap<String, serde_json::Value>> = wmi_con
        .raw_query("SELECT Name, CurrentHorizontalResolution, CurrentVerticalResolution FROM Win32_VideoController")
        .unwrap_or_default();

    let gpu = detect_best_gpu(&video_results);
    let display = format_display_resolution(&wmi_con, &video_results);
    let ram_type = detect_ram_type(&wmi_con);
    let os = detect_os_version(&wmi_con);

    Ok(WmiData { brand, model, gpu, display, ram_type, os })
}

#[cfg(windows)]
fn detect_brand_and_model(wmi: &wmi::WMIConnection) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut brand = "PC Generico".to_string();
    let mut model = "PC Desktop".to_string();
    
    if let Ok(results) = wmi.raw_query("SELECT Manufacturer, Model FROM Win32_ComputerSystem") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.first() {
            brand = res.get("Manufacturer").and_then(|v| v.as_str()).unwrap_or("PC Generico").trim().to_string();
            model = res.get("Model").and_then(|v| v.as_str()).unwrap_or("PC Desktop").trim().to_string();
        }
    }

    brand = clean_brand_name(&brand);

    if is_generic_info(&brand, &model) {
        if let Ok(mb_results) = wmi.raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard") {
            let mb_results: Vec<HashMap<String, serde_json::Value>> = mb_results;
            if let Some(res) = mb_results.first() {
                brand = clean_brand_name(res.get("Manufacturer").and_then(|v| v.as_str()).unwrap_or("PC Desktop"));
                let product = res.get("Product").and_then(|v| v.as_str()).unwrap_or("").trim();
                if !product.is_empty() && product != "Default string" {
                    model = product.to_string();
                }
            }
        }
    }

    let model_final = if brand.to_uppercase().contains("VIRTUALBOX") {
        "Virtual Machine".to_string()
    } else {
        refine_model_name(&brand, &model)
    };

    Ok((brand, model_final))
}

#[cfg(windows)]
fn detect_best_gpu(video_results: &[HashMap<String, serde_json::Value>]) -> String {
    let mut best_gpu = "Gráficos Integrados".to_string();
    let mut best_score = 0;

    for res in video_results {
        if let Some(raw_name) = res.get("Name").and_then(|v| v.as_str()) {
            let name = raw_name.replace("(R)", "").replace("(TM)", "").replace("  ", " ").trim().to_string();
            let score = rate_gpu(&name);

            if score > best_score {
                best_score = score;
                best_gpu = name;
            }
        }
    }

    if best_score >= 10 {
        if let Some(watts) = get_nvidia_watts() {
            best_gpu = format!("{} {}W", best_gpu, watts);
        }
    }

    best_gpu
}

fn rate_gpu(name: &str) -> i32 {
    let name_up = name.to_uppercase();
    if name_up.contains("NVIDIA") || name_up.contains("RTX") || name_up.contains("GTX") { 10 }
    else if name_up.contains("RX ") { 8 }
    else if name_up.contains("ARC") { 5 }
    else if name_up.contains("UHD") || name_up.contains("RADEON") || name_up.contains("IRIS") { 2 }
    else { 1 }
}

fn get_nvidia_watts() -> Option<String> {
    NVIDIA_POWER_LIMIT.get_or_init(|| {
        let script = r#"$val = (nvidia-smi -q -d POWER | Select-String "Max Power Limit" | Where-Object { $_ -notmatch "N/A" }); if ($val) { [int][float]($val.ToString().Split(':')[1].Replace('W','').Trim()) }"#;
        if let Ok(output) = Command::new("powershell").args(["-NoProfile", "-Command", script]).creation_flags(CREATE_NO_WINDOW).output() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() && stdout.chars().all(|c| c.is_numeric()) {
                return Some(stdout);
            }
        }
        None
    }).clone()
}

#[cfg(windows)]
fn format_display_resolution(wmi: &wmi::WMIConnection, video_results: &[HashMap<String, serde_json::Value>]) -> String {
    let mut max_h = 0;
    let mut max_v = 0;

    for res in video_results {
        let h = res.get("CurrentHorizontalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
        let v = res.get("CurrentVerticalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
        if h > max_h { max_h = h; max_v = v; }
    }

    if max_h == 0 {
        if let Ok(results) = wmi.raw_query("SELECT ScreenWidth, ScreenHeight FROM Win32_DesktopMonitor") {
            let results: Vec<HashMap<String, serde_json::Value>> = results;
            if let Some(res) = results.first() {
                max_h = res.get("ScreenWidth").and_then(|v| v.as_u64()).unwrap_or(0);
                max_v = res.get("ScreenHeight").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }

    if max_h == 0 { (max_h, max_v) = (1920, 1080); }

    let label = match (max_h, max_v) {
        (1920, 1080) => " (Full HD)",
        (1920, 1200) => " (WUXGA)",
        (2560, 1440) => " (2K QHD)",
        (2560, 1600) => " (QHD+)",
        (2880, 1800) => " (2.8K)",
        (3000, 2000) => " (3K)",
        (3200, 2000) => " (3.2K)",
        (3840, 2160) => " (4K UHD)",
        (3840, 2400) => " (UHD+)",
        (1366, 768)  => " (HD)",
        _ => ""
    };

    format!("{} x {}{}", max_h, max_v, label)
}

#[cfg(windows)]
fn detect_ram_type(wmi: &wmi::WMIConnection) -> String {
    if let Ok(results) = wmi.raw_query("SELECT SMBIOSMemoryType FROM Win32_PhysicalMemory") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.first() {
            return match res.get("SMBIOSMemoryType").and_then(|v| v.as_u64()).unwrap_or(0) {
                26 => "DDR4",
                30 | 31 => "LPDDR4",
                34 => "DDR5",
                35 => "LPDDR5",
                _ => "DDR4"
            }.to_string();
        }
    }
    "DDR4".to_string()
}

#[cfg(windows)]
fn detect_os_version(wmi: &wmi::WMIConnection) -> String {
    if let Ok(results) = wmi.raw_query("SELECT Caption FROM Win32_OperatingSystem") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.into_iter().next() {
            return res.get("Caption").and_then(|v| v.as_str()).unwrap_or("Windows").replace("Microsoft ", "").trim().to_string();
        }
    }
    "Windows".to_string()
}

// --- UTILIDADES ---

fn get_storage_info() -> String {
    let disks = Disks::new_with_refreshed_list();
    let total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();
    let total_gb = total_bytes as f64 / 1_000_000_000.0;
    
    if total_gb >= 872.0 {
        let tb = total_gb / 1000.0;
        let fract = tb.fract();
        if fract >= 0.872 { format!("{:.0}TB SSD", tb.ceil()) }
        else if fract <= 0.128 { format!("{:.0}TB SSD", tb.floor()) }
        else { format!("{:.1}TB SSD", tb).replace(".", ",") }
    } else {
        let rounded = (total_gb / 128.0).round() * 128.0;
        format!("{:.0}GB SSD", if rounded == 0.0 { total_gb.round() } else { rounded })
    }
}

fn detect_generation(name: &str) -> String {
    let re_intel = RE_INTEL.get_or_init(|| Regex::new(r"i[3579]-(\d+)").unwrap());
    let re_intel_core = RE_INTEL_CORE.get_or_init(|| Regex::new(r"Core\s+[3579]\s+(\d)").unwrap());
    let re_ryzen = RE_RYZEN.get_or_init(|| Regex::new(r"Ryzen\s+[3579]\s+(\d)(\d{2,3})").unwrap());

    if let Some(cap) = re_intel.captures(name) { format!("{}a Gen", &cap[1]) }
    else if let Some(cap) = re_intel_core.captures(name) { format!("Serie {}", &cap[1]) }
    else if name.contains("Ultra") { "Core Ultra".to_string() }
    else if name.contains("Ryzen AI") { "Ryzen AI".to_string() }
    else if let Some(cap) = re_ryzen.captures(name) { 
        if cap[2].len() == 2 {
            format!("{}00 Series", &cap[1])
        } else {
            format!("{}000 Series", &cap[1])
        }
    }
    else { "Desconocida".to_string() }
}

fn get_ram_display(total_bytes: u64) -> String {
    let gb = total_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    let size = (gb / 2.0).round() * 2.0;
    format!("{:.0}GB", if size == 0.0 { gb.round() } else { size })
}

fn clean_brand_name(raw: &str) -> String {
    let r = raw.to_uppercase();
    if r.contains("ASUSTEK") { "ASUS" }
    else if r.contains("HEWLETT-PACKARD") || r.contains("HP") { "HP" }
    else if r.contains("SAMSUNG") { "Samsung" }
    else if r.contains("DELL") { "Dell" }
    else if r.contains("LENOVO") { "Lenovo" }
    else if r.contains("ASROCK") { "ASRock" }
    else if r.contains("GIGABYTE") { "Gigabyte" }
    else if r.contains("MSI") || r.contains("MICRO-STAR") { "MSI" }
    else if r.contains("VIRTUALBOX") { "VirtualBox" }
    else { raw.trim() }.to_string()
}

fn is_generic_info(brand: &str, model: &str) -> bool {
    let b = brand.to_uppercase();
    let m = model.to_uppercase();
    b.contains("TO BE FILLED") || b.contains("O.E.M") || b.is_empty() || m.contains("SYSTEM PRODUCT") || m.contains("DEFAULT STRING") || m == b
}

fn refine_model_name(brand: &str, model: &str) -> String {
    let noise = ["ASUSTEK", "COMPUTER", "INC", "CORP", "CORPORATION", "LTD", "SYSTEMS", "PRODUCT", "NAME", "LAPTOP"];
    
    let mut clean = model
        .replace("_", " ")
        .replace("ASUSLaptop", " Laptop ")
        .replace("-", " ")
        .trim()
        .to_string();

    while clean.contains("  ") { clean = clean.replace("  ", " "); }

    let words: Vec<&str> = clean.split_whitespace().collect();
    let mut unique_words: Vec<String> = Vec::new();
    let brand_up = brand.to_uppercase();

    for &word in &words {
        let word_up = word.to_uppercase().replace(".", "");
        if noise.contains(&word_up.as_str()) { continue; }
        if word_up == brand_up || word_up.contains(&brand_up) { continue; }

        if let Some(last) = unique_words.last() {
            let last_up = last.to_uppercase();
            if word_up == last_up || word_up.starts_with(&last_up) || last_up.starts_with(&word_up) {
                if word.len() > last.len() {
                    unique_words.pop();
                    unique_words.push(word.to_string());
                }
                continue;
            }
        }
        unique_words.push(word.to_string());
    }

    let result_model = unique_words.join(" ");
    if result_model.is_empty() { brand.to_string() } else { format!("{} {}", brand, result_model) }
}

// --- OTROS COMANDOS ---

#[tauri::command]
pub fn get_video_path(app: AppHandle) -> String {
    get_resource_dir(&app).to_string_lossy().into_owned()
}

#[tauri::command]
pub fn set_max_brightness() {
    let script = r#"
        try {
            (Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, 100)
            powercfg /setacvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0
            powercfg /setdcvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0
            powercfg /x -hibernate-timeout-ac 0
            powercfg /x -standby-timeout-ac 0
            powercfg /x -monitor-timeout-ac 0
            powercfg /s SCHEME_CURRENT
        } catch {}
    "#;
    let _ = Command::new("powershell.exe").args(["-ExecutionPolicy", "Bypass", "-Command", script]).creation_flags(CREATE_NO_WINDOW).spawn();
}

#[cfg(not(windows))]
fn get_wmi_details() -> Result<WmiData, Box<dyn std::error::Error>> {
    Ok(default_wmi_fallback())
}
