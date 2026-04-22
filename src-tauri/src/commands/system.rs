use serde::{Serialize, Deserialize};
use sysinfo::{System, Disks};
use regex::Regex;
use tauri::AppHandle;
use crate::state::get_resource_dir;
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Serialize, Deserialize)]
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
    let mut sys = System::new_all();
    sys.refresh_all();

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

    let mut gen = "Desconocida".to_string();
    let re_intel = Regex::new(r"i[3579]-(\d+)").unwrap();
    let re_intel_core = Regex::new(r"Core\s+[3579]\s+(\d)").unwrap();
    let re_ryzen = Regex::new(r"Ryzen\s+[3579]\s+(\d)(\d{2,3})").unwrap();

    if let Some(cap) = re_intel.captures(&proc_name) {
        gen = format!("{}a Gen", &cap[1]);
    } else if let Some(cap) = re_intel_core.captures(&proc_name) {
        gen = format!("Serie {}", &cap[1]);
    } else if proc_name.contains("Ultra") {
        gen = "Core Ultra".to_string();
    } else if proc_name.contains("Ryzen AI") {
        gen = "Ryzen AI".to_string();
    } else if let Some(cap) = re_ryzen.captures(&proc_name) {
        if cap[2].len() == 2 {
            gen = format!("{}00 Series", &cap[1]);
        } else {
            gen = format!("{}000 Series", &cap[1]);
        }
    } else if proc_name.contains("N") && proc_name.chars().any(|c| c.is_numeric()) {
        gen = "N-Series".to_string();
    }

    // 2. RAM Info
    let total_memory = sys.total_memory(); // en bytes
    let ram_gb = total_memory as f64 / 1024.0 / 1024.0 / 1024.0;
    let ram_size = (ram_gb / 4.0).round() * 4.0;
    let ram_display = if ram_size == 0.0 { format!("{:.0} GB", ram_gb.round()) } else { format!("{:.0} GB", ram_size) };

    // 3. Storage Info
    let mut total_storage_bytes = 0;
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        total_storage_bytes += disk.total_space();
    }
    
    // Usamos gigabytes comerciales (10^9) para que coincida con los umbrales (872, 1744, etc.)
    let total_gb = total_storage_bytes as f64 / 1_000_000_000.0;
    
    let storage_display = if total_gb >= 872.0 {
        let tb = total_gb / 1000.0;
        let fract = tb.fract();
        
        // Regla personalizada: >872GB o >1.872TB -> redondear al siguiente TB
        if fract >= 0.872 {
            format!("{:.0}TB SSD", tb.ceil())
        } 
        // Si está muy cerca de la base del TB, redondear hacia abajo
        else if fract <= 0.128 {
            format!("{:.0}TB SSD", tb.floor())
        }
        // Para valores intermedios (como 1.5 TB), mostrar 1 decimal
        else {
            format!("{:.1}TB SSD", tb).replace(".", ",")
        }
    } else {
        // Para discos pequeños, redondear al múltiplo de 128GB más cercano o mostrar real
        let rounded_gb = (total_gb / 128.0).round() * 128.0;
        format!("{:.0}GB SSD", if rounded_gb == 0.0 { total_gb.round() } else { rounded_gb })
    };

    // 4. Detailed Info via WMI
    let (brand, model, gpu, display, ram_type, os_name) = get_wmi_details().await.unwrap_or_else(|_| (
        "PC Generico".to_string(),
        "PC Desktop".to_string(),
        "Gráficos Integrados".to_string(),
        "1920 x 1080".to_string(),
        "DDR4".to_string(),
        System::name().unwrap_or_else(|| "Windows".to_string()).replace("Microsoft ", "")
    ));

    Ok(SystemSpecs {
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
    })
}

