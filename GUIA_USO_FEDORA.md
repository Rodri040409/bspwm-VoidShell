# Guía de uso ideal en Fedora

Esta guía describe cómo está funcionando **VoidShell** en este equipo Fedora GNOME y cuál es la forma correcta e ideal de usarlo hoy, según la implementación actual del proyecto.

## Estado actual en este equipo

En esta máquina ya existe una integración real con GNOME:

- Hay un atajo global `Super+T`.
- Ese atajo ejecuta `target/debug/termvoid`.
- También existe el launcher local `~/.local/share/applications/io.github.voidscripter.TermVoid.desktop`.
- El `.desktop` está registrado para `inode/directory`, así que VoidShell aparece como aplicación recomendada para abrir carpetas.
- Nautilus sigue siendo la app por defecto para carpetas, pero VoidShell ya se puede usar desde **Abrir con**.

En otras palabras: aquí VoidShell no está solo como binario de desarrollo; ya está integrado al escritorio y se puede invocar como terminal principal de trabajo.

## Forma ideal de usarlo

La experiencia ideal de VoidShell en Fedora GNOME es esta:

1. Presionas `Super+T`.
2. Se abre VoidShell al instante como terminal principal.
3. Trabajas en un panel único cuando quieres concentración.
4. Divides en mosaico solo cuando necesitas contexto paralelo: editor, logs, git, monitorización, shells remotos.
5. Desde Archivos/Nautilus puedes abrir una carpeta con VoidShell para arrancar directamente en ese directorio.

Ese flujo encaja bien con cómo está construido el proyecto:

- La app acepta `--working-directory`, así que puede abrirse ya posicionada en una carpeta.
- También acepta `--execute`, así que puede arrancar corriendo un comando concreto.
- El layout por paneles evita abrir varias ventanas separadas para tareas relacionadas.

## Cómo usarlo bien en el día a día

### 1. Como terminal principal con `Super+T`

En este equipo `Super+T` ya abre VoidShell. Ese es el uso más natural: una terminal inmediata, visualmente integrada con GNOME y lista para trabajar sin buscarla en el overview.

En desarrollo, el atajo apunta a:

```bash
/home/voidscripter/Escritorio/Program/Rust/Terminal/target/debug/termvoid
```

Eso es correcto para tu entorno actual. Si más adelante dejas una build release instalada en otra ruta, conviene actualizar ese atajo para que apunte al binario final.

### 2. Desde Archivos / Nautilus

Ahora mismo VoidShell ya está registrado como app compatible con carpetas. Eso permite:

- Hacer clic derecho sobre una carpeta.
- Elegir **Abrir con**.
- Seleccionar **VoidShell**.

Cuando se lanza así, la app puede tomar esa carpeta y usarla como directorio inicial del panel.

Eso funciona porque el launcher instalado declara soporte para directorios y porque la app interpreta rutas, archivos y URIs para convertirlos en `working_directory`.

### 3. Como terminal contextual para proyectos

VoidShell funciona especialmente bien cuando lo abres dentro de un proyecto real:

- Repositorio Git
- Proyecto Python con `.venv`
- Entorno SSH
- Contenedor o toolbox/distrobox

La idea del programa no es ser una caja negra, sino una terminal que entiende mejor el contexto del panel activo:

- Detecta `cwd`
- Detecta host
- Detecta shell
- Detecta proceso foreground
- Detecta Git
- Detecta SSH y contenedores
- Puede sugerir activación de entorno virtual Python

## Funcionamiento correcto del programa

### Terminal real, no simulada

VoidShell en Linux usa `VTE` con una **PTY real por panel**. Ese punto define casi todo el comportamiento correcto del programa:

- El shell se comporta como shell real.
- `bash`, `zsh`, `ssh`, `git`, `nvim`, `htop`, `less`, `tmux` y herramientas TUI funcionan como se espera.
- Copia, pegado, selección y foco pasan por una terminal nativa, no por una imitación HTML o un textbox maquillado.

Por eso se siente consistente incluso al escribir rápido o al usar herramientas intensivas dentro del terminal.

### Escritura y renderizado fluidos

La sensación de fluidez viene de varias decisiones que sí están presentes en el código actual:

- El widget central es `vte4`, no una terminal falsa renderizada como webview.
- El texto usa fuente monoespaciada real configurada por preferencias.
- `VTE` tiene shaping de texto activado en condiciones normales.
- Cuando hay muchos paneles, VoidShell entra en modo compacto/denso para bajar costo visual.
- En modo denso reduce parte de la carga visual y desactiva shaping para mantener respuesta.
- El fondo se comparte a nivel de ventana y los paneles se dibujan encima, evitando una copia pesada por cada terminal.

