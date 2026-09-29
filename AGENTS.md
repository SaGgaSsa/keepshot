# KeepShot

Herramienta de capturas de pantalla open source, moderna y liviana, pensada como reemplazo de Lightshot. Diferencial: **historial local** de capturas (sin nube) con **anotaciones re-editables**, y una estética cuidada al estilo CleanShot X.

Plataformas: **Windows y Linux x86_64 (X11 y Wayland)**. macOS no es objetivo (no hay equipo para probar), pero no hay que cerrarle la puerta.

## Alcance del MVP

### Captura
- Hotkey global, sin botón intermedio: se congela la pantalla y se arrastra para seleccionar una región. Click sin arrastrar o Enter captura la pantalla completa del monitor.
- **Multimonitor bien resuelto**: todos los monitores a la vez, con distinto escalado de DPI y coordenadas negativas. Es el punto donde fallan las otras apps y es prioridad.
- La selección se puede reajustar (mover y redimensionar con handles) antes de confirmar.

### Editor de anotaciones (sobre la selección, en el mismo overlay)
- Herramientas: flecha (recta y **curva**, con punto de control), rectángulo, elipse, línea, texto, **resaltador tipo marcador real** (trazo semitransparente, multiply-like), **blur/pixelado** para tapar datos, numeración de pasos (1, 2, 3...).
- Selector de color y grosor.
- Undo/redo.
- Acciones: copiar al portapapeles (Ctrl+C), guardar a archivo (Ctrl+S), cerrar (Esc).

### Historial
- Se guardan las últimas ~30 capturas en disco con rotación automática.
- Hotkey (default Ctrl+Impr Pant) abre un panel rápido con las últimas capturas.
- Desde el historial se puede copiar, guardar o **reabrir en el editor con las anotaciones editables**. Para eso cada captura guarda: imagen original + JSON de capas de anotación + render final PNG.

### Configuración mínima
- Hotkeys (captura e historial), carpeta de guardado, iniciar con el sistema.
- Ícono en la bandeja del sistema.
- Aviso en el primer uso: en Windows 11, Impr Pant abre Recortes por defecto y hay que desactivarlo en Configuración > Accesibilidad > Teclado.

### Fuera de alcance (no agregar sin discutir)
Grabación de video/GIF, OCR, subida a la nube, scroll capture, historial de portapapeles de texto, pin to screen.

## Stack

- **Tauri 2** (backend en Rust, frontend web). Binario liviano, UI moderna con CSS y camino a Linux.
- Frontend: **TypeScript + Svelte 5 (SvelteKit en modo SPA con `adapter-static`)**. Anotaciones sobre **Canvas 2D** propio o **Konva**; decidir en la primera spike.
- Captura: crate **`xcap`** (Windows y Linux, incluido Wayland vía portal).
- Plugins de Tauri: `global-shortcut`, `clipboard-manager`, `autostart`, `store` (config), tray integrado.
- Persistencia: archivos en el app data dir (PNG + JSON). Sin base de datos salvo que haga falta.

## Estructura

- `src/` — frontend SvelteKit (SPA, sin SSR).
- `src-tauri/` — backend Rust (`src/lib.rs` es el entry point de la app).
- `site/` — landing page estática (HTML/CSS/JS sin build) publicada en GitHub Pages por `.github/workflows/pages.yml` en cada push a `main` que toque `site/`. La descarga y las release notes se leen de la API de GitHub Releases en el navegador; las capturas van en `site/img/` (`editor.webp`, `history.webp`) y se ocultan si faltan.
- `docs/design/DESIGN.md` — sistema de diseño (tokens, componentes, elevación).
- `docs/design/stitch-reference.html` — mockup exportado de Google Stitch, **solo local (en `.gitignore`, no se versiona)**. Puede no existir en otros clones. **Es solo referencia visual, no código real**: no copiar su Tailwind por CDN ni su estructura; tomar de ahí la intención visual y reimplementar con los tokens propios.

## Arquitectura y decisiones técnicas clave

- **Latencia**: el overlay tiene que aparecer en <150 ms desde el hotkey. Pre-crear las ventanas de overlay ocultas al iniciar y solo mostrarlas; no crearlas en cada captura.
- **Flujo de captura**: 1) capturar todos los monitores en Rust; 2) mostrar **una sola ventana overlay** borderless, always-on-top, que cubre el rectángulo envolvente del escritorio virtual (como Lightshot), con el frame congelado de cada monitor posicionado en su lugar; 3) seleccionar y anotar; 4) componer el resultado.
- **DPI**: la app debe ser Per-Monitor DPI Aware v2. Trabajar internamente en píxeles físicos y convertir a lógicos solo para la UI. Probar siempre con monitores de escalados distintos (100% + 150%).
- **Selección entre monitores**: decidido — la selección puede cruzar monitores (una sola ventana). Por ahora solo se soporta escala 100%; escalas mixtas quedan pendientes (la conversión CSS↔físico está centralizada en `src/lib/overlay/geometry.ts`).
- **Transferencia de imágenes al webview**: no usar base64 para capturas grandes; servirlas por protocolo custom de Tauri o asset protocol.
- **Linux**: la UI usa XWayland en sesiones Wayland para conservar la ventana overlay global; `xcap` captura por portal en Wayland, los atajos se lanzan con `--capture`/`--history` desde la configuración del escritorio y los protocolos custom usan `scheme://localhost`.

## Dirección visual

Ver `docs/design/DESIGN.md` ("Obsidian Frost"). Referencia: CleanShot X. Minimalista, toolbar flotante con bordes redondeados, sombras suaves, animaciones cortas, modo claro/oscuro según el sistema.
- En Windows 11 usar efectos **Mica/Acrylic** (soportados por Tauri con `windowEffects`) para el historial y la configuración.
- En Linux el blur depende del compositor: usar fallback de fondo sólido semitransparente.
- Definir tokens de diseño (colores, radios, espaciados) como CSS custom properties a partir de `DESIGN.md` antes de construir las pantallas.

## Convenciones

- Código, commits, issues y README en **inglés** (proyecto open source). La conversación de desarrollo puede ser en español.
- Commits convencionales (`feat:`, `fix:`, `chore:`...).
- Rust: `cargo fmt` + `clippy` sin warnings. TS: ESLint + Prettier.
- Licencia: MIT.

## Plan de trabajo inicial

1. **Spike de captura multimonitor**: Tauri + `xcap`, capturar todos los monitores y mostrar el overlay por monitor con DPI mixto. Medir latencia.
2. Selección de región + copiar/guardar (el "Lightshot mínimo").
3. Editor: modelo de capas en JSON, herramientas básicas, undo/redo.
4. Herramientas avanzadas: flecha curva, marcador, blur, numeración.
5. Historial en disco + panel rápido + reabrir para editar.
6. Tray, hotkeys configurables, autostart, onboarding de Impr Pant.
7. Pulido visual, empaquetado (instalador MSI/NSIS, winget).
8. Etapa 2: Linux (X11 y Wayland).

Ante dudas de alcance, preferir lo simple y preguntar antes de sumar features.
