<script lang="ts">
  interface FormulaOption {
    name: string;
    formula: string;
  }

  interface FormulaCategory {
    name: string;
    options: FormulaOption[];
  }

  let { open, onInsert, onClose }: {
    open: boolean;
    onInsert: (formula: string) => void;
    onClose: () => void;
  } = $props();

  let selectedCategory = $state(0);
  let currentFormula = $state("");

  const categories: FormulaCategory[] = [
    {
      name: "Simple",
      options: [
        { name: "Custom Text", formula: "" },
      ],
    },
    {
      name: "Time",
      options: [
        { name: "Hours (24h)", formula: "$df(HH)$" },
        { name: "Hours (12h)", formula: "$df(hh)$" },
        { name: "Minutes", formula: "$df(mm)$" },
        { name: "Seconds", formula: "$df(ss)$" },
        { name: "AM/PM", formula: "$df(a)$" },
        { name: "Hours:Minutes (24h)", formula: "$df(HH:mm)$" },
        { name: "Hours:Minutes (12h)", formula: "$df(hh:mm)$" },
        { name: "Hours:Minutes:Seconds", formula: "$df(HH:mm:ss)$" },
        { name: "Clock 12h", formula: "$df(hh:mm a)$" },
      ],
    },
    {
      name: "Date",
      options: [
        { name: "Day of Month", formula: "$df(d)$" },
        { name: "Day of Month (padded)", formula: "$df(dd)$" },
        { name: "Day of Week (full)", formula: "$df(EEEE)$" },
        { name: "Day of Week (short)", formula: "$df(EEE)$" },
        { name: "Month (number)", formula: "$df(M)$" },
        { name: "Month (short name)", formula: "$df(MMM)$" },
        { name: "Month (full name)", formula: "$df(MMMM)$" },
        { name: "Year (full)", formula: "$df(yyyy)$" },
        { name: "Year (short)", formula: "$df(yy)$" },
        { name: "Full Date", formula: "$df(EEEE, MMMM d, yyyy)$" },
        { name: "Short Date", formula: "$df(MMM d, yyyy)$" },
        { name: "Day of Year", formula: "$df(D)$" },
        { name: "Week of Year", formula: "$df(w)$" },
      ],
    },
    {
      name: "Battery",
      options: [
        { name: "Level (%)", formula: "$bi(level)$" },
        { name: "Status", formula: "$bi(status)$" },
        { name: "Temperature", formula: "$bi(temp)$" },
        { name: "Is Plugged", formula: "$bi(plugged)$" },
        { name: "Battery with %", formula: "$bi(level)$%" },
        { name: "Battery Status Text", formula: '$if(bi(status) = "CHARGING", "Charging", if(bi(status) = "FULL", "Full", "On Battery"))$' },
      ],
    },
    {
      name: "System",
      options: [
        { name: "Hostname", formula: "$si(model)$" },
        { name: "Distro", formula: "$si(man)$" },
        { name: "Kernel", formula: "$si(build)$" },
        { name: "Uptime", formula: "$si(boot)$" },
        { name: "Volume", formula: "$si(volr)$" },
        { name: "Dark Mode", formula: "$si(darkmode)$" },
      ],
    },
    {
      name: "Resources",
      options: [
        { name: "CPU Usage (%)", formula: "$rm(cpuuse)$" },
        { name: "RAM Used (MB)", formula: "$rm(memuse)$" },
        { name: "RAM Total (MB)", formula: "$rm(memtot)$" },
        { name: "RAM Used (GB)", formula: "$mu(round, rm(memuse) / 1024, 1)$" },
        { name: "RAM Total (GB)", formula: "$mu(round, rm(memtot) / 1024, 1)$" },
        { name: "RAM Usage Text", formula: "$mu(round, rm(memuse) / 1024, 1)$ / $mu(round, rm(memtot) / 1024, 1)$ GB" },
        { name: "RAM Free (MB)", formula: "$rm(memfree)$" },
        { name: "Swap Used", formula: "$mu(round, (rm(swptot) - rm(swpfree)) / 1024, 1)$ GB" },
        { name: "Disk Free (GB)", formula: "$mu(round, rm(sdfree) / 1024, 1)$" },
        { name: "Disk Total (GB)", formula: "$mu(round, rm(sdtot) / 1024, 1)$" },
      ],
    },
    {
      name: "Music",
      options: [
        { name: "Title", formula: "$mi(title)$" },
        { name: "Artist", formula: "$mi(artist)$" },
        { name: "Album", formula: "$mi(album)$" },
        { name: "Artist - Title", formula: "$mi(artist)$ - $mi(title)$" },
        { name: "State", formula: "$mi(state)$" },
        { name: "Volume", formula: "$mi(vol)$" },
        { name: "Position (sec)", formula: "$mi(pos)$" },
        { name: "Length (sec)", formula: "$mi(len)$" },
        { name: "Progress (%)", formula: "$mi(percent)$" },
        { name: "Player", formula: "$mi(package)$" },
      ],
    },
    {
      name: "Network",
      options: [
        { name: "Connected", formula: "$nc(connected)$" },
        { name: "WiFi Name", formula: "$nc(ssid)$" },
        { name: "IP Address", formula: "$nc(ip)$" },
        { name: "Signal Strength", formula: "$nc(strength)$" },
        { name: "Connection Type", formula: "$nc(type)$" },
      ],
    },
    {
      name: "Traffic",
      options: [
        { name: "Download Speed", formula: "$ts(srx)$" },
        { name: "Upload Speed", formula: "$ts(stx)$" },
        { name: "Total Downloaded", formula: "$ts(trx)$" },
        { name: "Total Uploaded", formula: "$ts(ttx)$" },
        { name: "Speed (KB/s)", formula: "$mu(round, ts(srx) / 1024, 1)$ KB/s" },
      ],
    },
    {
      name: "Weather",
      options: [
        { name: "Temperature", formula: "$wi(temp)$" },
        { name: "Temperature (C)", formula: "$wi(tempc)$" },
        { name: "Feels Like", formula: "$wi(flik)$" },
        { name: "Condition", formula: "$wi(cond)$" },
        { name: "Icon Code", formula: "$wi(icon)$" },
        { name: "Humidity (%)", formula: "$wi(hum)$" },
        { name: "Wind Speed", formula: "$wi(wspeed)$" },
        { name: "Wind Direction", formula: "$wi(wdir)$" },
        { name: "Pressure", formula: "$wi(press)$" },
        { name: "Cloud Cover (%)", formula: "$wi(clouds)$" },
        { name: "UV Index", formula: "$wi(uvindex)$" },
        { name: "Dew Point", formula: "$wi(dpoint)$" },
        { name: "Weather Text", formula: '$wi(temp)$° $wi(cond)$' },
        { name: "Last Updated", formula: "$wi(updated)$" },
        { name: "Forecast Today", formula: "$wf(0, temp)$" },
        { name: "Forecast Tomorrow", formula: "$wf(1, temp)$" },
        { name: "Tomorrow High/Low", formula: "$wf(1, max)$/$wf(1, min)$" },
        { name: "Tomorrow Condition", formula: "$wf(1, cond)$" },
        { name: "Forecast Icon +1d", formula: "$wf(1, icon)$" },
        { name: "Forecast Icon +2d", formula: "$wf(2, icon)$" },
        { name: "Forecast Day Name", formula: "$wf(2, day)$" },
      ],
    },
    {
      name: "Astronomy",
      options: [
        { name: "Sunrise", formula: "$ai(sunrise)$" },
        { name: "Sunset", formula: "$ai(sunset)$" },
        { name: "Is Daytime", formula: "$ai(isday)$" },
        { name: "Next Sunrise", formula: "$ai(nsunrise)$" },
        { name: "Next Sunset", formula: "$ai(nsunset)$" },
        { name: "Civil Twilight Start", formula: "$ai(csunrise)$" },
        { name: "Civil Twilight End", formula: "$ai(csunset)$" },
        { name: "Moon Phase", formula: "$ai(mphase)$" },
        { name: "Moon Phase Code", formula: "$ai(mphasec)$" },
        { name: "Moon Illumination (%)", formula: "$ai(mill)$" },
        { name: "Moonrise", formula: "$ai(moonrise)$" },
        { name: "Moonset", formula: "$ai(moonset)$" },
        { name: "Zodiac Sign", formula: "$ai(zodiac)$" },
        { name: "Season", formula: "$ai(season)$" },
      ],
    },
    {
      name: "Calendar",
      options: [
        { name: "Next Event Title", formula: "$ci(title, 0)$" },
        { name: "Next Event Start", formula: "$df(hh:mm, ci(start, 0))$" },
        { name: "Next Event End", formula: "$df(hh:mm, ci(end, 0))$" },
        { name: "Next Event Location", formula: "$ci(location, 0)$" },
        { name: "Event 2 Title", formula: "$ci(title, 1)$" },
        { name: "Today's Event Count", formula: "$ci(ecount)$" },
        { name: "Is All Day", formula: "$ci(allday, 0)$" },
        { name: "Event Time Range", formula: "$df(hh:mm, ci(start, 0))$-$df(hh:mm, ci(end, 0))$ $ci(title, 0)$" },
      ],
    },
    {
      name: "Notifications",
      options: [
        { name: "Count", formula: "$ni(count)$" },
        { name: "First Title", formula: "$ni(0, title)$" },
        { name: "First Text", formula: "$ni(0, text)$" },
        { name: "First App", formula: "$ni(0, app)$" },
        { name: "Second Title", formula: "$ni(1, title)$" },
        { name: "Persistent Count", formula: "$ni(scount)$" },
      ],
    },
    {
      name: "Shell",
      options: [
        { name: "Custom Command", formula: '$sh("command")$' },
        { name: "Process Count", formula: '$sh("ps aux | wc -l")$' },
        { name: "Logged Users", formula: '$sh("who | wc -l")$' },
        { name: "Kernel Info", formula: '$sh("uname -r")$' },
        { name: "Package Updates", formula: '$sh("checkupdates | wc -l")$' },
        { name: "Disk Usage (/)", formula: '$sh("df -h / | tail -1 | awk \'{print $5}\'")$' },
      ],
    },
    {
      name: "Web Get",
      options: [
        { name: "JSON Value", formula: '$wg("URL", json, path.to.value, 0)$' },
        { name: "RSS Title", formula: '$wg("URL", rss, feed, 0)$' },
        { name: "RSS Description", formula: '$wg("URL", rss, desc, 0)$' },
        { name: "RSS Link", formula: '$wg("URL", rss, link, 0)$' },
        { name: "RSS Date", formula: '$wg("URL", rss, date, 0)$' },
        { name: "RSS Image", formula: '$wg("URL", rss, img, 0)$' },
        { name: "RSS Item Count", formula: '$wg("URL", rss, cnt)$' },
        { name: "RSS Feed Title", formula: '$wg("URL", rss, ftitle)$' },
        { name: "RSS Cycling Item", formula: '$wg("URL", rss, feed, gv(timer))$' },
        { name: "Plain Text", formula: '$wg("URL", txt)$' },
        { name: "Text Line", formula: '$wg("URL", txt, line, 0)$' },
      ],
    },
    {
      name: "Conditional",
      options: [
        { name: "If/Else", formula: '$if(CONDITION, "true", "false")$' },
        { name: "Time Greeting", formula: '$if(df(H) < 12, "Good Morning", if(df(H) < 18, "Good Afternoon", "Good Evening"))$' },
        { name: "Battery Warning", formula: '$if(bi(level) < 20, "Low Battery!", "")$' },
        { name: "Playing Check", formula: '$if(mi(state) = "PLAYING", mi(artist) + " - " + mi(title), "Not Playing")$' },
        { name: "WiFi Status", formula: '$if(nc(connected) = "1", "Connected: " + nc(ssid), "Disconnected")$' },
        { name: "Day/Night", formula: '$if(ai(isday) = "1", "Day", "Night")$' },
      ],
    },
    {
      name: "Math",
      options: [
        { name: "Add", formula: "$mu(add, A, B)$" },
        { name: "Subtract", formula: "$mu(sub, A, B)$" },
        { name: "Multiply", formula: "$mu(mul, A, B)$" },
        { name: "Divide", formula: "$mu(div, A, B)$" },
        { name: "Modulo", formula: "$mu(mod, A, B)$" },
        { name: "Round", formula: "$mu(round, VALUE, PLACES)$" },
        { name: "Floor", formula: "$mu(floor, VALUE)$" },
        { name: "Ceiling", formula: "$mu(ceil, VALUE)$" },
        { name: "Absolute", formula: "$mu(abs, VALUE)$" },
        { name: "Random", formula: "$mu(rnd, MIN, MAX)$" },
        { name: "Power", formula: "$mu(pow, BASE, EXP)$" },
        { name: "Square Root", formula: "$mu(sqrt, VALUE)$" },
        { name: "Min", formula: "$mu(min, A, B)$" },
        { name: "Max", formula: "$mu(max, A, B)$" },
        { name: "Sine", formula: "$mu(sin, DEGREES)$" },
        { name: "Cosine", formula: "$mu(cos, DEGREES)$" },
        { name: "Hex to Decimal", formula: "$mu(h2d, HEX)$" },
        { name: "Decimal to Hex", formula: "$mu(d2h, NUMBER)$" },
        { name: "Inline Arithmetic", formula: "$mu(add, gv(score), 10)$" },
        { name: "Cycle Timer (10s)", formula: "$mu(mod, mu(floor, mu(div, dp(s), 10)), 10)$" },
      ],
    },
    {
      name: "Text",
      options: [
        { name: "Uppercase", formula: '$tc(up, "text")$' },
        { name: "Lowercase", formula: '$tc(low, "text")$' },
        { name: "Capitalize", formula: '$tc(cap, "text")$' },
        { name: "Length", formula: '$tc(len, "text")$' },
        { name: "Substring", formula: '$tc(cut, "text", START, LENGTH)$' },
        { name: "Truncate", formula: '$tc(ell, "text", MAX_LENGTH)$' },
        { name: "Split", formula: '$tc(split, "text", "delimiter", INDEX)$' },
        { name: "Regex Replace", formula: '$tc(reg, "text", "pattern", "replacement")$' },
        { name: "Count Chars", formula: '$tc(count, "text", "char")$' },
        { name: "Line Count", formula: '$tc(lines, "text")$' },
        { name: "Number to Words", formula: "$tc(n2w, 42)$" },
        { name: "Ordinal", formula: "$tc(ord, 1)$" },
        { name: "Roman Numeral", formula: "$tc(roman, 42)$" },
        { name: "URL Encode", formula: '$tc(url, "text")$' },
        { name: "Strip HTML", formula: '$tc(html, "<b>text</b>")$' },
        { name: "Parse JSON", formula: '$tc(json, "json_string", "$.path")$' },
      ],
    },
    {
      name: "Color",
      options: [
        { name: "Invert", formula: "$ce(#COLOR, invert)$" },
        { name: "Complement", formula: "$ce(#COLOR, comp)$" },
        { name: "Contrast (B/W)", formula: "$ce(#COLOR, contrast)$" },
        { name: "Set Alpha", formula: "$ce(#COLOR, alpha, AMOUNT)$" },
        { name: "Set Saturation", formula: "$ce(#COLOR, sat, AMOUNT)$" },
        { name: "Set Luminance", formula: "$ce(#COLOR, lum, AMOUNT)$" },
        { name: "Desaturate", formula: "$ce(#COLOR, sat, 0)$" },
        { name: "Gradient Mix", formula: "$ce(#COLOR1, #COLOR2, 50)$" },
        { name: "HSL Color", formula: "$cm(HUE, SATURATION, LIGHTNESS)$" },
      ],
    },
    {
      name: "Loops",
      options: [
        { name: "Count 1 to 10", formula: "$fl(1, 10, 1, lv(i), \" \")$" },
        { name: "Count by 2s", formula: "$fl(0, 10, 2, lv(i), \", \")$" },
        { name: "Repeat Symbol", formula: '$fl(1, 5, 1, "#")$' },
        { name: "Countdown", formula: "$fl(10, 1, -1, lv(i), \" \")$" },
        { name: "Custom Loop", formula: "$fl(START, END, STEP, BODY, SEPARATOR)$" },
      ],
    },
    {
      name: "Variables",
      options: [
        { name: "Get Global", formula: "$gv(name)$" },
        { name: "Get Local (in loop)", formula: "$lv(i)$" },
        { name: "Global + Arithmetic", formula: "$mu(add, gv(name), 1)$" },
        { name: "Nested in Web Get", formula: '$wg("URL", rss, feed, gv(index))$' },
        { name: "Nested in Condition", formula: '$if(gv(toggle), "ON", "OFF")$' },
      ],
    },
  ];

  let customText = $state("");
  let isSimpleCategory = $derived(categories[selectedCategory].name === "Simple");

  function selectOption(option: FormulaOption) {
    if (isSimpleCategory) {
      currentFormula = customText;
    } else {
      currentFormula = option.formula;
    }
  }

  function handleInsert() {
    if (currentFormula) {
      onInsert(currentFormula);
      currentFormula = "";
      customText = "";
      onClose();
    }
  }

  function handleDblClick(option: FormulaOption) {
    if (isSimpleCategory) {
      if (customText) {
        onInsert(customText);
        customText = "";
        onClose();
      }
    } else {
      onInsert(option.formula);
      currentFormula = "";
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    }
  }

  function handleOverlayClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("formula-overlay")) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="formula-overlay" onclick={handleOverlayClick}>
    <div class="formula-dialog">
      <div class="dialog-header">
        <span class="dialog-title">Formula Helper</span>
        <button class="close-btn" onclick={onClose} title="Close">&times;</button>
      </div>
      <div class="dialog-body">
        <div class="category-list">
          {#each categories as cat, i}
            <button
              class="category-item"
              class:active={selectedCategory === i}
              onclick={() => { selectedCategory = i; currentFormula = ""; }}
            >
              {cat.name}
            </button>
          {/each}
        </div>
        <div class="options-panel">
          {#if isSimpleCategory}
            <div class="simple-input">
              <label class="option-label">Custom Text</label>
              <input
                type="text"
                bind:value={customText}
                placeholder="Type your text..."
                oninput={() => { currentFormula = customText; }}
              />
            </div>
          {:else}
            {#each categories[selectedCategory].options as option}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="option-item"
                class:selected={currentFormula === option.formula}
                onclick={() => selectOption(option)}
                ondblclick={() => handleDblClick(option)}
              >
                <span class="option-name">{option.name}</span>
                <span class="option-formula">{option.formula}</span>
              </div>
            {/each}
          {/if}
        </div>
      </div>
      <div class="dialog-footer">
        <div class="formula-preview">
          <span class="preview-label">Formula:</span>
          <code class="preview-value">{currentFormula || "(none)"}</code>
        </div>
        <button
          class="insert-btn"
          disabled={!currentFormula}
          onclick={handleInsert}
        >
          Insert
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .formula-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .formula-dialog {
    width: 700px;
    max-width: 90vw;
    height: 500px;
    max-height: 80vh;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .dialog-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    color: var(--text-secondary);
    border-radius: 4px;
  }

  .close-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .dialog-body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .category-list {
    width: 160px;
    min-width: 160px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    padding: 4px 0;
  }

  .category-item {
    display: block;
    width: 100%;
    padding: 8px 16px;
    text-align: left;
    font-size: 13px;
    color: var(--text-secondary);
    border-radius: 0;
  }

  .category-item:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .category-item.active {
    background: var(--accent-dim);
    color: var(--accent);
    font-weight: 500;
  }

  .options-panel {
    flex: 1;
    overflow-y: auto;
    padding: 4px;
  }

  .option-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 12px;
    border-radius: 4px;
    cursor: pointer;
    user-select: none;
  }

  .option-item:hover {
    background: var(--bg-secondary);
  }

  .option-item.selected {
    background: var(--accent-dim);
  }

  .option-name {
    font-size: 13px;
    color: var(--text-primary);
  }

  .option-formula {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    word-break: break-all;
  }

  .simple-input {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .option-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .simple-input input {
    width: 100%;
    padding: 6px 10px;
    font-size: 13px;
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .formula-preview {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .preview-label {
    font-size: 12px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .preview-value {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .insert-btn {
    padding: 6px 20px;
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .insert-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .insert-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
