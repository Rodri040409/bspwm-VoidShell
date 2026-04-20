# AGENTS

## Arquitectura

El proyecto está dividido por responsabilidades:

- `src/main.rs`: bootstrap de la app libadwaita.
- `src/window.rs`: ventana principal, acciones globales, palette y coordinación de estado.
- `src/terminal_pane.rs`: panel VTE real, PTY, chrome contextual y spawn del shell.
- `src/layout.rs`: árbol binario de mosaicos y rebuild sobre `gtk::Paned`.
- `src/config.rs`: configuración persistente en TOML.
- `src/preferences.rs`: diálogo de preferencias.
- `src/context.rs`: detección contextual por `procfs`, SSH, contenedores y git.
- `src/history.rs`: persistencia de sesiones, directorios, acciones y conexiones recientes.
- `src/quick_actions.rs`: registro y generación de quick actions.
- `src/banner.rs` y `src/system_info.rs`: banner inicial y recolección de info del sistema.
- `src/theme.rs`: CSS y paleta terminal.
- `src/util.rs`: helpers compartidos.

## Decisiones clave

- Se usa `vte4` para garantizar una PTY real y comportamiento normal de terminal.
- El layout es un árbol binario simple, fácil de extender a persistencia y restore de layout.
- La detección contextual se apoya en `/proc`, el foreground process group del PTY y lecturas puntuales de `git`.
- La personalización visual se hace con CSS + overlays externos al VTE, evitando hacks frágiles sobre el contenido del terminal.
- El banner inicial se imprime con un wrapper POSIX antes de `exec` al shell interactivo real.

## Puntos de extensión

- Persistencia/restauración de layout completo.
- Workspaces o tabs lógicos encima del árbol de panes.
- Integración más profunda con `tmux`.
- SSH config avanzada y selección con metadata.
- Toolbox/distrobox con detección más precisa por namespaces.
- Notificaciones al terminar comandos usando shell integration u OSC 133.
- Renombrado de paneles y overview/minimap.

## MVP vs siguiente fase

MVP implementado:

- Terminal real con PTY
- Mosaicos, atajos, preferencias persistentes, historial contextual y palette

Siguiente fase recomendada:

- Restauración de sesión/layout
- Más animaciones de cierre/apertura
- Mejor shell integration para cwd/comando terminado
- Perfilado visual por contexto y perfiles de terminal
