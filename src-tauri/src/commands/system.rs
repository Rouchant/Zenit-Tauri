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
static RE_N_SERIES: OnceLock<Regex> = OnceLock::new();
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

/// Comando principal invocado por el frontend para obtener todas las especificaciones del sistema.
/// Utiliza un sistema de caché para que la detección solo ocurra una vez por ejecución.
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

    // 1. Detalles profundos vía WMI (Marca, Modelo, GPU, Resolución, Almacenamiento, CPU, etc.)
    let wmi = get_wmi_details().unwrap_or_else(|_| default_wmi_fallback());

    // 2. Detección y limpieza de CPU
    let sysinfo_cpu_name = sys.cpus().first().map(|c| c.brand().trim()).unwrap_or("");
    
    // Si sysinfo devuelve "Virtual CPU" o genérico, o si WMI provee el nombre físico real del hardware, se usa WMI
    let raw_proc_name = match &wmi.cpu_name {
        Some(wmi_name) if !wmi_name.is_empty() && (sysinfo_cpu_name.to_lowercase().contains("virtual") || !wmi_name.to_lowercase().contains("virtual")) => {
            wmi_name.clone()
        }
        _ if !sysinfo_cpu_name.is_empty() => sysinfo_cpu_name.to_string(),
        _ => wmi.cpu_name.clone().unwrap_or_else(|| "Procesador Genérico".to_string()),
    };

    let proc_name = clean_processor_name(&raw_proc_name);
    let vendor = if proc_name.to_uppercase().contains("INTEL") {
        "Intel"
    } else if proc_name.to_uppercase().contains("AMD") {
        "AMD"
    } else if proc_name.to_lowercase().contains("snapdragon") || proc_name.to_lowercase().contains("qualcomm") {
        "Snapdragon"
    } else {
        "Generic"
    };
    let gen = detect_generation(&proc_name);

    // 3. RAM (Formateo comercial)
    let ram_display = get_ram_display(sys.total_memory());

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
        storage: wmi.storage,
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
    storage: String,
    os: String,
    cpu_name: Option<String>,
}

fn default_wmi_fallback() -> WmiData {
    WmiData {
        brand: "PC Generico".to_string(),
        model: "PC Desktop".to_string(),
        gpu: "Gráficos Integrados".to_string(),
        display: "1920 x 1080".to_string(),
        ram_type: "DDR4".to_string(),
        storage: get_storage_info_fallback(),
        os: System::name().unwrap_or_else(|| "Windows".to_string()).replace("Microsoft ", ""),
        cpu_name: None,
    }
}

// --- FUNCIONES DE DETECCIÓN (Síncronas para estabilidad de hilos) ---

#[cfg(windows)]
/// Orquesta todas las consultas WMI para obtener la información detallada de Windows.
fn get_wmi_details() -> Result<WmiData, Box<dyn std::error::Error>> {
    use wmi::{COMLibrary, WMIConnection};
    let com_con = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_con)?;

    let (brand, model) = detect_brand_and_model(&wmi_con)?;
    
    let video_results: Vec<HashMap<String, serde_json::Value>> = wmi_con
        .raw_query("SELECT Name, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate FROM Win32_VideoController")
        .unwrap_or_default();

    let gpu = detect_best_gpu(&video_results);
    let display = format_display_resolution(&wmi_con, &video_results);
    let ram_type = detect_ram_type(&wmi_con);
    let os = detect_os_version(&wmi_con);
    let cpu_name = detect_cpu_name_wmi(&wmi_con);
    
    // Detección de almacenamiento excluyendo USB
    let storage = get_storage_info_wmi(&wmi_con).unwrap_or_else(|| get_storage_info_fallback());

    Ok(WmiData { brand, model, gpu, display, ram_type, storage, os, cpu_name })
}