#[cfg(windows)]
async fn get_wmi_details() -> Result<(String, String, String, String, String, String), Box<dyn std::error::Error>> {
    use wmi::{COMLibrary, WMIConnection};
    use std::collections::HashMap;

    // Inicializar COM una sola vez
    let com_con = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_con.into())?;

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

    // Fallback a Placa Base si es PC Armado
    if brand.contains("To be filled") || brand.contains("System manufacturer") || brand.contains("O.E.M.") || brand.is_empty() {
        if let Ok(mb_results) = wmi_con.raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard") {
            let mb_results: Vec<HashMap<String, serde_json::Value>> = mb_results;
            if let Some(res) = mb_results.first() {
                brand = res.get("Manufacturer").and_then(|v| v.as_str()).unwrap_or("PC Desktop").trim().to_string();
                if model == "System Product Name" || model == "Default string" || model == "PC Desktop" || model.contains("B550") {
                    model = res.get("Product").and_then(|v| v.as_str()).unwrap_or("PC Desktop").trim().to_string();
                }
            }
        }
    }

    let full_brand = if !model.is_empty() && model != "PC Desktop" {
        if model.to_uppercase().starts_with(&brand.to_uppercase()) { model } else { format!("{} {}", brand, model) }
    } else {
        brand
    };

    // --- 2. GPU y Resolución (fallback) ---
    let mut puntuacion_actual = 0;

    for res in &gpu_results {
        if let Some(name) = res.get("Name").and_then(|v| v.as_str()) {
            let name_up = name.to_uppercase();
            let mut puntuacion = 0;

            // 1. PRIORIDAD MÁXIMA: NVIDIA / RTX / GTX
            if name_up.contains("NVIDIA") || name_up.contains("RTX") || name_up.contains("GTX") {
                puntuacion = 4;
            } 
            // 2. SEGUNDA PRIORIDAD: AMD Radeon RX (Dedicadas)
            else if name_up.contains("RX ") {
                puntuacion = 3;
            } 
            // 3. TERCERA PRIORIDAD: Intel ARC (Dedicadas)
            else if name_up.contains("ARC") {
                puntuacion = 2;
            } 
            // 4. CUARTA PRIORIDAD: Integradas (UHD, Radeon Graphics, Iris, etc.)
            else if name_up.contains("UHD") || name_up.contains("RADEON") || 
                    name_up.contains("IRIS") || name_up.contains("INTEL") {
                puntuacion = 1;
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

    // --- 3. Resolución Nativa del Monitor ---
    let mut max_h = 0;
    let mut max_v = 0;
    if let Ok(wmi_mon_con) = WMIConnection::with_namespace_path("root\\wmi", COMLibrary::new()?) {
        if let Ok(monitor_results) = wmi_mon_con.raw_query("SELECT HorizontalActivePixels, VerticalActivePixels FROM WmiMonitorListedSupportedSourceModes") {
            let monitor_results: Vec<HashMap<String, serde_json::Value>> = monitor_results;
            for res in &monitor_results {
                let h = res.get("HorizontalActivePixels").and_then(|v| v.as_u64()).unwrap_or(0);
                let v = res.get("VerticalActivePixels").and_then(|v| v.as_u64()).unwrap_or(0);
                if h > max_h { max_h = h; max_v = v; }
            }
        }
    }

    if max_h == 0 { max_h = v_h; max_v = v_v; }

    let mut display = if max_h > 0 { format!("{} x {}", max_h, max_v) } else { "1920 x 1080".to_string() };
    if max_h == 1920 && max_v == 1080 { display.push_str(" (Full HD)"); }
    else if max_h == 2560 && max_v == 1440 { display.push_str(" (2K QHD)"); }
    else if max_h == 3840 && max_v == 2160 { display.push_str(" (4K UHD)"); }

    // --- 4. Tipo de RAM ---
    let mut ram_type = "DDR4".to_string();
    if let Ok(ram_results) = wmi_con.raw_query("SELECT SMBIOSMemoryType FROM Win32_PhysicalMemory") {
        let ram_results: Vec<HashMap<String, serde_json::Value>> = ram_results;
        if let Some(res) = ram_results.first() {
            let smbios_type = res.get("SMBIOSMemoryType").and_then(|v| v.as_u64()).unwrap_or(0);
            ram_type = match smbios_type {
                26 => "DDR4".to_string(),
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

    Ok((full_brand, "PC Desktop".to_string(), gpu, display, ram_type, os_name))
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

