# 🚀 Zenit - Kiosk Framework (Tauri v2 Edition)

![Version](https://img.shields.io/badge/version-1.0.3-blue.svg)
![Tauri](https://img.shields.io/badge/framework-Tauri%20v2-FFC131.svg)
![Vue 3](https://img.shields.io/badge/frontend-Vue%203-42b883.svg)
![Vite](https://img.shields.io/badge/build-Vite%206-646cff.svg)
![Windows](https://img.shields.io/badge/OS-Windows%2010%2F11-0078D4.svg)

Una solución de nivel empresarial para **Showcase Terminals**, diseñada específicamente para equipos de exhibición en puntos de venta (Retail). **Zenit** ha sido reconstruido desde cero utilizando **Tauri v2**, ofreciendo un rendimiento nativo superior, una seguridad mejorada y un consumo de recursos mínimo.

---

## ✨ Características Principales

### 🛡️ Kiosk Lockdown & Security
- **Anti-Focus Theft**: Implementación nativa en Rust que utiliza el `AttachThreadInput` hack y simulación de teclas de sistema (`Escape`) para asegurar que la aplicación mantenga siempre el foco, incluso ante overlays de Windows.
- **Bloqueo de Atajos**: Inhabilitación de combinaciones de teclas críticas como `Alt+Tab`, `Win+D`, `Alt+F4` y `Win+L` mediante el plugin de shortcuts globales de Tauri.
- **Auto-Restore**: Temporizador inteligente de 5 minutos que restaura la aplicación a pantalla completa si es minimizada por mantenimiento.

### 🛰️ Telemetría de Hardware Nativa
- **PowerShell Bridge**: Integración profunda con el sistema para detectar especificaciones exactas (CPU Intel Ultra/Ryzen 8000, RAM, GPU) mediante scripts optimizados ejecutados en el backend.
- **Detección de Pantalla**: Obtención de dimensiones físicas reales ignorando el escalado de DPI de Windows para una visualización pixel-perfect.

### 🎥 Gestión de Contenido Dinámico
- **Custom Assets**: Los administradores pueden subir videos personalizados que se almacenan de forma persistente en `%APPDATA%/zenit/custom_videos/`.
- **Protocolos Seguros**: Uso de las APIs de sistema de archivos de Tauri v2 para cargar contenido local pesado de forma eficiente sin comprometer la seguridad.

---

## 🛠️ Requisitos del Desarrollo

- **S.O.**: Windows 10/11 con **Webview2** instalado.
- **Entorno**: [Node.js](https://nodejs.org/) v20+ y [Rust](https://www.rust-lang.org/) (Stable).
- **Herramientas de Build**: Visual Studio Build Tools con soporte para C++.

---

## 🚀 Instalación y Ejecución

### 1. Preparación
```powershell
npm install
```

### 2. Modo Desarrollo
Ejecuta la interfaz de Vue y el backend de Tauri simultáneamente con HMR (Hot Module Replacement):
```powershell
npm run tauri:dev
```

### 3. Compilación de Producción
Genera un bundle optimizado y un instalador NSIS profesional:
```powershell
npm run tauri:build
```

---

## 📁 Estructura del Proyecto

- **`src-tauri/`**: El "cerebro" en Rust. Gestiona ventanas, seguridad, PowerShell y APIs de sistema.
- **`src/`**: Aplicación de frontend basada en Vue 3 y Pinia.
- **`public/`**: Assets estáticos y recursos base.
- **`dist_app/`**: Build de producción del frontend (consumido por Tauri).
- **`*.ps1`**: Scripts de automatización y telemetría distribuidos como recursos externos.

---

## 🔒 Acceso Administrativo
Para editar la configuración o cambiar los videos en modo exposición, utiliza el **Hotspot invisible** en la esquina superior derecha e introduce la clave maestra "zenit", Hotspot en la parte inferior derecha para cerrar Zenit.

---

> **Zenit** es una evolución tecnológica que prioriza la estabilidad en entornos Retail 24/7. Construido con ❤️ por el equipo de **Zenit**.
