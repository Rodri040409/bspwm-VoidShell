# VoidShell

VoidShell es un emulador de terminal escrito en Rust con `GTK4`. En Linux usa `libadwaita + VTE` para una PTY real por panel; en Windows usa `GTK4` puro y un backend nativo de shell mientras madura la capa multiplataforma. El layout usa un árbol binario simple y la UI añade contexto visual, historial, command palette y control rápido de paneles.

## Qué trae ahora

- PTY real por panel usando `vte4` en Linux y backend nativo de shell en Windows
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

## Plataformas y dependencias

Linux:

- Necesitas `GTK4`, `libadwaita` y `VTE GTK4` desde el gestor de paquetes de tu distro.
- El nombre exacto de los paquetes cambia entre Fedora, Arch, Debian, Ubuntu y otras distros, pero siempre necesitas los equivalentes de `gtk4`, `libadwaita`, `vte gtk4`, `graphene`, `pkg-config` y una toolchain de Rust/C.
- Ejemplo en Fedora:

```bash
sudo dnf install \
  rust cargo gcc pkgconf-pkg-config \
  gtk4-devel libadwaita-devel vte291-gtk4-devel graphene-devel
```

Para detección ampliada de hardware, quick actions e IP pública:

```bash
sudo dnf install pciutils git openssh-clients curl docker podman htop neovim
```

Windows:

- El proyecto usa el backend de `src/terminal_pane_windows.rs`, así que no depende de `VTE`.
- En Windows no depende de `libadwaita`; la UI cae a `GTK4` puro.
- Necesitas `GTK4` para `gtk4-rs` y una toolchain de Rust para Windows.
- Si vas a ejecutar el `.exe` fuera del entorno de build, también necesitas el runtime de GTK4 o un bundle con `bin`, `share` y `lib`.

## Build y ejecución

```bash
cargo check
cargo run
```

Build nativo en Windows:

```powershell
cargo build --release
```

Cross-build a Windows desde Linux con MinGW:

Necesitas una toolchain MinGW con `gtk4`. En Fedora, por ejemplo:

```bash
sudo dnf install mingw64-gcc mingw64-gtk4 pkgconf-pkg-config
```

```bash
rustup target add x86_64-pc-windows-gnu

PKG_CONFIG=x86_64-w64-mingw32-pkg-config \
PKG_CONFIG_ALLOW_CROSS=1 \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
cargo build --release --target x86_64-pc-windows-gnu
```

El `.exe` resultante queda en:

```text
target/x86_64-pc-windows-gnu/release/termvoid.exe
```

También puedes abrir VoidShell ya ubicado en una carpeta o ejecutando un comando al arrancar:

```bash
cargo run -- --working-directory /ruta/al/proyecto
cargo run -- --execute "htop"
cargo run -- --working-directory /ruta/al/proyecto --execute "git status"
```

En Windows se usan los mismos flags:

```powershell
cargo run -- --working-directory "C:\Users\tuusuario\proyecto"
cargo run -- --execute "git status"
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
- `Ctrl+Shift+A`: seleccionar todo el contenido del panel activo
- `Ctrl+Shift+C`: copiar selección del panel activo
- `Ctrl+Shift+V`: pegar en el panel activo
- `Alt+,`: preferencias
- `Alt+R`: recargar configuración
- `Alt+1..9`: enfocar panel por índice
- `Alt+Space` o `Ctrl+Shift+P`: command palette

## Accesos directos y atajos del sistema

Linux:

- VoidShell instala un `.desktop` local en `$XDG_DATA_HOME/applications` o, si no existe esa variable, en `~/.local/share/applications`.
- Ese launcher ya entiende `--working-directory` y `--execute`, así que puedes reutilizarlo o copiar su comando para tu desktop environment.
- Para crear un atajo global de teclado en GNOME, KDE, XFCE, Cinnamon, MATE, i3, sway o similares, crea un shortcut personalizado que ejecute algo como esto:

```bash
/ruta/al/binario/termvoid --working-directory /ruta/al/proyecto --execute "git status"
```

- Si no quieres arrancar un comando, deja solo `--working-directory`.

Windows:

- Crea un acceso directo a `termvoid.exe`.
- En el campo `Target` puedes dejar algo como:

```text
"C:\ruta\termvoid.exe" --working-directory "C:\ruta\proyecto" --execute "git status"
```

- Abre `Properties` del acceso directo y usa `Shortcut key` para asignarle una combinación del sistema.

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

En Windows, configuración e historial se guardan en las rutas que resuelve `ProjectDirs` bajo `AppData`.

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
- Linux tiene hoy el backend más completo porque usa `VTE` con PTY real; Windows ya está soportado, pero sigue usando un backend propio mientras madura la capa multiplataforma.
