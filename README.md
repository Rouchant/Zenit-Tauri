# 🚀 Zenit - Kiosk Framework (Tauri v2 Edition)

![Zenit Logo](public/assets/logo.png)

![Version](https://img.shields.io/badge/version-1.0.4-blue.svg)
![Tauri](https://img.shields.io/badge/framework-Tauri%20v2-FFC131.svg)
![Vue 3](https://img.shields.io/badge/frontend-Vue%203-42b883.svg)
![Vite](https://img.shields.io/badge/build-Vite%206-646cff.svg)
![Windows](https://img.shields.io/badge/OS-Windows%2010%2F11-0078D4.svg)

---

## 💡 ¿Alguna vez te ha pasado que en tu tienda de venta de computadores no encuentras una forma de mostrar de forma resumida el hardware de tu equipo?

**Zenit lo hace automático.** 

Zenit es una solución de nivel empresarial para **Showcase Terminals**, diseñada específicamente para equipos de exhibición en puntos de venta (Retail). Olvídate de configurar manualmente las specs de cada equipo; Zenit detecta el hardware y lo presenta de una forma visualmente impactante y profesional.

---

## ✨ Características Principales

### 🖥️ Detección de Hardware Inteligente
Zenit utiliza un potente motor de telemetría basado en Rust y PowerShell para extraer y mostrar de forma resumida:
- **Procesador (CPU)**: Identificación exacta (Intel Core Ultra, Ryzen AI, 8000 series, etc.).
- **Memoria RAM**: Capacidad y tipo.
- **Gráficos (GPU)**: Detección de modelos integrados y dedicados.
- **Almacenamiento**: Estado y velocidad.

### 🏷️ Personalización Comercial (E-Commerce Ready)
Sabemos que el hardware no lo es todo. Por eso Zenit te permite:
- **Precios Dinámicos**: Configura y muestra el precio actual del equipo directamente en pantalla.
- **Gestión de SKU**: Incluye el código de producto para facilitar la búsqueda en bodega o sistema de ventas.
- **Branding de Retail**: Opción única para incluir los logos de los principales retails de Chile (**Falabella, Paris, Ripley**) y las marcas líderes del mercado (**Asus, Acer, HP, Lenovo**, etc.).

### 🎥 Experiencia Visual Inmersiva
- **Videos Personalizables**: Cambia los videos de fondo y demostración según la marca o el modelo del equipo.
- **Transiciones Fluídas**: Interfaz ultra rápida y suave gracias a Vue 3 y Vite.
- **Detección de Pantalla**: Visualización pixel-perfect que ignora el escalado de Windows para asegurar que todo se vea nítido.

---

## 🛠️ Seguridad de Nivel Kiosko

- **Anti-Focus Theft**: Mantiene la aplicación siempre al frente, bloqueando intentos de minimizarla o solaparla.
- **Bloqueo Total de Atajos**: Inhabilita `Alt+Tab`, `Win+D`, `Alt+F4`, etc.
- **Auto-Restore**: Temporizador de inactividad que restaura la experiencia de usuario si el equipo es dejado en mantenimiento.

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
npm run tauri:dev

# Compilar para Producción (Crea instalador NSIS)
npm run tauri:build
```

---

## 📁 Estructura del Proyecto

- **`src-tauri/`**: Backend en Rust (Seguridad, Store, PowerShell Bridge, APIs de sistema).
- **`src/`**: Aplicación Frontend (Vue 3, Pinia for State Management).
- **`public/assets/logos/`**: Catálogo de logos de retail y marcas integrados.
- **`*.ps1`**: Scripts de telemetría personalizados.

---

## 🔒 Acceso Administrativo
Ajusta los precios, SKU, videos y logos mediante el **panel oculto**. Para acceder, utiliza el **Hotspot invisible** en la esquina superior izquierda e introduce la clave maestra de administrador (**"demo"**). Existe otro Hotspot en la esquina inferior derecha para cerrar Zenit.

---

> **Zenit** no es solo un software de vitrina, es la herramienta de ventas definitiva para el retail tecnológico. Construido con ❤️ para entornos 24/7.
