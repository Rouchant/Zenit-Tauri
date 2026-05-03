use serde::{Serialize, Deserialize};
use sysinfo::{System, Disks, CpuRefreshKind, MemoryRefreshKind, RefreshKind};
use regex::Regex;
use tauri::AppHandle;
use crate::state::get_resource_dir;
use std::process::Command;
use std::sync::OnceLock;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

// --- Expresiones Regulares Optimizadas (OnceLock) ---
static RE_INTEL: OnceLock<Regex> = OnceLock::new();
static RE_INTEL_CORE: OnceLock<Regex> = OnceLock::new();
static RE_RYZEN: OnceLock<Regex> = OnceLock::new();
static NVIDIA_POWER_LIMIT: OnceLock<Option<String>> = OnceLock::new();
static CACHED_SPECS: OnceLock<SystemSpecs> = OnceLock::new();

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

#[tauri::command]
pub async fn get_system_specs() -> Result<SystemSpecs, String> {
    // Si ya tenemos los datos en cache, retornarlos inmediatamente (la HW no cambia)
    if let Some(cached) = CACHED_SPECS.get() {
        return Ok(cached.clone());
    }

    // 0. Refresco selectivo para máximo rendimiento (descubriendo componentes primero)
    let sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
    );

    // 1. Processor Info
    let cpu = sys.cpus().first().ok_or("No se detectó CPU")?;
    let raw_proc_name = cpu.brand().trim();
    let proc_name = raw_proc_name
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace("  ", " ");
    
    let vendor = if proc_name.contains("Intel") { "Intel" } 
                else if proc_name.contains("AMD") { "AMD" } 
                else { "Generic" };

    let gen = detect_generation(&proc_name);

    // 2. RAM Info
    let ram_display = get_ram_display(sys.total_memory());

    // 3. Storage Info
    let mut total_storage_bytes = 0;
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        total_storage_bytes += disk.total_space();
    }
    let storage_display = format_storage(total_storage_bytes);

    // 4. Detailed Info via WMI
    let (brand, model, gpu, display, ram_type, os_name) = get_wmi_details().await.unwrap_or_else(|_| (
        "PC Generico".to_string(),
        "PC Desktop".to_string(),
        "Gráficos Integrados".to_string(),
        "1920 x 1080".to_string(),
        "DDR4".to_string(),
        System::name().unwrap_or_else(|| "Windows".to_string()).replace("Microsoft ", "")
    ));

    let specs = SystemSpecs {
        brand,
        model,
        processor: proc_name,
        cores: sys.physical_core_count().unwrap_or(0),
        threads: sys.cpus().len(),
        gen,
        vendor: vendor.to_string(),
        ram: ram_display,
        ram_type,
        gpu,
        storage: storage_display,
        display,
        os: os_name,
    };

    // Guardar en cache antes de retornar
    let _ = CACHED_SPECS.set(specs.clone());

    Ok(specs)
}

// --- FUNCIONES DE SOPORTE (MODULARIZACIÓN) ---

fn detect_generation(proc_name: &str) -> String {
    let re_intel = RE_INTEL.get_or_init(|| Regex::new(r"i[3579]-(\d+)").unwrap());
    let re_intel_core = RE_INTEL_CORE.get_or_init(|| Regex::new(r"Core\s+[3579]\s+(\d)").unwrap());
    let re_ryzen = RE_RYZEN.get_or_init(|| Regex::new(r"Ryzen\s+[3579]\s+(\d)(\d{2,3})").unwrap());

    if let Some(cap) = re_intel.captures(proc_name) {
        format!("{}a Gen", &cap[1])
    } else if let Some(cap) = re_intel_core.captures(proc_name) {
        format!("Serie {}", &cap[1])
    } else if proc_name.contains("Ultra") {
        "Core Ultra".to_string()
    } else if proc_name.contains("Ryzen AI") {
        "Ryzen AI".to_string()
    } else if let Some(cap) = re_ryzen.captures(proc_name) {
        if cap[2].len() == 2 {
            format!("{}00 Series", &cap[1])
        } else {
            format!("{}000 Series", &cap[1])
        }
    } else if proc_name.contains("N") && proc_name.chars().any(|c| c.is_numeric()) {
        "N-Series".to_string()
    } else {
        "Desconocida".to_string()
    }
}

