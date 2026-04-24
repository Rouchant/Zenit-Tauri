# 🚀 Zenit - Kiosk Framework (Tauri v2 Edition)

<img src="public/assets/logo.png" alt="Zenit Logo" width="200">

![Version](https://img.shields.io/badge/version-1.1.4-blue.svg)
![Tauri](https://img.shields.io/badge/framework-Tauri%20v2-FFC131.svg)
![Rust](https://img.shields.io/badge/backend-Rust-orange.svg)
![Vue 3](https://img.shields.io/badge/frontend-Vue%203-42b883.svg)
![Windows](https://img.shields.io/badge/OS-Windows%2010%2F11-0078D4.svg)

---

## 💡 ¿Alguna vez te ha pasado que en tu tienda de venta de computadores no encuentras una forma de mostrar de forma resumida el hardware de tu equipo?

**Zenit lo hace automático y nativo.** 

Zenit es una solución de nivel empresarial para **Showcase Terminals**, diseñada específicamente para equipos de exhibición en puntos de venta (Retail). Olvídate de configurar manualmente las specs de cada equipo; Zenit detecta el hardware en tiempo real y lo presenta de una forma visualmente impactante y profesional.

---

## ✨ Características Principales

### 🖥️ Detección de Hardware Nativa (100% Rust & WMI)
Zenit ha migrado su motor de telemetría a Rust nativo para una velocidad y fiabilidad sin precedentes:
- **Procesador (CPU)**: Identificación exacta con limpieza automática de marcas registradas. Soporte para Intel Core Ultra, Series 1/2 y Ryzen AI.
- **Gráficos (GPU)**: Identificación de modelos dedicados (RTX, RX) e integrados con prioridad inteligente.
- **Placa Base (PC Armados)**: Detección automática de Motherboard (Win32_BaseBoard) cuando el sistema reporta información genérica ("To be filled").
- **Almacenamiento Comercial**: Suma automática de discos con redondeo comercial inteligente (ej. >872GB -> 1TB).
- **Resolución Real (Triple Fallback)**: Sistema de detección robusto (VideoController -> DesktopMonitor -> Full HD) con etiquetas inteligentes (2K, 2.8K, 4K, etc.).
- **SO Detallado**: Muestra la versión exacta de Windows (Home, Pro, Single Language).

### 🏷️ Personalización Comercial (E-Commerce Ready)
- **Prioridad de Modelo**: Configura un "Nombre Completo" personalizado para sobrescribir la detección automática.
- **Gestión de SKU**: Código de producto integrado, ahora centralizado en la pestaña "Tienda".
- **Precios Dinámicos**: Configuración de precios de oferta y normales con jerarquía visual ajustable.
- **Branding de Retail**: Soporte para logos de retails (**Falabella, Paris, Ripley**) y marcas líderes.

### 🎥 Gestión Multimedia "Premium"
- **Bóveda de Videos**: Gestor inteligente con almacenamiento local persistente.
- **Alias de Marketing**: Asignación de nombres estéticos a archivos de video.
- **Inactividad Visual**: Ocultamiento automático del cursor y forzado de brillo al 100% durante la reproducción.

---

## 🔒 Seguridad y Kiosko Inteligente

### ⏱️ Monitoreo de Inactividad Global
- **Hardware Polling**: Utiliza APIs nativas de Windows para monitorear mouse y teclado en todo el sistema.
- **Salvapantallas Inteligente**: Regresa al video promocional tras el tiempo configurado de inactividad total.

### 🛡️ Modo Kiosko Robusto
- **Anti-Focus Theft**: Mantiene la aplicación siempre al frente, bloqueando intentos de minimizarla.
- **Ventana de Retorno Compacta**: Interfaz flotante para volver a Zenit rápidamente después de probar el equipo.

---

## 🚀 Instalación y Desarrollo

### Requisitos
- Windows 10/11 con **Webview2**.
- [Node.js](https://nodejs.org/) v20+.
- [Rust](https://www.rust-lang.org/) (Stable).

### Comandos Rápidos
```powershell
# Instalar dependencias
npm install

# Modo Desarrollo (HMR)
npm run tauri dev

# Compilar para Producción (Crea instalador NSIS)
npm run tauri build
```

---

## 📁 Estructura del Proyecto

- **`src-tauri/`**: Backend en Rust (Detección de Hardware, Seguridad, APIs de Sistema).
- **`src/`**: Aplicación Frontend (Vue 3, Pinia).
- **`public/assets/`**: Catálogo de logos y recursos estáticos.

---

## 🔑 Acceso Administrativo
Ajusta los precios, SKU, videos y logos mediante el **panel oculto**. Para acceder, utiliza el **Hotspot invisible** en la esquina superior derecha (hacer 4 clics rápidos) e introduce la clave maestra (**"demo"**). Existe otro Hotspot en la esquina inferior derecha (4 clics) para cerrar la aplicación.

---

> **Zenit** no es solo un software de vitrina, es la herramienta de ventas definitiva para el retail tecnológico. Construido con ❤️ para entornos 24/7.