#[cfg(windows)]
/// Consulta la clase Win32_Processor en WMI para obtener el nombre oficial del procesador.
fn detect_cpu_name_wmi(wmi: &wmi::WMIConnection) -> Option<String> {
    if let Ok(results) = wmi.raw_query("SELECT Name FROM Win32_Processor") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.first() {
            if let Some(name) = res.get("Name").and_then(|v| v.as_str()) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

#[cfg(windows)]
/// Detecta la Marca y el Modelo del equipo. 
/// Tiene un sistema de fallbacks: si no lo encuentra en ComputerSystem, lo busca en la Placa Base (BaseBoard).
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

    // Fallback a Placa Base si la info inicial es genérica (típico en PCs armados o VMs)
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

    // Refinado final del nombre del modelo para quitar ruido técnico
    let model_final = if brand.to_uppercase().contains("VIRTUALBOX") {
        "Virtual Machine".to_string()
    } else {
        refine_model_name(&brand, &model)
    };

    Ok((brand, model_final))
}

#[cfg(windows)]
/// Evalúa todas las GPUs instaladas y elige la "mejor" (Dedicada > Integrada).
/// Si la elegida es NVIDIA, intenta obtener su TGP (Wattage) mediante nvidia-smi.
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

/// Asigna una puntuación a la GPU según su fabricante y tipo para priorizar dedicadas sobre integradas.
fn rate_gpu(name: &str) -> i32 {
    let name_up = name.to_uppercase();
    if name_up.contains("NVIDIA") || name_up.contains("RTX") || name_up.contains("GTX") { 10 }
    // "RX " con espacio (ej: "Radeon RX 6600") o pegado a número (ej: "RX6600", "RX5700")
    else if name_up.contains("RX ") || name_up.contains("RX6") || name_up.contains("RX7") || name_up.contains("RX5") || name_up.contains("RX4") { 8 }
    else if name_up.contains("ARC") { 5 }
    else if name_up.contains("UHD") || name_up.contains("RADEON") || name_up.contains("IRIS") { 2 }
    else { 1 }
}

/// Consulta nvidia-smi para obtener el límite máximo de potencia (Wattage) de la tarjeta.
fn get_nvidia_watts() -> Option<String> {
    NVIDIA_POWER_LIMIT.get_or_init(|| {
        let script = r#"$val = (nvidia-smi -q -d POWER | Select-String "Max Power Limit" | Where-Object { $_ -notmatch "N/A" }); if ($val) { [int][float]($val.ToString().Split(':')[1].Replace('W','').Trim()) }"#;
        if let Ok(output) = Command::new("powershell").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script]).creation_flags(CREATE_NO_WINDOW).output() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() && stdout.chars().all(|c| c.is_numeric()) {
                return Some(stdout);
            }
        }
        None
    }).clone()
}

#[cfg(windows)]
/// Determina la resolución de pantalla actual buscando el valor máximo entre todos los controladores.
/// Incluye etiquetas comerciales como (Full HD), (2K), etc.
fn approximate_refresh_rate(hz: u64) -> u64 {
    let commercial_rates = [60, 75, 90, 100, 120, 144, 165, 240, 360, 480, 540];
    let mut closest = hz;
    let mut min_diff = i64::MAX;
    for &r in &commercial_rates {
        let diff = (hz as i64 - r as i64).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = r;
        }
    }
    if min_diff <= 10 {
        closest
    } else {
        hz
    }
}

