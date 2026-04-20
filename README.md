# VoidShell

VoidShell es un emulador de terminal real escrito en Rust con `GTK4 + libadwaita + VTE`. Cada panel corre una PTY real, el layout usa un árbol binario simple, y la capa de UI añade contexto visual, historial, command palette y control rápido de paneles.

## Qué trae ahora

- PTY real por panel usando `vte4`
- Splits horizontales y verticales
- Reordenado de paneles por teclado y drag & drop desde el header del panel
- Zoom/fullscreen por panel sin fullscreen global de la ventana
- Preferencias persistentes en `~/.config/io.github/voidscripter/VoidShell/config.toml`
- Historial persistente de sesiones, directorios, acciones, conexiones y comandos vistos
- Banner ASCII reutilizable con distro, kernel, GNOME, CPU, RAM, GPU, IP local e IP pública
- Detección de GPU híbrida más útil para laptops Intel + NVIDIA/AMD
- Contexto por panel: cwd, host, shell, proceso foreground, SSH, contenedor y rama git
- Command palette con acciones rápidas, historial y comandos internos tipo `:info`, `:zoom`, `:swap left`, `:theme red`
- Paletas por panel con presets rápidos: `red`, `green`, `blue`, `amber`, `rose`, `cyan`
- Modo compacto automático cuando hay muchos paneles para reducir costo visual y mantener fluidez
- Fondo compartido a nivel de ventana, con paneles translúcidos encima en vez de una copia independiente por panel
- Integración de Readline para shells tipo bash con búsqueda/autocompletado desde historial
- Logo e icono propios con `assets/branding/logo.png` y tamaños de app en `assets/icons/hicolor/...`

## Dependencias del sistema

En Linux necesitas tener disponibles `GTK4`, `libadwaita` y `VTE GTK4` desde el gestor de paquetes de tu distro. En Fedora, por ejemplo:

```bash
sudo dnf install \
  rust cargo gcc pkgconf-pkg-config \
  gtk4-devel libadwaita-devel vte291-gtk4-devel graphene-devel
```

Para detección ampliada de hardware, quick actions e IP pública:

```bash
sudo dnf install pciutils git openssh-clients curl docker podman htop neovim
```

## Build y ejecución

```bash
cargo check
cargo run
```

## Atajos principales

- `Alt+T`: nuevo panel
- `Alt+H`: split horizontal
- `Alt+V`: split vertical
- `Alt+Q`: cerrar panel
- `Alt+Arrow`: mover foco
- `Alt+Shift+Arrow`: redimensionar split
- `Ctrl+Alt+Arrow`: intercambiar el panel activo con el vecino más cercano
- `Alt+Shift+Enter`: zoom/fullscreen del panel activo
- `Alt+Enter` o `F11`: fullscreen de la ventana
- `Alt+I`: imprimir el banner ASCII con info en el panel activo
- `Alt+C`: copiar
- `Alt+P`: pegar
- `Alt+,`: preferencias
- `Alt+R`: recargar configuración
- `Alt+1..9`: enfocar panel por índice
- `Alt+Space` o `Ctrl+Shift+P`: command palette

## Comandos internos de la palette

Abre la palette y escribe comandos como estos:

```text
:info
:zoom
:swap left
:swap right
:swap up
:swap down
:theme red
:theme green
:theme default
```

También puedes:

- Arrastrar un panel desde su header y soltarlo sobre otro para intercambiarlos.
- Hacer doble clic en el header de un panel para alternar su zoom.

## Historial y autocompletado

VoidShell no reemplaza el historial del shell, pero sí mejora la integración para shells basados en Readline:

- `Tab`: completion normal del shell, incluyendo rutas y directorios para comandos como `cd`
- `Alt+/`: completar desde historial previo del shell
- `Shift+Tab`: vuelve a disparar completion normal
- `ArrowUp` y `ArrowDown`: búsqueda incremental por prefijo dentro del historial

Esto se inyecta sin tocar tu `~/.bashrc`; VoidShell genera un `INPUTRC` temporal y sigue incluyendo tu configuración existente si ya usabas `~/.inputrc` o `INPUTRC`.

## Archivos usados

Branding e iconos:

```text
assets/branding/logo.png
assets/icons/hicolor/512x512/apps/io.github.voidscripter.TermVoid.png
assets/icons/hicolor/256x256/apps/io.github.voidscripter.TermVoid.png
assets/icons/hicolor/128x128/apps/io.github.voidscripter.TermVoid.png
assets/icons/hicolor/64x64/apps/io.github.voidscripter.TermVoid.png
```

Configuración:

```text
~/.config/io.github/voidscripter/TermVoid/config.toml
```

Historial:

```text
~/.local/state/io.github/voidscripter/TermVoid/history.json
```

Caché de IP pública e integración de Readline:

```text
~/.local/state/io.github/voidscripter/TermVoid/public-ip.txt
~/.local/state/io.github/voidscripter/TermVoid/readline.inputrc
~/.local/state/io.github/voidscripter/TermVoid/bashrc
```

El nombre visible ya es `VoidShell`, pero las rutas siguen usando `TermVoid` para no romper compatibilidad con instalaciones previas.

## Repo

Código publicado en:

```text
https://github.com/Rodri040409/bspwm-VoidShell.git
```

## Notas de rendimiento

- Cuando hay muchos paneles, VoidShell entra en modo compacto y reduce chrome secundario, badges y wallpapers no esenciales.
- El refresco contextual de paneles inactivos se hace con menos frecuencia para bajar carga en `/proc` y en consultas de git.
- El wallpaper ahora se pinta una sola vez detrás de todos los paneles; eso reduce trabajo de render y mantiene continuidad visual.
- La IP pública se cachea para no meter una consulta de red completa en cada render del banner.

## Limitaciones actuales

- La animación del texto sigue siendo externa al VTE; lo que se anima es el contenedor del panel, su glow y sus transiciones de entrada/salida.
- El autocompletado por historial mejorado está pensado para shells tipo bash/readline; no todos los shells lo usarán igual.
- La ayuda con `sudo` es heurística y está orientada a comandos administrativos comunes en bash; no intercepta de forma perfecta cualquier binario arbitrario.
- La reorganización por mouse hoy intercambia paneles completos; todavía no existe un editor visual del árbol o arrastre libre del divisor.
- La app ya evita depender de `/proc` para parte de la info del sistema cuando no está en Linux, pero la UI sigue basada en `GTK4 + libadwaita + VTE`; eso la deja Linux-first hasta hacer una capa de terminal realmente multiplataforma.