Eso hace que escribir se mantenga suave incluso cuando tienes varios paneles abiertos o estás trabajando dentro de herramientas como Codex, shells interactivos, editores de terminal o procesos con salida continua.

No es magia: el programa intenta preservar la respuesta del terminal antes que meter adornos pesados encima del área de texto.

### Mosaicos útiles, no decorativos

Los paneles existen para trabajo real. La funcionalidad correcta hoy es:

- Crear panel nuevo con `Alt+T`
- Split horizontal con `Alt+H`
- Split vertical con `Alt+V`
- Cerrar panel con `Alt+Q`
- Mover foco con `Alt+Flechas`
- Redimensionar con `Alt+Shift+Flechas`
- Intercambiar paneles con `Ctrl+Alt+Flechas`
- Hacer zoom del panel activo con `Alt+Shift+Enter`
- Abrir palette con `Alt+Space` o `Ctrl+Shift+P`

La lógica correcta aquí no es llenar la ventana de paneles porque sí, sino tener varios contextos vivos sin perder foco ni abrir varias ventanas sueltas.

### Palette e historial

La palette actual sirve como centro rápido de acciones:

- Comandos internos como `:info`, `:zoom`, `:swap left`, `:theme red`
- Historial de acciones
- Historial de directorios
- Comandos recientes
- Conexiones recientes

Eso vuelve más natural reutilizar flujos repetidos sin abandonar el teclado.

### Integración con shell sin romper tu entorno

VoidShell también inyecta integración para shells tipo `bash`, pero sin modificar directamente tu `~/.bashrc`.

El comportamiento correcto aquí es:

- Mantener tu shell real
- Añadir mejoras temporales de readline
- Publicar el estado del entorno virtual Python
- Permitir búsqueda por historial con flechas
- Mantener autocompletado útil para rutas y directorios
- Mostrar confirmación para ciertos comandos normalmente asociados a `sudo`

La clave es que la app mejora la sesión interactiva sin apropiarse de la configuración permanente del usuario.

## Ejemplos de uso ideal

### Abrir la terminal general

```bash
Super+T
```

Uso esperado:

- Arranca VoidShell como terminal principal del sistema.

### Abrir una carpeta concreta con VoidShell

Desde Nautilus:

- Clic derecho en una carpeta
- `Abrir con`
- `VoidShell`

Uso esperado:

- El panel inicial abre en esa carpeta.

### Abrir VoidShell ya ubicado en un proyecto

```bash
termvoid --working-directory /ruta/al/proyecto
```

Uso esperado:

- La terminal inicia directamente en el proyecto.

### Abrir y ejecutar un comando al arrancar

```bash
termvoid --working-directory /ruta/al/proyecto --execute "git status"
```

Uso esperado:

- Se abre la terminal en esa carpeta.
- El comando corre dentro del panel real.

## Qué describe correctamente a VoidShell hoy

La descripción más fiel del programa, en su estado actual, sería esta:

> VoidShell es una terminal real para Fedora GNOME, con PTY por panel, mosaicos prácticos, contexto visual, integración con directorios y un flujo orientado a abrirse rápido, escribir fluido y mantener varias tareas relacionadas dentro de una sola ventana.

No está planteado como una demo visual ni como una terminal “fake”. Su comportamiento correcto se apoya en:

- `GTK4 + libadwaita`
- `VTE` en Linux
- Shell real
- PTY real
- Atajos rápidos
- Contexto por panel
- Integración con el escritorio

## Recomendación práctica para tu setup

Tu setup actual ya va en la dirección correcta. La forma ideal de dejarlo estable sería:

- Mantener `Super+T` como entrada principal a VoidShell.
- Seguir usando Nautilus como explorador, pero abrir carpetas en VoidShell cuando el objetivo sea trabajar en terminal.
- Apoyarte en splits solo para tareas simultáneas reales.
- Usar la palette para acciones repetidas y cambios rápidos de contexto.
- Cuando pases a una versión más estable, cambiar el atajo y el launcher para apuntar a una build release en vez de `target/debug/termvoid`.

## Archivos relevantes en este equipo

- Configuración: `~/.config/io.github/voidscripter/TermVoid/config.toml`
- Historial: `~/.local/state/io.github/voidscripter/TermVoid/history.json`
- Launcher local: `~/.local/share/applications/io.github.voidscripter.TermVoid.desktop`
- Atajo global actual en GNOME: `Super+T -> /home/voidscripter/Escritorio/Program/Rust/Terminal/target/debug/termvoid`
