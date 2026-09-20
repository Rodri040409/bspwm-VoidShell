# Guía de integración portable de VoidShell

Este documento describe la instalación de referencia que produce la apariencia y el comportamiento actuales en Fedora GNOME. Está dirigido a una persona o agente que deba llevar el proyecto a otro equipo sin sustituir componentes visuales o de terminal por equivalentes incompatibles.

## Resultado que se debe preservar

VoidShell es una aplicación GTK4/libadwaita escrita en Rust. No es una terminal web ni dibuja texto de terminal por cuenta propia: en Linux usa `vte4`, que aporta una PTY real, emulación de terminal y renderizado de texto. La interfaz alrededor del terminal se compone de GTK, CSS generado en tiempo de ejecución y capas `Overlay`.

La apariencia depende de cuatro capas que deben estar presentes juntas:

1. GTK4, libadwaita y VTE del sistema, con versiones compatibles.
2. Fuentes monoespaciadas instaladas y seleccionadas en la configuración.
3. Los assets del proyecto: iconos y wallpaper.
4. La configuración persistente del usuario, que define color, transparencia, tipografía, padding y animaciones.

Si una de ellas cambia, la aplicación sigue funcionando, pero puede verse muy distinta: es habitual que cambie el peso tipográfico, el antialiasing, las esquinas, el contraste, las sombras o el color real de las transparencias.

## Arquitectura de renderizado

```text
GTK Application / libadwaita
        │
        ├── MainWindow (window.rs)
        │     ├── HeaderBar, acciones y atajos
        │     ├── Overlay de fondo compartido
        │     └── layout_host
        │             └── TileTree → gtk::Paned anidados
        │                     └── TerminalPane por hoja
        │                            ├── chrome contextual GTK
        │                            ├── capas de wallpaper/tinte
        │                            └── VTE + PTY + shell real
        │
        ├── theme.rs → CssProvider de prioridad de aplicación
        ├── config.rs → TOML persistente
        └── util.rs → iconos, launcher .desktop y paths de runtime
```

El wallpaper se coloca una sola vez detrás del árbol de paneles. Cada `TerminalPane` tiene tintes y chrome propios, pero no duplica el fondo completo. Esto reduce costo visual y hace que la transparencia se perciba uniforme entre mosaicos.

`theme.rs` genera CSS a partir de `AppConfig` e instala un `CssProvider` con prioridad de aplicación. Por eso los valores de color, bordes, opacidad, padding y animaciones son consistentes dentro de la aplicación, aunque los controles base de GTK todavía responden al tema y a la versión del sistema.

## Referencia Linux/Fedora

La referencia actual utiliza Fedora con GTK4, libadwaita y VTE GTK4 del sistema. Instala los paquetes equivalentes a:

```bash
sudo dnf install rust cargo gcc pkgconf-pkg-config \
  gtk4-devel libadwaita-devel vte291-gtk4-devel graphene-devel
```

Para conservar el aspecto del texto, instala la familia configurada en `config.toml`. No asumas que una fuente existe solo porque figura en la configuración: si falta, Pango hará fallback y cambiarán métricas, densidad y alineación del terminal.

Antes de comparar estética entre equipos, iguala estos factores:

- Versión de GTK4/libadwaita y tema de escritorio.
- Sesión Wayland/X11 y escala fraccional del monitor.
- Familia de fuente, peso disponible, hinting y tamaño configurado.
- Perfil de color/driver de GPU y compositor.
- Wallpaper y valores de opacidad, superficie, borde activo y padding.

## Configuración y estado del usuario

En Linux, la configuración se guarda en:

```text
~/.config/io.github/voidscripter/TermVoid/config.toml
```

El historial se guarda en:

```text
~/.local/state/io.github/voidscripter/TermVoid/history.json
```

Para clonar la experiencia visual a otro equipo, copia el `config.toml` después de instalar las fuentes y verificar que la ruta de `ruta_fondo` exista. No copies rutas absolutas de wallpaper sin adaptarlas al nuevo usuario.

Importante: el valor inicial del wallpaper se resuelve desde `CARGO_MANIFEST_DIR` al compilar. Es adecuado para desarrollo dentro del repositorio, pero una distribución portable debe definir una ruta de wallpaper válida en el equipo destino o empaquetar el asset y resolverlo de forma relativa al binario.

## Launcher de escritorio y binario correcto

En Linux el programa instala en el arranque:

