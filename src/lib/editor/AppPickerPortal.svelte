<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  interface Props {
    forPinned: boolean;
    search: string;
    apps: { name: string; exec: string; icon: string; categories: string }[];
    onSearch: (v: string) => void;
    onSelect: (exec: string) => void;
    onClose: () => void;
    iconBg: (name: string) => string;
    iconLetter: (name: string) => string;
  }
  let { forPinned, search, apps, onSearch, onSelect, onClose, iconBg, iconLetter }: Props = $props();

  let portal: HTMLDivElement;
  let listEl: HTMLDivElement;
  let inputEl: HTMLInputElement;
  let rowEls: HTMLDivElement[] = [];
  let rowExecs: string[] = [];
  let rowNames: string[] = [];

  onMount(() => {
    portal = document.createElement("div");
    document.body.appendChild(portal);
    buildShell();
  });

  onDestroy(() => {
    if (portal) portal.remove();
  });

  $effect(() => { void apps; buildRows(); });
  $effect(() => { void search; filterRows(); });

  function buildShell() {
    const backdrop = document.createElement("div");
    backdrop.style.cssText = "position:fixed;inset:0;background:rgba(0,0,0,0.5);z-index:9999;display:flex;align-items:center;justify-content:center;";
    backdrop.addEventListener("mousedown", (e) => { if (e.target === backdrop) onClose(); });

    const modal = document.createElement("div");
    modal.style.cssText = "background:#1e1e2e;border:1px solid #444;border-radius:8px;width:360px;max-height:500px;display:flex;flex-direction:column;box-shadow:0 8px 32px rgba(0,0,0,0.4);";
    modal.addEventListener("mousedown", (e) => e.stopPropagation());

    const header = document.createElement("div");
    header.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:10px 12px;border-bottom:1px solid #444;";
    header.innerHTML = `<span style="font-weight:600;font-size:13px;color:#cdd6f4;">${forPinned ? "Add Pinned App" : "Select App"}</span>`;
    const closeBtn = document.createElement("span");
    closeBtn.style.cssText = "cursor:pointer;font-size:18px;line-height:1;color:#888;";
    closeBtn.textContent = "\u00d7";
    closeBtn.addEventListener("mousedown", onClose);
    header.appendChild(closeBtn);

    inputEl = document.createElement("input");
    inputEl.type = "text";
    inputEl.placeholder = "Search apps...";
    inputEl.style.cssText = "margin:8px;padding:6px 10px;border-radius:4px;border:1px solid #444;background:#181825;color:#cdd6f4;font-size:12px;box-sizing:border-box;outline:none;";
    inputEl.addEventListener("input", () => onSearch(inputEl.value));

    listEl = document.createElement("div");
    listEl.style.cssText = "overflow-y:auto;flex:1;";

    modal.appendChild(header);
    modal.appendChild(inputEl);
    modal.appendChild(listEl);

    if (forPinned) {
      const doneBtn = document.createElement("span");
      doneBtn.style.cssText = "display:block;text-align:center;padding:8px;cursor:pointer;border-top:1px solid #444;color:#cdd6f4;font-size:12px;";
      doneBtn.textContent = "Done";
      doneBtn.addEventListener("mousedown", onClose);
      modal.appendChild(doneBtn);
    }

    backdrop.appendChild(modal);
    portal.appendChild(backdrop);
    inputEl.focus();
  }

  function buildRows() {
    if (!listEl) return;
    listEl.innerHTML = "";
    rowEls = [];
    rowExecs = [];
    rowNames = [];

    if (apps.length === 0) {
      listEl.innerHTML = '<div style="padding:16px;text-align:center;color:#888;font-size:12px;">Loading apps...</div>';
      return;
    }

    for (let i = 0; i < apps.length; i++) {
      const appName = apps[i].name;
      const appExec = apps[i].exec;

      rowExecs.push(appExec);
      rowNames.push(appName.toLowerCase());

      const row = document.createElement("div");
      row.style.cssText = "display:flex;align-items:center;gap:8px;padding:8px 12px;cursor:pointer;border-bottom:1px solid #333;";
      row.addEventListener("mousedown", ((idx: number) => () => {
        onSelect(rowExecs[idx]);
      })(i));
      row.addEventListener("mouseenter", () => { row.style.background = "#2a2a3e"; });
      row.addEventListener("mouseleave", () => { row.style.background = ""; });

      const letter = document.createElement("span");
      letter.style.cssText = `width:28px;height:28px;display:flex;align-items:center;justify-content:center;border-radius:4px;color:#fff;font-size:13px;font-weight:bold;background:${iconBg(appName)};flex-shrink:0;`;
      letter.textContent = iconLetter(appName);

      const info = document.createElement("span");
      info.style.cssText = "flex:1;overflow:hidden;pointer-events:none;";
      info.innerHTML = `<div style="font-size:12px;color:#cdd6f4;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${appName.replace(/</g,"&lt;")}</div><div style="font-size:10px;color:#888;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${appExec.split(" ")[0].replace(/</g,"&lt;")}</div>`;

      row.appendChild(letter);
      row.appendChild(info);
      listEl.appendChild(row);
      rowEls.push(row);
    }

    filterRows();
  }

  function filterRows() {
    const q = search.toLowerCase();
    for (let i = 0; i < rowEls.length; i++) {
      rowEls[i].style.display = rowNames[i].includes(q) ? "flex" : "none";
    }
  }
</script>