fn get_ram_display(total_memory_bytes: u64) -> String {
    let ram_gb = total_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    // Redondeo a múltiplos de 2 para absorber reserva de hardware (ej: 15.4 -> 16, 5.6 -> 6)
    let ram_size = (ram_gb / 2.0).round() * 2.0;
    
    if ram_size == 0.0 { 
        format!("{:.0} GB", ram_gb.round()) 
    } else { 
        format!("{:.0} GB", ram_size) 
    }
}

fn format_storage(total_storage_bytes: u64) -> String {
    // Usamos gigabytes comerciales (10^9)
    let total_gb = total_storage_bytes as f64 / 1_000_000_000.0;
    
    if total_gb >= 872.0 {
        let tb = total_gb / 1000.0;
        let fract = tb.fract();
        
        // >872GB o >1.872TB -> redondear al siguiente TB
        if fract >= 0.872 {
            format!("{:.0}TB SSD", tb.ceil())
        } 
        else if fract <= 0.128 {
            format!("{:.0}TB SSD", tb.floor())
        }
        else {
            format!("{:.1}TB SSD", tb).replace(".", ",")
        }
    } else {
        // Redondear al múltiplo de 128GB más cercano o mostrar real
        let rounded_gb = (total_gb / 128.0).round() * 128.0;
        format!("{:.0}GB SSD", if rounded_gb == 0.0 { total_gb.round() } else { rounded_gb })
    }
}

#[cfg(windows)]
/// Normaliza nombres de fabricantes técnicos a sus nombres comerciales.
fn clean_brand_name(raw: &str) -> String {
    let raw_upper = raw.to_uppercase();
    
    if raw_upper.contains("ASUSTEK") { return "ASUS".to_string(); }
    if raw_upper.contains("HEWLETT-PACKARD") || raw_upper.contains("HP") { return "HP".to_string(); }
    if raw_upper.contains("SAMSUNG") { return "Samsung".to_string(); }
    if raw_upper.contains("DELL") { return "Dell".to_string(); }
    if raw_upper.contains("LENOVO") { return "Lenovo".to_string(); }
    if raw_upper.contains("ASROCK") { return "ASRock".to_string(); }
    if raw_upper.contains("GIGABYTE") { return "Gigabyte".to_string(); }
    if raw_upper.contains("MSI") || raw_upper.contains("MICRO-STAR") { return "MSI".to_string(); }
    if raw_upper.contains("VIRTUALBOX") { return "VirtualBox".to_string(); }
    
    if raw_upper.contains("TO BE FILLED") || raw_upper.contains("SYSTEM MANUFACTURER") || 
       raw_upper.contains("O.E.M") || raw_upper.is_empty() {
        return "PC Desktop".to_string();
    }

    raw.trim().to_string()
}