fn format_display_resolution(wmi: &wmi::WMIConnection, video_results: &[HashMap<String, serde_json::Value>]) -> String {
    let mut max_h = 0;
    let mut max_v = 0;
    let mut max_hz = 0;

    for res in video_results {
        let h = res.get("CurrentHorizontalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
        let v = res.get("CurrentVerticalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
        let hz = res.get("CurrentRefreshRate").and_then(|v| v.as_u64()).unwrap_or(0);
        if h > max_h {
            max_h = h;
            max_v = v;
            max_hz = hz;
        }
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

    if max_h == 0 { max_h = 1920; max_v = 1080; }

    let label = match (max_h, max_v) {
        (1920, 1080) => " (Full HD)",
        (1920, 1200) => " (WUXGA)",
        (2560, 1440) => " (2K QHD)",
        (2560, 1600) => " (2.5K)",
        (2880, 1800) => " (3K)",
        (3000, 2000) => " (3K)",
        (3200, 2000) => " (3.2K)",
        (3840, 2160) => " (4K UHD)",
        (3840, 2400) => " (UHD+)",
        (1366, 768)  => " (HD)",
        _ => ""
    };

    if max_hz > 0 {
        let hz_val = approximate_refresh_rate(max_hz);
        format!("{} x {}{} - {}Hz", max_h, max_v, label, hz_val)
    } else {
        format!("{} x {}{}", max_h, max_v, label)
    }
}

#[cfg(windows)]
/// Detecta el tipo de tecnología de RAM (DDR4, DDR5, LPDDR5, etc.) y su velocidad usando SMBIOS y la propiedad Speed.
fn detect_ram_type(wmi: &wmi::WMIConnection) -> String {
    if let Ok(results) = wmi.raw_query("SELECT SMBIOSMemoryType, ConfiguredClockSpeed FROM Win32_PhysicalMemory") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.first() {
            let mut ram_type = match res.get("SMBIOSMemoryType").and_then(|v| v.as_u64()).unwrap_or(0) {
                20 => "DDR",
                21 | 22 => "DDR2",
                24 => "DDR3",
                26 => "DDR4",
                29 => "LPDDR3",
                30 | 31 => "LPDDR4",
                34 => "DDR5",
                35 => "LPDDR5",
                _ => "DDR4"
            };
            let speed = match res.get("ConfiguredClockSpeed") {
                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
                Some(serde_json::Value::String(s)) => s.parse::<u64>().unwrap_or(0),
                _ => 0,
            };
            if ram_type == "LPDDR5" && speed >= 6000 {
                ram_type = "LPDDR5X";
            }
            if speed > 0 {
                return format!("{} - {} MT/s", ram_type, speed);
            }
            return ram_type.to_string();
        }
    }
    "DDR4".to_string()
}

#[cfg(windows)]
/// Obtiene el nombre legible de la versión de Windows instalada.
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

/// Formatea los bytes totales para representarlos con capacidades comerciales de Retail (SSD/almacenamiento).
fn format_bytes_commercial(total_bytes: u64) -> String {
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

/// Fallback de almacenamiento usando la librería sysinfo para multiplataforma o si WMI falla.
fn get_storage_info_fallback() -> String {
    let disks = Disks::new_with_refreshed_list();
    let total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();
    format_bytes_commercial(total_bytes)
}

#[cfg(windows)]
/// Obtiene el almacenamiento comercial sumando el tamaño de todos los discos internos
/// mediante WMI, ignorando discos con interfaz USB o id de dispositivo USBSTOR.
fn get_storage_info_wmi(wmi: &wmi::WMIConnection) -> Option<String> {
    if let Ok(results) = wmi.raw_query("SELECT Size, InterfaceType, PNPDeviceID FROM Win32_DiskDrive") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        let mut total_bytes: u64 = 0;
        let mut found_any = false;

        for res in results {
            let interface_type = res.get("InterfaceType").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
            let pnp_device_id = res.get("PNPDeviceID").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
            
            // Ignorar explícitamente dispositivos USB y USBSTOR
            if interface_type.contains("USB") || pnp_device_id.contains("USBSTOR") {
                continue;
            }

            let size_val = res.get("Size");
            let size_bytes: u64 = match size_val {
                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
                Some(serde_json::Value::String(s)) => s.parse::<u64>().unwrap_or(0),
                _ => 0,
            };

            if size_bytes > 0 {
                total_bytes += size_bytes;
                found_any = true;
            }
        }

        if found_any && total_bytes > 0 {
            return Some(format_bytes_commercial(total_bytes));
        }
    }
    None
}

/// Identifica la generación del procesador mediante expresiones regulares aplicadas al nombre del modelo.
fn detect_generation(name: &str) -> String {
    let n = name.to_lowercase();
    let re_intel = RE_INTEL.get_or_init(|| Regex::new(r"i[3579]-(\d+)").unwrap());
    let re_intel_core = RE_INTEL_CORE.get_or_init(|| Regex::new(r"core\s+[3579]\s+(\d)").unwrap());
    let re_ryzen = RE_RYZEN.get_or_init(|| Regex::new(r"ryzen\s+[3579]\s+(\d)(\d{2,3})").unwrap());
    let re_n_series = RE_N_SERIES.get_or_init(|| Regex::new(r"n\d{3}").unwrap());

    if let Some(cap) = re_intel.captures(&n) { 
        let digits = &cap[1];
        let gen = if digits.len() >= 5 {
            &digits[0..2]
        } else if digits.len() == 4 {
            // Para procesadores Intel de 10ª a 15ª Gen con modelos de 4 dígitos sin sufijo G (ej: i5-1235U, i5-1334U)
            if digits.starts_with('1') {
                &digits[0..2]
            } else {
                &digits[0..1]
            }
        } else {
            digits
        };
        format!("{}ª Gen", gen) 
    }
    else if let Some(cap) = re_intel_core.captures(&n) { 
        format!("Serie {}", &cap[1]) 
    }
    else if n.contains("ultra") { 
        "Core Ultra".to_string() 
    }
    else if n.contains("ryzen ai") { 
        "Ryzen AI".to_string() 
    }
    else if let Some(cap) = re_ryzen.captures(&n) { 
        if cap[2].len() == 2 {
            format!("{}00 Series", &cap[1])
        } else {
            format!("{}000 Series", &cap[1])
        }
    }
    else if re_n_series.is_match(&n) {
        "N-Series".to_string()
    }
    else if n.contains("snapdragon") || n.contains("qualcomm") {
        if n.contains("snapdragon x") || n.contains("x elite") || n.contains("x plus") || n.contains("x1") || n.contains("x2") || n.contains("x3") || n.contains("x4") {
            "Snapdragon X".to_string()
        } else {
            "Qualcomm ARM".to_string()
        }
    }
    else { 
        "Desconocida".to_string() 
    }
}

/// Formatea la capacidad de RAM, redondeando para absorber la memoria reservada por hardware.
fn get_ram_display(total_bytes: u64) -> String {
    let gb = total_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    let size = (gb / 2.0).round() * 2.0;
    format!("{:.0}GB", if size == 0.0 { gb.round() } else { size })
}

/// Normaliza nombres técnicos de fabricantes a sus nombres comerciales conocidos.
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

/// Indica si la información obtenida de marca/modelo es genérica (O.E.M, Default string, etc.) para forzar búsqueda en placa base.
fn is_generic_info(brand: &str, model: &str) -> bool {
    let b = brand.to_uppercase();
    let m = model.to_uppercase();
    b.contains("TO BE FILLED") || b.contains("O.E.M") || b.is_empty() || m.contains("SYSTEM PRODUCT") || m.contains("DEFAULT STRING") || m == b
}

/// Refina el nombre del modelo eliminando ruido repetitivo, normalizando espaciados de ASUS y deduplicando palabras.
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

/// Obtiene la ruta física del directorio de recursos de la aplicación.
#[tauri::command]
pub fn get_video_path(app: AppHandle) -> String {
    get_resource_dir(&app).to_string_lossy().into_owned()
}

#[tauri::command]
pub fn log_frontend_debug(msg: String) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:/Users/jmema/Proyectos/Zenit-Tauri/webview_debug.log")
    {
        use std::io::Write;
        let _ = writeln!(file, "{}", msg);
    }
}

/// Forza el brillo al 100% y desactiva el brillo adaptativo y el sueño mediante PowerShell.
/// Se ejecuta cuando el kiosco entra en modo video/inactividad.
#[tauri::command]
pub fn set_max_brightness() {
    // 1. Bloqueo Nativo (Hard Block): Informa a Windows que la pantalla y el sistema DEBEN estar activos,
    // ignorando cualquier plan de energía de terceros (como Armoury Crate o Lenovo Vantage).
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
        SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
    }

    // 2. Ajuste de Brillo (Ligero)
    // Solo sube el brillo de las pantallas conectadas al 100% sin modificar planes de energía de forma repetitiva.
    let script = r#"
        $ErrorActionPreference = 'SilentlyContinue'
        try {
            Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{ Timeout = 1; Brightness = 100 }
        } catch {}
    "#;
    match Command::new("powershell.exe")
        .args(["-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-Command", script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(_) => {},
        Err(e) => log::warn!("[Brightness] No se pudo lanzar PowerShell: {}", e),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessorInfo {
    pub vendor: String,
    pub gen: String,
}

/// Limpia y normaliza el nombre del procesador eliminando ruido técnico (marcas registradas, frecuencias, arquitecturas, sufijos WMI/Qualcomm).
pub fn clean_processor_name(raw: &str) -> String {
    let mut s = raw
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace("(tm)", "")
        .replace("(r)", "")
        .trim()
        .to_string();

    while s.contains("  ") {
        s = s.replace("  ", " ");
    }

    // 1. Manejo específico para Snapdragon / Qualcomm
    if s.to_lowercase().contains("snapdragon") || s.to_lowercase().contains("qualcomm") {
        // Cortar ruido técnico común en Win32_Processor Name para Qualcomm/Snapdragon:
        // Ej: "Snapdragon(R) X - X126100 - Qualcomm(R) Oryon(TM) CPU ARMv8 (64-bit) Family 8 Model 1 Revision 201 2956"
        let cut_markers = [
            " - Qualcomm",
            " - Oryon",
            " - ARMv8",
            " Qualcomm Oryon",
            " Oryon CPU",
            " ARMv8",
        ];
        for marker in cut_markers {
            if let Some(pos) = s.find(marker) {
                s = s[..pos].trim().to_string();
                break;
            }
        }

        if s.starts_with("Qualcomm Snapdragon") {
            s = s.replacen("Qualcomm Snapdragon", "Snapdragon", 1);
        }

        if s.is_empty() || s == "Qualcomm" {
            s = "Snapdragon X".to_string();
        }

        return s.trim().to_string();
    }

    // 2. Manejo para Intel / AMD u otros procesadores
    if let Some(pos) = s.find(" CPU @") {
        s = s[..pos].trim().to_string();
    } else if let Some(pos) = s.find("@") {
        s = s[..pos].trim().to_string();
    }

    if s.to_lowercase().ends_with(" processor") {
        let lower = s.to_lowercase();
        if let Some(pos) = lower.rfind(" processor") {
            s = s[..pos].trim().to_string();
        }
    }

    let words: Vec<&str> = s.split_whitespace().collect();
    if let Some(last_word) = words.last() {
        if last_word.to_lowercase().ends_with("-core") {
            s = words[..words.len()-1].join(" ");
        }
    }

    s.trim().to_string()
}

/// Infiere el fabricante y la generación de un procesador a partir de su nombre utilizando la lógica robusta del backend.
#[tauri::command]
pub fn infer_processor_info(name: String) -> ProcessorInfo {
    let clean_name = clean_processor_name(&name);
    let vendor = if clean_name.to_uppercase().contains("INTEL") {
        "Intel".to_string()
    } else if clean_name.to_uppercase().contains("AMD") {
        "AMD".to_string()
    } else if clean_name.to_lowercase().contains("snapdragon") || clean_name.to_lowercase().contains("qualcomm") {
        "Snapdragon".to_string()
    } else {
        "Generic".to_string()
    };
    let gen = detect_generation(&clean_name);
    ProcessorInfo { vendor, gen }
}

/// Abre una URL en el navegador predeterminado del sistema operativo.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(["/C", "start", "", &url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryStatus {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub used_percent: f32,
    pub is_low_memory: bool,
    pub is_critical: bool,
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_FRONTEND_HEARTBEAT: AtomicU64 = AtomicU64::new(0);

/// Recibe latidos periódicos del frontend (JS) para verificar la salud del hilo de la UI.
#[tauri::command]
pub fn frontend_heartbeat() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    LAST_FRONTEND_HEARTBEAT.store(now, Ordering::SeqCst);
}

pub fn get_last_heartbeat() -> u64 {
    LAST_FRONTEND_HEARTBEAT.load(Ordering::SeqCst)
}

use crate::state::{AppMode, AppState};
use tauri::Emitter;

#[tauri::command]
pub async fn set_app_mode(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    mode: AppMode,
) -> Result<(), String> {
    let mut guard = state.current_mode.lock().await;
    if *guard != mode {
        *guard = mode;
        log::info!("[Rust Master Engine] AppMode cambiado a: {:?}", mode);
        let _ = app_handle.emit("app-mode-changed", mode);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_app_mode(
    state: tauri::State<'_, AppState>,
) -> Result<AppMode, String> {
    let guard = state.current_mode.lock().await;
    Ok(*guard)
}

#[tauri::command]
pub async fn notify_user_activity(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.current_mode.lock().await;
    if *guard == AppMode::InactivityVideo {
        *guard = AppMode::InfoView;
        log::info!("[Rust Master Engine] Actividad de usuario detectada. Retornando a InfoView.");
        let _ = app_handle.emit("app-mode-changed", AppMode::InfoView);
    }
    Ok(())
}

#[tauri::command]
pub async fn notify_playlist_finished(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.current_mode.lock().await;
    if *guard == AppMode::InactivityVideo {
        *guard = AppMode::InfoView;
        log::info!("[Rust Master Engine] Playlist finalizada reportada por Vue. Retornando a InfoView.");
        let _ = app_handle.emit("app-mode-changed", AppMode::InfoView);
    }
    Ok(())
}



/// Obtiene el estado actual de la memoria RAM del sistema.
#[tauri::command]
pub fn get_memory_status() -> MemoryStatus {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_memory(MemoryRefreshKind::everything())
    );
    sys.refresh_memory();

    let total_bytes = sys.total_memory();
    let used_bytes = sys.used_memory();
    let available_bytes = sys.available_memory();

    let total_mb = total_bytes / 1024 / 1024;
    let used_mb = used_bytes / 1024 / 1024;
    let available_mb = available_bytes / 1024 / 1024;

    let used_percent = if total_bytes > 0 {
        (used_bytes as f32 / total_bytes as f32) * 100.0
    } else {
        0.0
    };

    // Umbrales adaptativos según la capacidad total del equipo (8GB vs 16GB/32GB)
    let (is_low_memory, is_critical) = if total_mb <= 9000 {
        // En equipos de 8GB de RAM:
        // - Alerta (trim): > 88% de uso o menos de 1000 MB libres.
        // - Crítico (reinicio): > 94% de uso o menos de 500 MB libres.
        (used_percent > 88.0 || available_mb < 1000, used_percent > 94.0 || available_mb < 500)
    } else {
        // En equipos de 16GB / 32GB de RAM:
        // - Alerta (trim): > 85% de uso o menos de 2500 MB libres.
        // - Crítico (reinicio): > 92% de uso o menos de 1200 MB libres.
        (used_percent > 85.0 || available_mb < 2500, used_percent > 92.0 || available_mb < 1200)
    };

    MemoryStatus {
        total_mb,
        used_mb,
        available_mb,
        used_percent,
        is_low_memory,
        is_critical,
    }
}

#[tauri::command]
pub fn trim_memory() {}

#[cfg(not(windows))]
fn get_wmi_details() -> Result<WmiData, Box<dyn std::error::Error>> {
    Ok(default_wmi_fallback())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let cases = vec![
            // Intel Core i-Series
            ("Intel Core i5-1334U", "13ª Gen"),
            ("Intel Core i5-1335U", "13ª Gen"),
            ("Intel Core i5-1235U", "12ª Gen"),
            ("Intel Core i7-1355U", "13ª Gen"),
            ("Intel Core i7-12700H", "12ª Gen"),
            ("Intel Core i5-1135G7", "11ª Gen"),
            ("Intel Core i9-10900K", "10ª Gen"),
            ("Intel Core i7-9750H", "9ª Gen"),
            ("intel core i5-8250u", "8ª Gen"), // Minúsculas
            
            // Intel Core (Series)
            ("Intel Core 5 120U", "Serie 1"),
            ("Intel Core 7 150U", "Serie 1"),
            ("intel core 3 100u", "Serie 1"),
            
            // Intel Ultra
            ("Intel Core Ultra 7 155H", "Core Ultra"),
            ("intel core ultra 9 185h", "Core Ultra"),
            
            // Intel N-Series
            ("Intel Processor N100", "N-Series"),
            ("Intel Processor N200", "N-Series"),
            ("intel processor n305", "N-Series"),
            
            // AMD Ryzen AI
            ("AMD Ryzen AI 9 HX 370", "Ryzen AI"),
            ("amd ryzen ai 7 pro 360", "Ryzen AI"),
            
            // AMD Ryzen Clásico
            ("AMD Ryzen 7 5700U", "5000 Series"),
            ("AMD Ryzen 5 5600X", "5000 Series"),
            ("AMD Ryzen 9 7900X", "7000 Series"),
            ("AMD Ryzen 5 7520U", "7000 Series"),
            ("amd ryzen 3 3250u", "3000 Series"),
            ("AMD Ryzen 7 270U", "200 Series"), // 3 dígitos (ej: X00 Series)
            ("AMD Ryzen 5 350U", "300 Series"),
            
            // Snapdragon / Qualcomm
            ("Snapdragon X - X126100", "Snapdragon X"),
            ("Snapdragon X Elite X1E-84-100", "Snapdragon X"),
            ("Qualcomm Snapdragon X Plus X1P-64-100", "Snapdragon X"),
            ("Qualcomm Snapdragon 8cx Gen 3", "Qualcomm ARM"),
            
            // Genéricos / Desconocidos
            ("Intel Pentium Gold 7505", "Desconocida"),
            ("AMD Athlon Gold 3150U", "Desconocida"),
        ];

        for (input, expected) in cases {
            let res = detect_generation(input);
            assert_eq!(res, expected, "Failed for CPU: {}", input);
        }
    }

    #[test]
    fn test_approximate_refresh_rate() {
        assert_eq!(approximate_refresh_rate(59), 60);
        assert_eq!(approximate_refresh_rate(60), 60);
        assert_eq!(approximate_refresh_rate(74), 75);
        assert_eq!(approximate_refresh_rate(143), 144);
        assert_eq!(approximate_refresh_rate(165), 165);
        assert_eq!(approximate_refresh_rate(239), 240);
        assert_eq!(approximate_refresh_rate(360), 360);
        assert_eq!(approximate_refresh_rate(0), 0);
        assert_eq!(approximate_refresh_rate(13), 13); // should not map since difference is > 10
    }

    #[test]
    fn test_format_bytes_commercial() {
        // Tamaños estándar
        assert_eq!(format_bytes_commercial(512_000_000_000), "512GB SSD");
        assert_eq!(format_bytes_commercial(1_024_000_000_000), "1TB SSD");
        
        // Redondeo comercial (lo que Windows lee vs lo que se vende)
        assert_eq!(format_bytes_commercial(476_940_000_000), "512GB SSD"); // 512GB real
        assert_eq!(format_bytes_commercial(931_000_000_000), "1TB SSD");   // 1TB real
        assert_eq!(format_bytes_commercial(238_000_000_000), "256GB SSD"); // 256GB real
        assert_eq!(format_bytes_commercial(119_000_000_000), "128GB SSD"); // 128GB real
    }

    #[test]
    fn test_rate_gpu() {
        let dedicated_nvidia = rate_gpu("NVIDIA GeForce RTX 4060 Laptop GPU");
        let dedicated_amd = rate_gpu("AMD Radeon RX 7600S");
        let integrated_intel = rate_gpu("Intel(R) UHD Graphics");
        let integrated_amd = rate_gpu("AMD Radeon(TM) Graphics");
        let generic = rate_gpu("Basic Display Adapter");

        assert!(dedicated_nvidia > integrated_intel, "NVIDIA debería ganarle a Intel UHD");
        assert!(dedicated_amd > integrated_amd, "RX debería ganarle a Radeon genérico");
        assert_eq!(dedicated_nvidia, 10);
        assert_eq!(dedicated_amd, 8);
        assert_eq!(integrated_intel, 2);
        assert_eq!(generic, 1);
    }

    #[test]
    fn test_clean_brand_name() {
        assert_eq!(clean_brand_name("ASUSTeK COMPUTER INC."), "ASUS");
        assert_eq!(clean_brand_name("Hewlett-Packard"), "HP");
        assert_eq!(clean_brand_name("Micro-Star International Co., Ltd."), "MSI");
        assert_eq!(clean_brand_name("LENOVO"), "Lenovo");
    }

    #[test]
    fn test_refine_model_name() {
        assert_eq!(refine_model_name("ASUS", "ASUS TUF Gaming F15"), "ASUS TUF Gaming F15");
        assert_eq!(refine_model_name("HP", "HP Laptop 15s-eq2xxx"), "HP 15s eq2xxx");
        assert_eq!(refine_model_name("Lenovo", "82K2"), "Lenovo 82K2"); 
    }

    #[test]
    fn test_clean_processor_name() {
        assert_eq!(
            clean_processor_name("Snapdragon(R) X - X126100 - Qualcomm(R) Oryon(TM) CPU ARMv8 (64-bit) Family 8 Model 1 Revision 201 2956"),
            "Snapdragon X - X126100"
        );
        assert_eq!(
            clean_processor_name("Qualcomm(R) Snapdragon(R) X Plus X1P-64-100 - Qualcomm(R) Oryon(TM) CPU ARMv8 (64-bit)"),
            "Snapdragon X Plus X1P-64-100"
        );
        assert_eq!(
            clean_processor_name("Intel(R) Core(TM) i7-13700H CPU @ 2.40GHz"),
            "Intel Core i7-13700H"
        );
        assert_eq!(
            clean_processor_name("AMD Ryzen 5 5600X 6-Core Processor"),
            "AMD Ryzen 5 5600X"
        );
    }
}
