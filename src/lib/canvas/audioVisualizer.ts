/**
 * Audio band data module for the music visualizer.
 * - In Tauri mode: listens to "audio-bands" events from the backend.
 * - In wallpaper mode: polls /__lava_audio every 33ms only when music is playing.
 * - Gate: only processes audio when a music app is actively playing (via MPRIS/mi provider),
 *   preventing browser/system sounds from triggering the visualizer.
 */

import { markDirty } from "./renderScheduler";

export const NUM_BANDS = 24;

// Current smoothed band values (0.0 - 1.0)
let bands: Float32Array = new Float32Array(NUM_BANDS);
// Peak hold values for each band
let peaks: Float32Array = new Float32Array(NUM_BANDS);

let initialized = false;
let musicPlaying = false; // set by provider data check
let audioPollId: ReturnType<typeof setInterval> | null = null;

export function getAudioBands(): Float32Array {
  return bands;
}

export function getAudioPeaks(): Float32Array {
  return peaks;
}

/** Called by formula service when provider data updates to gate audio on music state */
export function setMusicPlaying(playing: boolean) {
  const wasPlaying = musicPlaying;
  musicPlaying = playing;

  // Start/stop audio polling based on music state (wallpaper mode only)
  if (playing && !wasPlaying && audioPollId === null && initialized) {
    startAudioPolling();
  } else if (!playing && wasPlaying) {
    stopAudioPolling();
    // Zero out bands when music stops
    bands.fill(0);
    peaks.fill(0);
  }
}

/** Update bands from raw array, computing smooth peaks */
function updateBands(raw: number[]) {
  if (!musicPlaying) return; // ignore audio data when no music app is playing

  const PEAK_DECAY = 0.985;
  for (let i = 0; i < NUM_BANDS && i < raw.length; i++) {
    bands[i] = raw[i];
    if (bands[i] > peaks[i]) {
      peaks[i] = bands[i];
    } else {
      peaks[i] *= PEAK_DECAY;
    }
  }

  // Wake renderer if audio is playing
  const hasAudio = bands.some(b => b > 0.01);
  if (hasAudio) markDirty();
}

function startAudioPolling() {
  if (audioPollId !== null) return;
  audioPollId = setInterval(async () => {
    try {
      const res = await fetch("/__lava_audio");
      const data: number[] = await res.json();
      updateBands(data);
    } catch {
      // Silently ignore — audio may not be available
    }
  }, 33); // ~30fps polling
}

function stopAudioPolling() {
  if (audioPollId !== null) {
    clearInterval(audioPollId);
    audioPollId = null;
  }
}

/** Initialize audio band tracking. Call once from CanvasRenderer. */
export function initAudioVisualizer() {
  if (initialized) return;
  initialized = true;

  const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

  if (isTauri) {
    // Tauri mode: listen to backend events
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<number[]>("audio-bands", (event) => {
        updateBands(event.payload);
      });
    });
  } else {
    // Wallpaper mode: only start polling when music is actually playing
    // The formula service will call setMusicPlaying(true) when mi.state = PLAYING
    if (musicPlaying) {
      startAudioPolling();
    }
  }
}
