# KustomLinux — Live Wallpaper Engine for Arch Linux

## Project Vision

Build **KustomLinux**, a desktop live wallpaper engine inspired by [KLWP (Kustom Live Wallpaper)](https://docs.kustom.rocks/docs/reference/functions/) for Android. The app has three core components:

1. **Editor** — A visual drag-and-drop editor to design wallpapers with layers, formulas, animations, and data bindings
2. **Viewer/Renderer** — A real-time rendering engine that displays the live wallpaper
3. **System Integration** — Sets the rendered output as the actual desktop wallpaper on Arch Linux (X11 via `xwinwrap`/`xwallpaper`, Wayland via `swww`/`swaybg`/`hyprpaper`/`mpvpaper`)

Additionally, it must support **importing existing `.klwp` preset files** (designed for Android) and **exporting them as working Linux wallpapers**.

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| GUI Framework | **Tauri v1** (Rust backend + Svelte frontend) |
| Renderer | **Canvas API** (2D) + **WebGL** (shader effects) in the Svelte webview |
| System wallpaper | Rust backend sets wallpaper via platform-specific tools |
| Formula engine | Rust-based parser & evaluator (performance-critical) |
| KLWP import | Rust-based `.klwp` file parser (ZIP archive with XML preset + assets) |
| Data providers | Rust backend fetches system data (battery, weather, music, etc.) and pushes to frontend via Tauri events |
| Config/state | JSON project files (`.klx` — KustomLinux format) |
| Package manager | `pnpm` (frontend), `cargo` (backend) |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                  Svelte Frontend                │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐ │
│  │  Editor   │  │  Viewer   │  │  Preview    │ │
│  │  (drag &  │  │  (canvas  │  │  (real-time │ │
│  │   drop)   │  │  render)  │  │   update)   │ │
│  └─────┬─────┘  └─────┬─────┘  └──────┬──────┘ │
│        │              │               │         │
│  ┌─────┴──────────────┴───────────────┴───────┐ │
│  │         Formula Engine (WASM)              │ │
│  │   Parses $formula$ → evaluated values      │ │
│  └─────────────────┬──────────────────────────┘ │
└────────────────────┼────────────────────────────┘
                     │ Tauri IPC (invoke/events)
┌────────────────────┼────────────────────────────┐
│              Rust Backend                       │
│  ┌─────────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ Data        │ │ KLWP     │ │ Wallpaper    │ │
│  │ Providers   │ │ Importer │ │ Setter       │ │
│  │ (battery,   │ │ (.klwp   │ │ (X11/Wayland │ │
│  │  weather,   │ │  parser) │ │  integration)│ │
│  │  music..)   │ │          │ │              │ │
│  └─────────────┘ └──────────┘ └──────────────┘ │
└─────────────────────────────────────────────────┘
```

---

## Part 1: Formula Engine

The heart of KLWP is its formula system. All dynamic content is expressed as `$formula$` strings that evaluate in real-time.

### Formula Syntax

- Formulas are enclosed in `$...$` delimiters
- Functions use the format: `category(parameter, ...)`
- Operators: `+`, `-`, `*`, `/`, `=`, `!=`, `>`, `>=`, `<`, `<=`, `&` (AND), `|` (OR)
- String literals use double quotes inside formulas
- Nesting is supported: `$if(df(h) > 12, "PM", "AM")$`

### Function Categories to Implement

Implement ALL of the following function categories. Each maps to a data provider in the Rust backend.

#### Date & Time
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Date Format | `df(format, [date])` | Format dates with patterns (h, mm, ss, d, M, EEEE, etc.) | `chrono` crate |
| Date Parser | `dp(date_string, format)` | Parse date strings into dates | `chrono` crate |
| Time Span | `tf(seconds, format)` | Format durations (mm:ss, HH:mm:ss) | `chrono` crate |
| Timer Utilities | `tu(mode, ...)` | Sequence timers, countdowns | `std::time` |

**DF format codes:** `h/hh` (hour 12h), `H/HH` (hour 24h), `m/mm` (minute), `s/ss` (second), `a/A` (AM/PM), `d/dd` (day), `M/MM/MMM/MMMM` (month), `EEEE/EEE/E` (weekday), `D/DDD` (day of year), `w` (week of year), `e/f` (day of week number), `yyyy/yy` (year).

**Date offsets:** `a1d` (add 1 day), `r2h` (remove 2 hours), relative date math.

#### Weather
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Current Weather | `wi(field)` | Current conditions | OpenWeatherMap / wttr.in API |
| Weather Forecast | `wf(day, field)` | Multi-day forecast | OpenWeatherMap API |

**WI fields:** `temp`, `tempc`, `flik` (feels like), `dpoint`, `fpoint`, `cond` (condition text), `icon` (CLEAR/RAIN/SNOW/CLOUDY/etc.), `code` (detailed code), `wspeed`, `wspeedm`, `wchill`, `wdir`, `press`, `hum`, `clouds`, `uvindex`, `updated`, `provider`.

**WF fields:** Same as WI but indexed by day: `wf(0, temp)` = today, `wf(1, icon)` = tomorrow.

#### System & Device
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Battery Info | `bi(field)` | Battery status | `/sys/class/power_supply/` or UPower D-Bus |
| System Info | `si(field)` | System information | Various Linux APIs |
| Resource Monitor | `rm(field)` | CPU, RAM, disk | `/proc/stat`, `/proc/meminfo`, `statvfs` |
| Network Connectivity | `nc(field)` | Network status | NetworkManager D-Bus |

**BI fields:** `level` (0-100), `temp`, `status` (CHARGING/DISCHARGING/FULL), `plugged`, `health`.

**SI fields (Linux-adapted):**
- `model` → hostname, `man` → distro name, `build` → kernel version, `aver` → distro version
- `alarmd/alarmt/alarmon` → next systemd timer or KDE/GNOME alarm
- `land` → monitor orientation, `swidth/sheight/sdpi` → screen dimensions
- `locked` → session lock status, `darkmode` → GTK/KDE dark mode
- `volr/vola` → PulseAudio/PipeWire volume, `ringer` → mute status
- `boot` → system boot time from `uptime`

**RM fields:** `cpuuse` (CPU %), `memtot/memfree/memuse` (RAM), `swptot/swpfree`, `sdtot/sdfree` (disk).

#### Music & Media
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Music Info | `mi(field)` | Current track info | MPRIS D-Bus interface |
| Music Queue | `mq(index, field)` | Playlist/queue data | MPRIS D-Bus |

**MI fields:** `title`, `artist`, `album`, `len` (duration secs), `pos` (position secs), `percent`, `vol` (0-100), `cover` (album art path/URL), `state` (PLAYING/PAUSED/STOPPED), `track`, `package` (player name).

#### Location & Astronomy
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Location Info | `li(field)` | GPS/location data | GeoClue D-Bus or IP geolocation |
| Astronomical Info | `ai(field)` | Sun/moon/zodiac data | Calculated from lat/lon + date |

**AI fields:** `sunrise/sunset`, `csunrise/csunset` (civil twilight), `nsunrise/nsunset` (next sunrise/set), `isday`, `mphase` (moon phase name), `mphasec` (moon phase code: NEW/FULL/etc.), `moonrise/moonset`, `mage`, `mill` (illumination %), `zodiac/zodiacc`, `season/seasonc`.

#### Text & Math
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Text Converter | `tc(mode, text, ...)` | Text manipulation | Pure computation |
| Math Utilities | `mu(func, ...)` | Math operations | Pure computation |
| If Conditions | `if(cond, then, [else])` | Conditional logic | Pure computation |
| For Loops | `fl(init, stop, incr, body, [sep])` | Iteration | Pure computation |

**TC modes:** `low`, `up`, `cap` (case), `cut` (substring), `ell` (ellipsis truncate), `split` (split by delimiter), `len` (length), `count` (char count), `lines` (line count), `reg` (regex replace), `json` (JSONPath), `html` (strip tags), `url` (URL encode), `fmt` (format), `n2w` (number to words), `ord` (ordinal: 1st/2nd), `roman` (roman numerals), `utf` (unicode char), `asort/nsort` (sort).

**MU functions:** `ceil`, `floor`, `round`, `abs`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `log`, `ln`, `pow`, `sqrt`, `min`, `max`, `rnd` (random), `h2d`/`d2h` (hex conversion).

#### Color
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Color Editor | `ce(color, filter, [amount])` | Color manipulation | Pure computation |
| Color Maker | `cm(mode, ...)` | Generate colors | Pure computation |
| Bitmap Palette | `bp(image, type)` | Extract colors from images | Image processing |

**CE operations:** `invert`, `comp` (complementary), `contrast` (black/white), `alpha` (transparency), `sat` (saturation), `lum` (luminance), gradient mixing between two colors.

#### Variables
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Global Variables | `gv(name)` | User-defined persistent vars | App state (JSON) |
| Local Variables | `lv(name)` | Component-scoped vars | Component state |

#### Data & Network
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Calendar Events | `ci(index, field)` | Calendar data | ICS files or D-Bus (GNOME/KDE) |
| Web Get | `wg(url, format, path)` | Fetch web data | `reqwest` crate |
| Shell Command | `sh(command)` | Execute shell commands | `std::process::Command` |
| Notification Info | `ni(field)` | Desktop notifications | D-Bus notification listener |
| Air Quality | `aq(field)` | Air quality index | API (OpenWeatherMap/AQICN) |
| Traffic Stats | `ts(field)` | Network traffic | `/proc/net/dev` |
| Unread Counters | `uc(source)` | Unread email/messages | Plugin system |

#### Linux-Specific Extensions (not in KLWP)
| Function | Code | Description | Linux Source |
|----------|------|-------------|-------------|
| Window Manager | `wm(field)` | WM/compositor info | X11/Wayland APIs |
| Workspaces | `ws(field)` | Virtual desktop info | WM-specific IPC (i3/Hyprland/Sway) |
| System Theme | `th(field)` | GTK/Qt theme colors | gsettings / kdeglobals |
| Package Manager | `pm(field)` | Package update count | pacman/yay |

---

## Part 2: Layer/Item System

KLWP wallpapers are composed of a tree of **items** (layers). Implement the following item types:

### Item Types

| Type | Description | Properties |
|------|-------------|-----------|
| **Shape** | Rectangle, circle, oval, triangle, arc, SVG path | fill color, stroke, corner radius, SVG path data |
| **Text** | Dynamic text with formula support | font family, size, color, alignment, shadow, max lines, line spacing |
| **Image** | Static or dynamic images | source (file/URL/formula), tint, scale mode (fit/fill/crop), filters |
| **Komponent** | Reusable component group | inputs (globals), encapsulated layer tree |
| **Stack Group** | Vertical/horizontal layout container | orientation, spacing, alignment |
| **Overlap Group** | Z-stacked layers | children overlap at same position |
| **Scroll Group** | Scrollable container | scroll direction, page count |
| **FontIcon** | Icon from icon fonts | icon set, glyph code, size, color |
| **Progress** | Arc/bar/circle progress indicators | min, max, value (formula), style |
| **Morphing Shape** | Shape that animates between states | path keyframes |

### Common Item Properties

Every item has:
- **Position**: `x`, `y` (relative to parent or anchor point)
- **Size**: `width`, `height` (pixels, percentage, or `wrap_content`)
- **Rotation**: degrees
- **Scale**: x-scale, y-scale
- **Opacity**: 0-255
- **Anchor**: alignment point (center, top-left, etc.)
- **Visibility**: visible/invisible/gone (formula-driven)
- **Padding/Margin**: spacing
- **Touch action**: on-tap behavior (launch app, toggle global, open URL, etc.)
- **Formulas**: ANY property can be driven by a `$formula$` expression

### Animations

Items can have animations triggered by:
- **Scroll** — React to wallpaper page scroll position (0.0-1.0 per page)
- **Time** — Continuous animation (e.g., rotate 360° over 60 seconds for a clock hand)
- **Reactive** — React to a formula value (e.g., battery level drives progress arc)
- **On tap** — Triggered by user interaction
- **On show/hide** — Triggered by visibility changes

**Animation properties:**
- `type`: fade, rotate, scale, translate, color, blur
- `rule`: scroll position / time / formula value
- `speed`: animation duration
- `amount`: magnitude of effect
- `easing`: linear, ease-in, ease-out, ease-in-out, bounce, elastic
- `delay`: start delay
- `loop`: none, restart, reverse

---

## Part 3: Editor UI

The editor is a Svelte-based visual design tool.

### Layout

```
┌──────────────────────────────────────────────────────┐
│  Menu Bar  [File] [Edit] [View] [Insert] [Help]     │
├──────────┬───────────────────────┬───────────────────┤
│          │                       │                   │
│  Layer   │                       │   Properties      │
│  Tree    │    Canvas Preview     │   Panel           │
│          │    (WYSIWYG)          │                   │
│  - Root  │                       │   Position        │
│   ├ BG   │   ┌───────────────┐   │   Size            │
│   ├ Clock│   │               │   │   Color/Fill      │
│   ├ Date │   │  Live Preview │   │   Font            │
│   └ ...  │   │  with formulas│   │   Formula: [___]  │
│          │   │  evaluated    │   │   Animation       │
│          │   └───────────────┘   │   Touch Action     │
│          │                       │                   │
├──────────┴───────────────────────┴───────────────────┤
│  Formula Bar: $df(hh:mm)$                    [▶ Test]│
│  Status: 1920x1080 | 60fps | 12 layers | Saved      │
└──────────────────────────────────────────────────────┘
```

### Editor Features

- **Drag & drop** items on canvas
- **Resize handles** with snap-to-grid
- **Layer tree** with drag-to-reorder, visibility toggle, lock toggle
- **Property panel** with per-property formula toggle (static value ↔ `$formula$`)
- **Formula bar** with syntax highlighting, autocomplete for function names, and live preview
- **Global variables manager** — define name, type (text/number/color/switch/list), default value
- **Komponent library** — save/load reusable components
- **Undo/redo** with full history
- **Multi-select** with batch property editing
- **Zoom & pan** canvas navigation
- **Ruler & guides** for alignment
- **Color picker** with eyedropper and palette extraction
- **Font browser** with system font listing
- **Asset manager** for imported images/fonts

---

## Part 4: KLWP Import/Export

### `.klwp` File Format

A `.klwp` file is a **ZIP archive** containing:
```
preset.klwp (ZIP)
├── preset_conf.xml    ← Main configuration (layer tree, formulas, globals)
├── komponent_XXX.xml  ← Embedded komponent definitions
├── fonts/             ← Custom font files (.ttf, .otf)
├── bitmaps/           ← Image assets
│   ├── image1.png
│   └── image2.jpg
└── icons/             ← Icon font files
```

### Import Pipeline

```
.klwp file
  │
  ├─ 1. Unzip archive
  │
  ├─ 2. Parse preset_conf.xml
  │     ├─ Extract layer tree structure
  │     ├─ Extract all formulas
  │     ├─ Extract global variable definitions
  │     ├─ Extract animation configs
  │     └─ Extract touch actions
  │
  ├─ 3. Transform Android-specific functions to Linux equivalents
  │     ├─ bi(level) → read /sys/class/power_supply/
  │     ├─ mi(title) → MPRIS D-Bus
  │     ├─ si(model) → hostname
  │     ├─ si(aver)  → kernel version
  │     ├─ nc(*)     → NetworkManager D-Bus
  │     ├─ ni(*)     → Desktop notification D-Bus
  │     └─ [unsupported] → show warning, use placeholder
  │
  ├─ 4. Copy assets (fonts, images) to project directory
  │
  ├─ 5. Adapt dimensions
  │     ├─ KLWP uses phone resolution (e.g., 1080x2400)
  │     ├─ Scale to desktop resolution (e.g., 1920x1080)
  │     └─ Adjust scroll pages (KLWP = multi-page, desktop = single/multi-monitor)
  │
  └─ 6. Generate .klx project file (KustomLinux native format)
```

### Compatibility Matrix

| KLWP Feature | Linux Support | Notes |
|-------------|---------------|-------|
| Text + formulas | Full | All text functions work |
| Shapes & images | Full | Canvas rendering |
| Date/time formulas | Full | chrono crate |
| Weather formulas | Full | OpenWeatherMap API |
| Battery info | Full | UPower / sysfs |
| Music info | Full | MPRIS D-Bus |
| Animations (scroll) | Partial | Map to multi-monitor or parallax |
| Animations (time) | Full | CSS/JS animations |
| Touch actions | Partial | Launch app, open URL (no Android intents) |
| Komponents | Full | Mapped to reusable components |
| Fitness data (FD) | None | No equivalent — show N/A |
| Complication data (CD) | None | Smartwatch only — ignore |
| Broadcast receiver (BR) | Partial | Map to D-Bus signals |
| Unread counters (UC) | Partial | Plugin-based |

### Export Format

Export a KustomLinux project as a runnable wallpaper:
```
wallpaper_name/
├── wallpaper.html       ← Self-contained HTML/CSS/JS renderer
├── wallpaper.css
├── wallpaper.js          ← Compiled formula engine + render loop
├── config.json           ← Evaluated globals and settings
├── assets/
│   ├── fonts/
│   └── images/
├── data-bridge.sh        ← Shell script that outputs system data as JSON
└── run.sh                ← Launcher script (calls xwinwrap/mpvpaper/etc.)
```

**OR** as a native Tauri background process:
```
kustomlinux-wallpaper     ← Single binary (Tauri app in wallpaper mode)
```

---

## Part 5: System Wallpaper Integration

### X11 (Xorg)

```bash
# Option 1: xwinwrap + chromium/webview
xwinwrap -fs -fdt -ni -b -nf -un -o 1.0 -- \
  /path/to/kustomlinux-renderer --wallpaper project.klx

# Option 2: Render to image, update periodically
kustomlinux --render project.klx --output /tmp/wallpaper.png
feh --bg-fill /tmp/wallpaper.png
# Run in loop for live updates
```

### Wayland (Hyprland/Sway)

```bash
# Option 1: mpvpaper with rendered video/frames
mpvpaper '*' --fork /tmp/kustomlinux-output.mp4

# Option 2: swww with frame sequence
swww img /tmp/wallpaper.png --transition-type none
# Update in loop for animation

# Option 3: Custom Wayland layer-shell surface (ideal)
# Use wlr-layer-shell protocol to create a background surface
# Render directly via GPU
```

### Preferred: Wayland Layer Shell

For Hyprland/Sway/other wlroots compositors, the ideal approach is:
1. Create a Wayland surface using `wlr-layer-shell-unstable-v1` protocol
2. Set `layer = BACKGROUND`, `anchor = all edges`
3. Render the wallpaper directly on this surface using GPU (via `wgpu` or `smithay-client-toolkit`)
4. This gives true live wallpaper with no hacks

The Tauri app should detect the display server and choose the appropriate method.

---

## Part 6: Data Provider System

Each data provider runs in the Rust backend and pushes updates to the frontend.

### Provider Architecture

```rust
trait DataProvider: Send + Sync {
    /// Unique prefix (e.g., "bi", "wi", "mi")
    fn prefix(&self) -> &str;

    /// Available fields
    fn fields(&self) -> Vec<&str>;

    /// Get current value for a field
    fn get(&self, field: &str) -> ProviderValue;

    /// Update interval (how often to poll)
    fn interval(&self) -> Duration;

    /// Start the provider (subscribe to D-Bus signals, etc.)
    fn start(&mut self) -> Result<()>;

    /// Stop the provider
    fn stop(&mut self);
}
```

### Provider Update Strategy

| Provider | Method | Interval |
|----------|--------|----------|
| Date/Time (DF) | Timer | 1 second |
| Battery (BI) | UPower D-Bus signal | Event-driven |
| Music (MI) | MPRIS D-Bus signal | Event-driven |
| Weather (WI/WF) | HTTP poll | 15 minutes |
| System (SI) | Mixed | 5 seconds |
| Resource Monitor (RM) | /proc poll | 2 seconds |
| Network (NC) | NetworkManager signal | Event-driven |
| Location (LI) | GeoClue signal | Event-driven |
| Notifications (NI) | D-Bus signal | Event-driven |
| Calendar (CI) | File watch / poll | 5 minutes |
| Astronomy (AI) | Calculated | 1 minute |
| Traffic (TS) | /proc/net/dev | 2 seconds |

---

## Part 7: Project File Format (`.klx`)

The native KustomLinux project format is a JSON file:

```jsonc
{
  "version": "1.0.0",
  "name": "My Wallpaper",
  "resolution": { "width": 1920, "height": 1080 },
  "background": { "type": "color", "value": "#1a1a2e" },
  "globals": [
    { "name": "accent_color", "type": "color", "value": "#e94560" },
    { "name": "show_seconds", "type": "switch", "value": true },
    { "name": "city", "type": "text", "value": "Tokyo" },
    { "name": "temp_unit", "type": "list", "options": ["C", "F"], "value": "C" }
  ],
  "layers": [
    {
      "id": "clock_text",
      "type": "text",
      "properties": {
        "x": 960, "y": 400,
        "text": "$df(hh:mm)$",
        "fontSize": 120,
        "fontFamily": "JetBrains Mono",
        "color": "$gv(accent_color)$",
        "anchor": "center",
        "opacity": 255,
        "shadow": { "color": "#00000080", "dx": 2, "dy": 2, "radius": 4 }
      },
      "animations": [
        {
          "type": "fade",
          "trigger": "time",
          "rule": "$mu(sin, df(s) * 6)$",
          "amount": 50
        }
      ]
    },
    {
      "id": "battery_arc",
      "type": "progress",
      "properties": {
        "x": 100, "y": 100,
        "width": 80, "height": 80,
        "style": "arc",
        "min": 0, "max": 100,
        "value": "$bi(level)$",
        "color": "$if(bi(level) < 20, #ff0000, gv(accent_color))$",
        "trackColor": "#ffffff20",
        "strokeWidth": 6
      }
    }
  ]
}
```

---

## Part 8: Implementation Phases

### Phase 1 — Foundation
- [ ] Set up Tauri + Svelte project structure
- [ ] Implement formula parser & evaluator in Rust (compile to WASM for frontend too)
- [ ] Implement core functions: `df`, `if`, `mu`, `tc`, `fl`, `gv`, `lv`, `ce`, `cm`
- [ ] Basic canvas renderer: Text, Shape, Image items
- [ ] Basic editor: layer tree, property panel, canvas with drag/drop
- [ ] Project save/load (`.klx` JSON format)

### Phase 2 — Data Providers
- [ ] Battery provider (UPower / sysfs)
- [ ] Date/time provider (built-in)
- [ ] Music provider (MPRIS D-Bus)
- [ ] System info provider
- [ ] Resource monitor provider (/proc)
- [ ] Network provider (NetworkManager)
- [ ] Weather provider (OpenWeatherMap API)
- [ ] Astronomy provider (calculated from location + date)
- [ ] Location provider (GeoClue / IP geolocation)

### Phase 3 — Advanced Rendering
- [ ] Animations (time-based, reactive, scroll-based)
- [ ] Progress items (arc, bar, circle)
- [ ] Stack/Overlap/Scroll groups
- [ ] FontIcon support (Material Icons, FontAwesome, Weather Icons)
- [ ] Image filters (blur, tint, saturation)
- [ ] SVG path shapes
- [ ] Komponent system (reusable components with inputs)

### Phase 4 — Groups & Containers
- [ ] Layer hierarchy — layers can have `children` and `groupType` (overlap/stack)
- [ ] Overlap Group — children rendered at same position, offset via anchor/padding
- [ ] Stack Group — children arranged sequentially (horizontal or vertical), auto-positioned
- [ ] Scroll Group — scrollable container with direction and page count
- [ ] Group transforms — position, scale, rotation applied recursively to all children
- [ ] LayerTree UI — tree with expand/collapse, drag to reorder/reparent
- [ ] Renderer — recursive rendering respecting group transforms
- [ ] Komponent support — reusable overlap groups with encapsulated variables

### Phase 5 — KLWP Import
- [ ] ZIP extraction and XML parsing
- [ ] Layer tree reconstruction from preset_conf.xml
- [ ] Formula migration (Android → Linux function mapping)
- [ ] Asset extraction (fonts, images)
- [ ] Resolution adaptation (portrait → landscape)
- [ ] Unsupported feature warnings & graceful fallbacks
- [ ] Komponent import

### Phase 6 — System Integration
- [ ] Detect display server (X11 vs Wayland) and compositor
- [ ] X11: xwinwrap-based wallpaper mode
- [ ] Wayland: wlr-layer-shell wallpaper surface
- [ ] Wallpaper autostart (systemd user service / XDG autostart)
- [ ] Multi-monitor support (per-monitor or spanned)
- [ ] Export as standalone wallpaper package

### Phase 7 — Polish
- [ ] Formula autocomplete & syntax highlighting
- [ ] Undo/redo system
- [ ] Komponent library (save/load/share)
- [ ] Theming for the editor itself
- [ ] Performance profiling & optimization (target 60fps at <5% CPU idle)
- [ ] Notification provider (D-Bus)
- [ ] Calendar provider
- [ ] Shell command function (`sh()`)
- [ ] Web get function (`wg()`)
- [ ] Air quality provider
- [ ] Traffic stats provider
- [ ] Linux-specific extensions: `wm()`, `ws()`, `th()`, `pm()`

---

## Non-Functional Requirements

- **Performance**: Idle wallpaper should use <5% CPU, <100MB RAM. Animations at 60fps.
- **Startup**: Wallpaper should be visible within 2 seconds of boot.
- **Battery**: Reduce update frequency on battery power (laptops).
- **Crash recovery**: Auto-save editor state. Wallpaper process should auto-restart on crash.
- **Accessibility**: Editor UI should support keyboard navigation.
- **File size**: Exported wallpaper packages should be <50MB unless heavy on images.

---

## Key Design Decisions

1. **Formula engine in Rust compiled to WASM** — Runs in both backend (for providers) and frontend (for live preview) without duplication.
2. **Canvas rendering, not DOM** — Better performance for complex wallpapers with many layers and animations.
3. **Tauri, not Electron** — Smaller binary, lower memory, native Rust backend.
4. **JSON project files, not XML** — Simpler to work with than KLWP's XML format; import converts XML→JSON.
5. **Layer-shell for Wayland** — True live wallpaper integration, not screenshot-and-set hacks.
6. **Event-driven data providers** — D-Bus signals for real-time updates without polling where possible.

---

## Reference Documentation

- KLWP Functions: https://docs.kustom.rocks/docs/reference/functions/
- KLWP Animations: https://docs.kustom.rocks/docs/reference/animations/
- Tauri v1 Docs: https://tauri.app/v1/guides/
- wlr-layer-shell protocol: https://wayland.app/protocols/wlr-layer-shell-unstable-v1
- MPRIS D-Bus spec: https://specifications.freedesktop.org/mpris-spec/latest/
- UPower D-Bus: https://upower.freedesktop.org/docs/
- GeoClue D-Bus: https://www.freedesktop.org/wiki/Software/GeoClue/
