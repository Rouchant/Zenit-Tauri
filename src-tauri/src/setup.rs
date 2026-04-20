use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const NO_WINDOW: u32 = 0x08000000;

/// Ejecuta configuraciones de sistema (Brillo, Energía, etc.)
pub fn run_system_setup() {
    println!("Configurando ajustes de sistema (PowerCfg)...");

    // 1. Cambiar a plan de Alto Rendimiento primero
    if let Ok(output) = Command::new("powercfg").arg("/l")
        .stdout(Stdio::piped()).stderr(Stdio::null())
        .creation_flags(NO_WINDOW).output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().find(|l| l.contains("High performance") || l.contains("Alto rendimiento")) {
            if let Some(guid) = line.split_whitespace().nth(3) {
                let _ = Command::new("powercfg").args(["/s", guid])
                    .stdout(Stdio::null()).stderr(Stdio::null())
                    .creation_flags(NO_WINDOW).status();
            }
        }
    }

    // 2. Ahora aplicar todos los ajustes sobre el plan activo
    for args in [
        ["/x", "-hibernate-timeout-ac", "0"],
        ["/x", "-standby-timeout-ac",   "0"],
        ["/x", "-monitor-timeout-ac",   "0"],
        ["/hibernate", "off",           ""],
    ] {
        let _ = Command::new("powercfg")
            .args(args.iter().filter(|a| !a.is_empty()))
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).status();
    }

    // 3. Desactivar Adaptive Brightness (atenuación automática por luz ambiental)
    for args in [
        ["-setacvalueindex", "SCHEME_CURRENT", "SUB_VIDEO", "ADAPTBRIGHT", "0"],
        ["-setdcvalueindex", "SCHEME_CURRENT", "SUB_VIDEO", "ADAPTBRIGHT", "0"],
    ] {
        let _ = Command::new("powercfg")
            .args(args)
            .stdout(Stdio::null()).stderr(Stdio::null())
            .creation_flags(NO_WINDOW).status();
    }

    // 4. Brillo al 100%
    let _ = Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive", "-Command",
            "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue)?.WmiSetBrightness(1,100)"
        ])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .creation_flags(NO_WINDOW)
        .spawn();
}



