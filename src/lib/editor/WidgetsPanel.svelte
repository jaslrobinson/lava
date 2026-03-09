<script lang="ts">
  import { insertWidget, ensureGlobal } from "../stores/project.svelte";
  import { generateId, type Layer, type GlobalVarType } from "../types/project";

  type GlobalDef = { name: string; type: GlobalVarType; value: string | number | boolean };
  type WidgetDef = { name: string; description: string; build: () => Layer; globals?: GlobalDef[] };
  type Category = { name: string; widgets: WidgetDef[] };

  function id() { return generateId(); }

  function mkText(name: string, props: Record<string, any>): Layer {
    return { id: id(), name, type: "text", properties: { x: 0, y: 0, width: 200, height: 30, opacity: 255, anchor: "top-left", fontSize: 24, fontFamily: "sans-serif", color: "#ffffff", textAlign: "left", ...props }, visible: true, locked: false };
  }
  function mkShape(name: string, props: Record<string, any>): Layer {
    return { id: id(), name, type: "shape", properties: { x: 0, y: 0, width: 200, height: 200, opacity: 255, anchor: "top-left", shapeKind: "rectangle", fill: "#e94560", ...props }, visible: true, locked: false };
  }
  function mkImage(name: string, props: Record<string, any>): Layer {
    return { id: id(), name, type: "image", properties: { x: 0, y: 0, width: 200, height: 200, opacity: 255, anchor: "top-left", src: "", scaleMode: "fit", ...props }, visible: true, locked: false };
  }
  function mkProgress(name: string, props: Record<string, any>): Layer {
    return { id: id(), name, type: "progress", properties: { x: 0, y: 0, width: 80, height: 80, opacity: 255, anchor: "top-left", style: "arc", min: 0, max: 100, value: 50, color: "#e94560", trackColor: "#ffffff20", strokeWidth: 6, ...props }, visible: true, locked: false };
  }
  function mkOverlap(name: string, props: Record<string, any>, children: Layer[]): Layer {
    return { id: id(), name, type: "overlap", properties: { x: 0, y: 0, width: 400, height: 200, opacity: 255, anchor: "top-left", ...props }, children, visible: true, locked: false };
  }
  function mkStack(name: string, props: Record<string, any>, children: Layer[]): Layer {
    return { id: id(), name, type: "stack", properties: { x: 0, y: 0, width: 400, height: 200, opacity: 255, anchor: "top-left", orientation: "vertical", spacing: 0, ...props }, children, visible: true, locked: false };
  }

  const categories: Category[] = [
    // ── CLOCKS ──
    {
      name: "Clocks",
      widgets: [
        {
          name: "Minimal Clock",
          description: "Clean time and date",
          build: () => mkStack("Minimal Clock", { x: 100, y: 100, width: 300, height: 120, spacing: 4 }, [
            mkText("Time", { text: "$df(hh:mm)$", fontSize: 64, fontFamily: "sans-serif", color: "#ffffff", textAlign: "center", width: 300, height: 70 }),
            mkText("Date", { text: "$df(EEEE, MMMM d)$", fontSize: 18, color: "#aaaaaa", textAlign: "center", width: 300, height: 24 }),
          ]),
        },
        {
          name: "Bold Clock",
          description: "Large bold time with thin date",
          build: () => mkStack("Bold Clock", { x: 100, y: 100, width: 400, height: 160, spacing: 0 }, [
            mkText("Time", { text: "$df(HH:mm)$", fontSize: 96, fontFamily: "sans-serif", color: "#ffffff", textAlign: "left", width: 400, height: 110 }),
            mkText("Date", { text: "$df(EEE d MMM yyyy)$", fontSize: 16, color: "#888888", textAlign: "left", width: 400, height: 22 }),
          ]),
        },
        {
          name: "Split Clock",
          description: "Hour and minute separated with accent",
          build: () => mkStack("Split Clock", { x: 100, y: 100, width: 220, height: 180, spacing: 0 }, [
            mkText("Hour", { text: "$df(HH)$", fontSize: 100, color: "#e94560", textAlign: "center", width: 220, height: 110 }),
            mkText("Minute", { text: "$df(mm)$", fontSize: 100, color: "#ffffff", textAlign: "center", width: 220, height: 110 }),
          ]),
        },
        {
          name: "Clock + Seconds Arc",
          description: "Time with circular seconds progress",
          build: () => mkOverlap("Clock + Seconds", { x: 100, y: 100, width: 200, height: 200 }, [
            mkProgress("Seconds Ring", { style: "arc", min: 0, max: 60, value: "$dp(s)$", color: "#e94560", trackColor: "#ffffff10", strokeWidth: 4, width: 200, height: 200, x: 0, y: 0 }),
            mkText("Time", { text: "$df(HH:mm)$", fontSize: 42, color: "#ffffff", textAlign: "center", width: 200, height: 50, x: 0, y: 75, anchor: "top-left" }),
          ]),
        },
        {
          name: "Word Clock",
          description: "Time spelled out in words",
          build: () => mkStack("Word Clock", { x: 100, y: 100, width: 400, height: 100, spacing: 2 }, [
            mkText("Time Words", { text: "$df(hh:mm a)$", fontSize: 48, color: "#ffffff", textAlign: "center", width: 400, height: 56 }),
            mkText("Day Info", { text: "$df(EEEE)$ . $df(MMMM d, yyyy)$", fontSize: 14, color: "#666666", textAlign: "center", width: 400, height: 20 }),
          ]),
        },
      ],
    },

    // ── WEATHER ──
    {
      name: "Weather",
      widgets: [
        {
          name: "Current Weather",
          description: "Temperature, condition, and icon",
          build: () => mkOverlap("Current Weather", { x: 100, y: 100, width: 300, height: 120 }, [
            mkShape("BG", { shapeKind: "rectangle", fill: "#00000066", cornerRadius: 12, width: 300, height: 120, x: 0, y: 0 }),
            mkImage("Icon", { src: "$wi(iconurl)$", width: 80, height: 80, x: 15, y: 20 }),
            mkText("Temp", { text: "$wi(temp)$\u00B0", fontSize: 48, color: "#ffffff", x: 110, y: 20, width: 100, height: 56 }),
            mkText("Condition", { text: "$wi(cond)$", fontSize: 14, color: "#aaaaaa", x: 110, y: 78, width: 180, height: 20 }),
            mkText("HL", { text: "H:$wf(0, max)$\u00B0 L:$wf(0, min)$\u00B0", fontSize: 12, color: "#888888", x: 110, y: 96, width: 180, height: 18 }),
          ]),
        },
        {
          name: "Weather Compact",
          description: "Single line weather",
          build: () => mkText("Weather Compact", { x: 100, y: 100, width: 400, height: 30, text: "$wi(temp)$\u00B0 $wi(cond)$ | H:$wf(0, max)$\u00B0 L:$wf(0, min)$\u00B0", fontSize: 16, color: "#cccccc" }),
        },
        {
          name: "5-Day Forecast",
          description: "Forecast cards for 5 days",
          build: () => {
            const days: Layer[] = [];
            for (let i = 0; i < 5; i++) {
              days.push(mkStack(`Day ${i}`, { width: 70, height: 100, spacing: 2 }, [
                mkText(`Day`, { text: `$wf(${i}, day)$`, fontSize: 11, color: "#aaaaaa", textAlign: "center", width: 70, height: 16 }),
                mkImage(`Icon`, { src: `$wf(${i}, iconurl)$`, width: 40, height: 40, x: 15, y: 0 }),
                mkText(`Temp`, { text: `$wf(${i}, max)$\u00B0/$wf(${i}, min)$\u00B0`, fontSize: 11, color: "#ffffff", textAlign: "center", width: 70, height: 16 }),
              ]));
            }
            return mkOverlap("5-Day Forecast", { x: 100, y: 100, width: 380, height: 130 }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#00000066", cornerRadius: 12, width: 380, height: 130, x: 0, y: 0 }),
              mkStack("Days", { width: 370, height: 110, x: 5, y: 10, orientation: "horizontal", spacing: 4 }, days),
            ]);
          },
        },
      ],
    },

    // ── MUSIC ──
    {
      name: "Music",
      widgets: [
        {
          name: "Music Player",
          description: "Album art, title, artist, progress bar",
          build: () => mkOverlap("Music Player", { x: 100, y: 400, width: 360, height: 120 }, [
            mkShape("BG", { shapeKind: "rectangle", fill: "#1a1a2ecc", cornerRadius: 12, width: 360, height: 120, x: 0, y: 0 }),
            mkImage("Cover", { src: "$mi(cover)$", width: 80, height: 80, x: 16, y: 16, cornerRadius: 8 }),
            mkText("Title", { text: "$mi(title)$", fontSize: 16, color: "#ffffff", x: 110, y: 18, width: 230, height: 22 }),
            mkText("Artist", { text: "$mi(artist)$", fontSize: 13, color: "#888888", x: 110, y: 42, width: 230, height: 18 }),
            mkProgress("Progress", { style: "bar", min: 0, max: 100, value: "$mi(percent)$", color: "#e94560", trackColor: "#ffffff15", strokeWidth: 3, width: 230, height: 6, x: 110, y: 70 }),
            mkText("Time", { text: "$mi(pos)$s / $mi(len)$s", fontSize: 10, color: "#666666", x: 110, y: 82, width: 230, height: 14 }),
          ]),
        },
        {
          name: "Music Minimal",
          description: "Just title and artist",
          build: () => mkStack("Music Minimal", { x: 100, y: 400, width: 300, height: 50, spacing: 2 }, [
            mkText("Title", { text: "$mi(title)$", fontSize: 18, color: "#ffffff", width: 300, height: 24 }),
            mkText("Artist", { text: "$mi(artist)$ \u2022 $mi(album)$", fontSize: 13, color: "#888888", width: 300, height: 18 }),
          ]),
        },
        {
          name: "Music Full",
          description: "Large album art with info overlay",
          build: () => mkOverlap("Music Full", { x: 100, y: 300, width: 320, height: 320 }, [
            mkImage("Cover", { src: "$mi(cover)$", width: 320, height: 320, scaleMode: "fill", x: 0, y: 0 }),
            mkShape("Gradient", { shapeKind: "rectangle", fill: "#000000aa", width: 320, height: 120, x: 0, y: 200 }),
            mkText("Title", { text: "$mi(title)$", fontSize: 20, color: "#ffffff", x: 16, y: 220, width: 288, height: 26 }),
            mkText("Artist", { text: "$mi(artist)$", fontSize: 14, color: "#cccccc", x: 16, y: 250, width: 288, height: 20 }),
            mkProgress("Progress", { style: "bar", min: 0, max: 100, value: "$mi(percent)$", color: "#e94560", trackColor: "#ffffff20", strokeWidth: 4, width: 288, height: 6, x: 16, y: 280 }),
          ]),
        },
      ],
    },

    // ── NEWS / RSS ──
    {
      name: "News",
      widgets: [
        {
          name: "RSS Headlines",
          description: "Cycling news from RSS feed",
          build: () => {
            const w = mkOverlap("RSS Headlines", { x: 100, y: 600, width: 400, height: 80 }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#00000055", cornerRadius: 8, width: 400, height: 80, x: 0, y: 0 }),
              mkText("Headline", { text: '$wg("https://news.google.com/rss", rss, feed, mu(mod, mu(floor, dp(s), 30), 10))$', fontSize: 15, color: "#ffffff", x: 12, y: 12, width: 376, height: 22 }),
              mkText("Description", { text: '$wg("https://news.google.com/rss", rss, desc, mu(mod, mu(floor, dp(s), 30), 10))$', fontSize: 12, color: "#999999", x: 12, y: 38, width: 376, height: 34 }),
            ]);
            return w;
          },
        },
        {
          name: "RSS Ticker",
          description: "Single line scrolling headline",
          build: () => mkText("RSS Ticker", { x: 100, y: 600, width: 600, height: 24, text: '$wg("https://news.google.com/rss", rss, feed, mu(mod, mu(floor, dp(s), 15), 10))$', fontSize: 14, color: "#aaaaaa" }),
        },
      ],
    },

    // ── SYSTEM ──
    {
      name: "System",
      widgets: [
        {
          name: "Battery + RAM + CPU",
          description: "System stats with progress arcs",
          build: () => mkStack("System Stats", { x: 100, y: 300, width: 300, height: 100, orientation: "horizontal", spacing: 20 }, [
            mkOverlap("Battery", { width: 80, height: 100 }, [
              mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$bi(level)$", color: "#4ecca3", trackColor: "#ffffff15", strokeWidth: 5, width: 80, height: 80, x: 0, y: 0 }),
              mkText("Label", { text: "$bi(level)$%", fontSize: 14, color: "#ffffff", textAlign: "center", width: 80, height: 18, x: 0, y: 32 }),
              mkText("Sub", { text: "BAT", fontSize: 9, color: "#666666", textAlign: "center", width: 80, height: 12, x: 0, y: 85 }),
            ]),
            mkOverlap("RAM", { width: 80, height: 100 }, [
              mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$rm(ramp)$", color: "#e94560", trackColor: "#ffffff15", strokeWidth: 5, width: 80, height: 80, x: 0, y: 0 }),
              mkText("Label", { text: "$rm(ramp)$%", fontSize: 14, color: "#ffffff", textAlign: "center", width: 80, height: 18, x: 0, y: 32 }),
              mkText("Sub", { text: "RAM", fontSize: 9, color: "#666666", textAlign: "center", width: 80, height: 12, x: 0, y: 85 }),
            ]),
            mkOverlap("CPU", { width: 80, height: 100 }, [
              mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$si(cpupercent)$", color: "#0088ff", trackColor: "#ffffff15", strokeWidth: 5, width: 80, height: 80, x: 0, y: 0 }),
              mkText("Label", { text: "$si(cpupercent)$%", fontSize: 14, color: "#ffffff", textAlign: "center", width: 80, height: 18, x: 0, y: 32 }),
              mkText("Sub", { text: "CPU", fontSize: 9, color: "#666666", textAlign: "center", width: 80, height: 12, x: 0, y: 85 }),
            ]),
          ]),
        },
        {
          name: "Network Info",
          description: "IP address and connection status",
          build: () => mkStack("Network Info", { x: 100, y: 500, width: 300, height: 50, spacing: 2 }, [
            mkText("SSID", { text: "$ni(ssid)$", fontSize: 16, color: "#ffffff", width: 300, height: 22 }),
            mkText("IP", { text: "$ni(ip)$", fontSize: 12, color: "#888888", width: 300, height: 16 }),
          ]),
        },
      ],
    },

    // ── FULL THEMES ──
    {
      name: "Themes",
      widgets: [
        {
          name: "Desktop Dashboard",
          description: "Clock + weather + music + system stats",
          build: () => mkOverlap("Dashboard", { x: 0, y: 0, width: 1920, height: 1080, anchor: "top-left" }, [
            // Clock - top left
            mkStack("Clock", { x: 60, y: 60, width: 400, height: 140, spacing: 4 }, [
              mkText("Time", { text: "$df(HH:mm)$", fontSize: 80, color: "#ffffff", width: 400, height: 90 }),
              mkText("Date", { text: "$df(EEEE, MMMM d, yyyy)$", fontSize: 18, color: "#888888", width: 400, height: 24 }),
            ]),
            // Weather - top right
            mkOverlap("Weather", { x: 1560, y: 60, width: 300, height: 120 }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#00000044", cornerRadius: 12, width: 300, height: 120, x: 0, y: 0 }),
              mkImage("Icon", { src: "$wi(iconurl)$", width: 70, height: 70, x: 15, y: 20 }),
              mkText("Temp", { text: "$wi(temp)$\u00B0", fontSize: 42, color: "#ffffff", x: 100, y: 18, width: 100, height: 50 }),
              mkText("Cond", { text: "$wi(cond)$", fontSize: 13, color: "#aaaaaa", x: 100, y: 68, width: 180, height: 18 }),
              mkText("HL", { text: "H:$wf(0, max)$\u00B0 L:$wf(0, min)$\u00B0", fontSize: 11, color: "#666666", x: 100, y: 88, width: 180, height: 16 }),
            ]),
            // Music - bottom left
            mkOverlap("Music", { x: 60, y: 900, width: 360, height: 110 }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#00000044", cornerRadius: 12, width: 360, height: 110, x: 0, y: 0 }),
              mkImage("Cover", { src: "$mi(cover)$", width: 70, height: 70, x: 16, y: 16 }),
              mkText("Title", { text: "$mi(title)$", fontSize: 15, color: "#ffffff", x: 100, y: 18, width: 244, height: 20 }),
              mkText("Artist", { text: "$mi(artist)$", fontSize: 12, color: "#888888", x: 100, y: 40, width: 244, height: 16 }),
              mkProgress("Bar", { style: "bar", min: 0, max: 100, value: "$mi(percent)$", color: "#e94560", trackColor: "#ffffff15", strokeWidth: 3, width: 244, height: 4, x: 100, y: 66 }),
            ]),
            // System stats - bottom right
            mkStack("System", { x: 1600, y: 920, width: 260, height: 90, orientation: "horizontal", spacing: 16 }, [
              mkOverlap("BAT", { width: 70, height: 90 }, [
                mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$bi(level)$", color: "#4ecca3", trackColor: "#ffffff10", strokeWidth: 4, width: 70, height: 70 }),
                mkText("Val", { text: "$bi(level)$%", fontSize: 12, color: "#ffffff", textAlign: "center", width: 70, height: 16, x: 0, y: 28 }),
                mkText("Lbl", { text: "BAT", fontSize: 8, color: "#555555", textAlign: "center", width: 70, height: 10, x: 0, y: 76 }),
              ]),
              mkOverlap("RAM", { width: 70, height: 90 }, [
                mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$rm(ramp)$", color: "#e94560", trackColor: "#ffffff10", strokeWidth: 4, width: 70, height: 70 }),
                mkText("Val", { text: "$rm(ramp)$%", fontSize: 12, color: "#ffffff", textAlign: "center", width: 70, height: 16, x: 0, y: 28 }),
                mkText("Lbl", { text: "RAM", fontSize: 8, color: "#555555", textAlign: "center", width: 70, height: 10, x: 0, y: 76 }),
              ]),
              mkOverlap("CPU", { width: 70, height: 90 }, [
                mkProgress("Arc", { style: "arc", min: 0, max: 100, value: "$si(cpupercent)$", color: "#0088ff", trackColor: "#ffffff10", strokeWidth: 4, width: 70, height: 70 }),
                mkText("Val", { text: "$si(cpupercent)$%", fontSize: 12, color: "#ffffff", textAlign: "center", width: 70, height: 16, x: 0, y: 28 }),
                mkText("Lbl", { text: "CPU", fontSize: 8, color: "#555555", textAlign: "center", width: 70, height: 10, x: 0, y: 76 }),
              ]),
            ]),
          ]),
        },
        {
          name: "Centered Focus",
          description: "Center-screen clock, weather bar, 5-day forecast, news ticker",
          build: () => mkOverlap("Centered Focus", { x: 0, y: 0, width: 1920, height: 1080, anchor: "top-left" }, [
            // Big centered clock
            mkOverlap("Clock Block", { x: 660, y: 280, width: 600, height: 220 }, [
              mkText("Time", { text: "$df(HH)$:$df(mm)$", fontSize: 140, color: "#ffffff", textAlign: "center", width: 600, height: 160, x: 0, y: 0 }),
              mkShape("Divider", { shapeKind: "rectangle", fill: "#e94560", width: 60, height: 3, x: 270, y: 165 }),
              mkText("Date", { text: "$df(EEEE)$  |  $df(MMMM d, yyyy)$", fontSize: 16, color: "#666666", textAlign: "center", width: 600, height: 22, x: 0, y: 180 }),
            ]),
            // Weather bar below clock
            mkOverlap("Weather Bar", { x: 610, y: 530, width: 700, height: 70 }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#ffffff08", cornerRadius: 35, width: 700, height: 70, x: 0, y: 0 }),
              mkImage("Icon", { src: "$wi(iconurl)$", width: 50, height: 50, x: 20, y: 10 }),
              mkText("Temp", { text: "$wi(temp)$\u00B0", fontSize: 32, color: "#ffffff", x: 80, y: 16, width: 80, height: 40 }),
              mkText("Cond", { text: "$wi(cond)$", fontSize: 13, color: "#999999", x: 170, y: 26, width: 140, height: 18 }),
              mkShape("Sep1", { shapeKind: "rectangle", fill: "#ffffff15", width: 1, height: 40, x: 320, y: 15 }),
              mkText("Wind", { text: "$wi(wspeed)$ $wi(wdir)$", fontSize: 12, color: "#888888", x: 340, y: 18, width: 100, height: 16 }),
              mkText("Hum", { text: "Humidity $wi(hum)$%", fontSize: 12, color: "#888888", x: 340, y: 38, width: 120, height: 16 }),
              mkShape("Sep2", { shapeKind: "rectangle", fill: "#ffffff15", width: 1, height: 40, x: 480, y: 15 }),
              mkText("HL", { text: "H $wf(0, max)$\u00B0  L $wf(0, min)$\u00B0", fontSize: 14, color: "#aaaaaa", x: 500, y: 24, width: 180, height: 20 }),
            ]),
            // 5-day forecast row
            ...(() => {
              const days: Layer[] = [];
              for (let i = 0; i < 5; i++) {
                days.push(mkOverlap(`Day${i}`, { width: 100, height: 90 }, [
                  mkText("Day", { text: `$wf(${i}, day)$`, fontSize: 11, color: "#555555", textAlign: "center", width: 100, height: 14, x: 0, y: 0 }),
                  mkImage("Ic", { src: `$wf(${i}, iconurl)$`, width: 36, height: 36, x: 32, y: 16 }),
                  mkText("Temps", { text: `$wf(${i}, max)$\u00B0 / $wf(${i}, min)$\u00B0`, fontSize: 11, color: "#aaaaaa", textAlign: "center", width: 100, height: 14, x: 0, y: 58 }),
                  mkText("Cond", { text: `$wf(${i}, cond)$`, fontSize: 9, color: "#555555", textAlign: "center", width: 100, height: 12, x: 0, y: 74 }),
                ]));
              }
              return [mkStack("Forecast Row", { x: 660, y: 620, width: 600, height: 90, orientation: "horizontal", spacing: 20 }, days)];
            })(),
            // Music - bottom center
            mkOverlap("Music", { x: 710, y: 920, width: 500, height: 80, visible: "$if(mi(state)=PLAYING, ALWAYS, NEVER)$" }, [
              mkShape("BG", { shapeKind: "rectangle", fill: "#ffffff06", cornerRadius: 40, width: 500, height: 80, x: 0, y: 0 }),
              mkImage("Cover", { src: "$mi(cover)$", width: 56, height: 56, x: 14, y: 12, cornerRadius: 28 }),
              mkStack("Info", { x: 82, y: 16, width: 320, height: 48, spacing: 2 }, [
                mkText("Title", { text: "$mi(title)$", fontSize: 14, color: "#ffffff", width: 320, height: 20 }),
                mkText("Artist", { text: "$mi(artist)$", fontSize: 11, color: "#777777", width: 320, height: 16 }),
              ]),
              mkProgress("Bar", { style: "bar", min: 0, max: 100, value: "$mi(percent)$", color: "#e94560", trackColor: "#ffffff10", strokeWidth: 2, width: 400, height: 3, x: 50, y: 72 }),
            ]),
            // News ticker at very bottom
            mkText("News", { text: '$wg("https://news.google.com/rss", rss, feed, mu(mod, mu(floor, dp(s), 20), 10))$', fontSize: 12, color: "#444444", textAlign: "center", x: 460, y: 1050, width: 1000, height: 18 }),
            // System bar - top right corner
            mkStack("System Bar", { x: 1700, y: 20, width: 180, height: 14, orientation: "horizontal", spacing: 12 }, [
              mkText("CPU", { text: "CPU $si(cpupercent)$%", fontSize: 10, color: "#555555", width: 60, height: 14 }),
              mkText("RAM", { text: "RAM $rm(ramp)$%", fontSize: 10, color: "#555555", width: 60, height: 14 }),
              mkText("BAT", { text: "$bi(level)$%", fontSize: 10, color: "#555555", width: 40, height: 14 }),
            ]),
          ]),
        },
      ],
    },
  ];

  let selectedCategory = $state(categories[0].name);

  function handleAddWidget(widget: WidgetDef) {
    if (widget.globals) {
      for (const g of widget.globals) {
        ensureGlobal(g.name, g.type, g.value);
      }
    }
    insertWidget(widget.build());
  }
</script>

<div class="widgets-panel">
  <div class="category-tabs">
    {#each categories as cat}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span
        class="cat-tab"
        class:active={selectedCategory === cat.name}
        onclick={() => { selectedCategory = cat.name; }}
      >{cat.name}</span>
    {/each}
  </div>

  <div class="widget-list">
    {#each categories.find(c => c.name === selectedCategory)?.widgets ?? [] as widget}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="widget-card" onclick={() => handleAddWidget(widget)}>
        <div class="widget-name">{widget.name}</div>
        <div class="widget-desc">{widget.description}</div>
      </div>
    {/each}
  </div>
</div>

<style>
  .widgets-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .category-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0;
    border-bottom: 1px solid var(--border);
    padding: 4px 6px 0;
    flex-shrink: 0;
  }
  .cat-tab {
    padding: 4px 8px 3px;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    cursor: pointer;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    user-select: none;
  }
  .cat-tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .widget-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .widget-card {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .widget-card:hover {
    border-color: var(--accent);
  }
  .widget-name {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
  }
  .widget-desc {
    font-size: 10px;
    color: var(--text-muted);
    margin-top: 2px;
  }
</style>