#[cfg(windows)]
async fn get_wmi_details() -> Result<(String, String, String, String, String, String), Box<dyn std::error::Error>> {
    use wmi::{COMLibrary, WMIConnection};
    use std::collections::HashMap;

    // Inicializar COM una sola vez
    let com_con = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_con)?;

    // --- 1. Marca y Modelo ---
    let mut brand = "PC Generico".to_string();
    let mut model = "PC Desktop".to_string();
    
    if let Ok(results) = wmi_con.raw_query("SELECT Manufacturer, Model FROM Win32_ComputerSystem") {
        let results: Vec<HashMap<String, serde_json::Value>> = results;
        if let Some(res) = results.first() {
            brand = res.get("Manufacturer").and_then(|v| v.as_str()).unwrap_or("PC Generico").trim().to_string();
            model = res.get("Model").and_then(|v| v.as_str()).unwrap_or("PC Desktop").trim().to_string();
        }
    }

    // Limpiar nombres de fabricantes técnicos usando helper compartido
    brand = clean_brand_name(&brand);

    // Fallback a Placa Base si es PC Armado o información genérica (insensible a mayúsculas)
    let b_up = brand.to_uppercase();
    let m_up = model.to_uppercase();
    
    if b_up.contains("TO BE FILLED") || b_up.contains("PC DESKTOP") || b_up.contains("O.E.M") || 
       brand.is_empty() || m_up.contains("SYSTEM PRODUCT") || m_up.contains("DEFAULT STRING") || m_up == b_up {
        
        if let Ok(mb_results) = wmi_con.raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard") {
            let mb_results: Vec<HashMap<String, serde_json::Value>> = mb_results;
            if let Some(res) = mb_results.first() {
                let mb_brand = res.get("Manufacturer").and_then(|v| v.as_str()).unwrap_or("PC Desktop");
                brand = clean_brand_name(mb_brand); // Reutilizamos el mismo helper
                
                // Forzar el uso del modelo de la placa si el actual es genérico o igual a la marca
                let current_model_upper = model.to_uppercase();
                let brand_upper_local = brand.to_uppercase();
                
                if model == "System Product Name" || model == "Default string" || model == "PC Desktop" || 
                   model.len() < 3 || current_model_upper == brand_upper_local {
                    
                    let product = res.get("Product").and_then(|v| v.as_str()).unwrap_or("").trim();
                    if !product.is_empty() && product != "Default string" {
                        model = product.to_string();
                    }
                }
            }
        }
    }

    // --- LIMPIEZA Y FORMATEO DE MODELO ---
    if brand.to_uppercase().contains("VIRTUALBOX") {
        model = "Virtual Machine".to_string();
    } else if brand == "PC Generico" || brand == "PC Desktop" || model == "PC Desktop" {
        if model.is_empty() || model.len() < 2 {
            model = "PC Desktop".to_string();
        }
    } else {
        model = refine_model_name(&brand, &model);
    }

    // --- 2. GPU y Resolución (fallback) ---
    let mut gpu = "Gráficos Integrados".to_string();
    let mut v_h = 0;
    let mut v_v = 0;
    let mut puntuacion_actual = 0;

    let gpu_results: Vec<HashMap<String, serde_json::Value>> = wmi_con
        .raw_query("SELECT Name, CurrentHorizontalResolution, CurrentVerticalResolution FROM Win32_VideoController")
        .unwrap_or_default();

    for res in &gpu_results {
        if let Some(raw_name) = res.get("Name").and_then(|v| v.as_str()) {
            let cleaned_name = raw_name
                .replace("(R)", "")
                .replace("(TM)", "")
                .replace("  ", " ");
            let name = cleaned_name.trim();
            
            let name_up = name.to_uppercase();
            let mut puntuacion = 1; // Puntuación base por tener nombre

            // 1. PRIORIDAD MÁXIMA: NVIDIA / RTX / GTX
            if name_up.contains("NVIDIA") || name_up.contains("RTX") || name_up.contains("GTX") {
                puntuacion = 10;
            } 
            // 2. SEGUNDA PRIORIDAD: AMD Radeon RX (Dedicadas)
            else if name_up.contains("RX ") {
                puntuacion = 8;
            } 
            // 3. TERCERA PRIORIDAD: Intel ARC (Dedicadas)
            else if name_up.contains("ARC") {
                puntuacion = 5;
            } 
            // 4. CUARTA PRIORIDAD: Integradas conocidas (UHD, Radeon Graphics, Iris, etc.)
            else if name_up.contains("UHD") || name_up.contains("RADEON") || 
                    name_up.contains("IRIS") || name_up.contains("INTEL") {
                puntuacion = 2;
            }

            // Solo actualizamos si esta GPU es "mejor" que la que ya teníamos
            if puntuacion > puntuacion_actual {
                puntuacion_actual = puntuacion;
                gpu = name.to_string();
                v_h = res.get("CurrentHorizontalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
                v_v = res.get("CurrentVerticalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }

    // --- DETECCIÓN DE POWER LIMIT (NVIDIA) --- Post-loop: solo si la GPU ganadora es NVIDIA
    let gpu_up = gpu.to_uppercase();
    if gpu_up.contains("NVIDIA") || gpu_up.contains("RTX") || gpu_up.contains("GTX") {
        let watts_opt = NVIDIA_POWER_LIMIT.get_or_init(|| {
            // Usamos el comando que el usuario confirmó que funciona (Max Power Limit)
            // Esto evita problemas donde power.limit devuelve N/A en algunos sistemas
            let script = r#"$val = (nvidia-smi -q -d POWER | Select-String "Max Power Limit" | Where-Object { $_ -notmatch "N/A" }); if ($val) { [int][float]($val.ToString().Split(':')[1].Replace('W','').Trim()) }"#;
            
            if let Ok(output) = Command::new("powershell")
                .args(["-NoProfile", "-Command", script])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output() {
                
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !stdout.is_empty() && stdout.chars().all(|c| c.is_numeric()) {
                    return Some(stdout);
                }
            }
            None
        });

        if let Some(watts) = watts_opt {
            gpu = format!("{} {}W", gpu, watts);
        }
    }

    // --- 3. Resolución Real (Triple Fallback) ---
    let mut max_h = 0;
    let mut max_v = 0;

    // Fallback 1: Intentar vía Win32_VideoController (Lo más común)
    // Ya tenemos v_h y v_v del loop anterior, pero el usuario prefiere un loop dedicado o validar el máximo
    if v_h > 0 {
        max_h = v_h;
        max_v = v_v;
    } else {
        for res in &gpu_results {
            let h = res.get("CurrentHorizontalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
            let v = res.get("CurrentVerticalResolution").and_then(|v| v.as_u64()).unwrap_or(0);
            if h > max_h { max_h = h; max_v = v; }
        }
    }

    // Fallback 2: Si el anterior falló, intentar Win32_DesktopMonitor
    if max_h == 0 {
        if let Ok(results) = wmi_con.raw_query("SELECT ScreenWidth, ScreenHeight FROM Win32_DesktopMonitor") {
            let results: Vec<HashMap<String, serde_json::Value>> = results;
            if let Some(res) = results.first() {
                max_h = res.get("ScreenWidth").and_then(|v| v.as_u64()).unwrap_or(0);
                max_v = res.get("ScreenHeight").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }

    // Fallback 3: Si todo falla, forzar Full HD por defecto
    if max_h == 0 { max_h = 1920; max_v = 1080; }

    let mut display = format!("{} x {}", max_h, max_v);

    // --- Etiquetas Inteligentes ---
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
    display.push_str(label);

    // --- 4. Tipo de RAM ---
    let mut ram_type = "DDR4".to_string();
    if let Ok(ram_results) = wmi_con.raw_query("SELECT SMBIOSMemoryType FROM Win32_PhysicalMemory") {
        let ram_results: Vec<HashMap<String, serde_json::Value>> = ram_results;
        if let Some(res) = ram_results.first() {
            let smbios_type = res.get("SMBIOSMemoryType").and_then(|v| v.as_u64()).unwrap_or(0);
            ram_type = match smbios_type {
                26 => "DDR4".to_string(),
                30 | 31 => "LPDDR4".to_string(),
                34 => "DDR5".to_string(),
                35 => "LPDDR5".to_string(),
                _ => "DDR4".to_string()
            };
        }
    }

    // --- 5. Versión de Windows ---
    let mut os_name = "Windows".to_string();
    if let Ok(os_results) = wmi_con.raw_query("SELECT Caption FROM Win32_OperatingSystem") {
        let os_results: Vec<HashMap<String, serde_json::Value>> = os_results;
        if let Some(res) = os_results.first() {
            os_name = res.get("Caption").and_then(|v| v.as_str()).unwrap_or("Windows").replace("Microsoft ", "").trim().to_string();
        }
    }

    Ok((brand, model, gpu, display, ram_type, os_name))
}

#[cfg(not(windows))]
async fn get_wmi_details() -> Result<(String, String, String, String, String, String), Box<dyn std::error::Error>> {
    Ok(("Generic Brand".to_string(), "Generic Model".to_string(), "Generic GPU".to_string(), "1920x1080".to_string(), "DDR4".to_string(), "Windows".to_string()))
}

#[tauri::command]
pub fn get_video_path(app: AppHandle) -> String {
    get_resource_dir(&app).to_string_lossy().into_owned()
}

/// Intenta fijar el brillo al 100% vía WMI.
/// Se llama al entrar al modo video (inactividad).
#[tauri::command]
pub fn set_max_brightness() {
    let script = r#"
        try {
            $methods = Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods -ErrorAction Stop
            $methods.WmiSetBrightness(1, 100)
        } catch {}
    "#;
    let _ = Command::new("powershell.exe")
        .args(["-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
}

/// Limpia y unifica los nombres de modelos (especialmente para Asus, HP, etc.)
fn refine_model_name(brand: &str, model: &str) -> String {
    let noise = ["ASUSTEK", "COMPUTER", "INC", "CORP", "CORPORATION", "LTD", "SYSTEMS", "PRODUCT", "NAME", "LAPTOP"];
    
    let mut clean = model
        .replace("_", " ")
        .replace("ASUSLaptop", " Laptop ")
        .replace("-", " ")
        .trim()
        .to_string();

    while clean.contains("  ") {
        clean = clean.replace("  ", " ");
    }

    let words: Vec<&str> = clean.split_whitespace().collect();
    let mut unique_words: Vec<String> = Vec::new();
    let brand_up = brand.to_uppercase();

    for &word in &words {
        let word_up = word.to_uppercase().replace(".", "");
        
        if noise.contains(&word_up.as_str()) {
            continue;
        }

        if word_up == brand_up || word_up.contains(&brand_up) {
            continue;
        }

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
    
    if result_model.is_empty() {
        return brand.to_string();
    }

    format!("{} {}", brand, result_model)
}