```text
~/.local/share/applications/io.github.voidscripter.TermVoid.desktop
~/.local/share/icons/hicolor/.../io.github.voidscripter.TermVoid.png
```

El archivo `.desktop` determina qué binario abre el menú del escritorio. Para una instalación de uso diario debe apuntar al binario release, no a `target/debug/termvoid`.

Flujo recomendado para desarrollo y entrega:

```bash
cargo test
cargo build --release
./target/release/termvoid
```

La primera ejecución del binario release actualiza el launcher con su ruta y copia/actualiza los iconos locales. La lógica de `util.rs` también evita que una ejecución posterior con `cargo run` reemplace ese launcher por el binario debug cuando ya existe el release hermano.

Para comprobarlo:

```bash
rg '^Exec=' ~/.local/share/applications/io.github.voidscripter.TermVoid.desktop
```

La ruta debe terminar en `target/release/termvoid` para este checkout. Si se abre un menú y siguen faltando funciones recién compiladas, esta es la primera comprobación obligatoria.

## Mosaicos: modelo y controles

`layout.rs` mantiene un árbol binario (`TileTree`). Cada hoja representa un `TerminalPane`; cada nodo interno contiene eje, proporción y dos hijos. La interfaz se reconstruye como `gtk::Paned` anidados.

- `Alt+T`: crea un panel con una heurística que evita encadenar el mismo eje bajo el mismo padre.
- `Alt+H` / `Alt+V`: división explícita horizontal / vertical.
- `Alt+O`: alterna el eje de la división más pequeña que contiene el panel activo.
- `Alt+Shift+B`: restaura todos los divisores a 50/50.
- `Alt+Flechas`: foco; `Alt+Shift+Flechas`: tamaño; `Ctrl+Alt+Flechas`: intercambio.
- Arrastrar el header de un panel y soltarlo sobre otro también intercambia las sesiones.

Al restaurar o reconstruir el árbol, GTK puede emitir posiciones provisionales para un `Paned` todavía sin asignación final. El código ignora esas posiciones hasta estabilizar la asignación; de otro modo un 50/50 puede convertirse erróneamente en 15/85.

## Portabilidad a otra distribución Linux

1. Instala los equivalentes de GTK4, libadwaita, VTE GTK4, Graphene, Rust y una toolchain C.
2. Clona el repositorio y compila con `cargo build --release`.
3. Instala las fuentes que indique el `config.toml` de referencia.
4. Copia assets si se entrega fuera del repositorio; conserva `share/icons` junto al prefijo del binario para que `runtime_icon_search_paths` pueda encontrarlos.
5. Ejecuta el binario release una vez para generar el `.desktop` y los iconos del usuario.
6. Copia/adapta `config.toml`, sobre todo la fuente y `ruta_fondo`.
7. Comprueba el `Exec=` del launcher antes de evaluar diferencias visuales.

No sustituyas `vte4` por un widget de texto ni conviertas el fondo compartido en un wallpaper por panel: ambas decisiones cambian rendimiento y estética de forma notable.

## Windows

Windows usa `src/terminal_pane_windows.rs`, no VTE ni libadwaita. Por tanto no puede ser una réplica de píxel del Linux de referencia. El script `scripts/build-windows-gnu.sh` construye un bundle que debe conservar su estructura `bin/`, `share/` y `lib/`; `main.rs` detecta ese prefijo y configura GTK, esquemas, pixbuf y PATH para usar el runtime incluido.

La adaptación correcta en Windows es preservar el bundle completo, no copiar solo `termvoid.exe`. Para acercar la estética, distribuye las mismas fuentes, wallpaper y configuración, pero acepta diferencias propias de DirectWrite, el compositor y el tema GTK disponible.

## Validación para un agente de migración

Antes de dar por terminada una migración, verificar:

```bash
cargo fmt --check
cargo test
cargo build --release
rg '^Exec=' ~/.local/share/applications/io.github.voidscripter.TermVoid.desktop
```

Además de compilar, validar visualmente: una ventana con 1, 2, 3 y 6 paneles; crear panel desde zoom; rotar con `Alt+O`; equilibrar con `Alt+Shift+B`; arrastrar un header; redimensionar en los cuatro sentidos; cerrar paneles anidados; y reiniciar desde el launcher del escritorio. La última prueba es esencial: ejecutarla desde terminal no prueba el binario que realmente abre el menú.
