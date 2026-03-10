/**
 * Audio band data module for the music visualizer.
 * - In Tauri mode: listens to "audio-bands" events from the backend.
 * - In wallpaper mode: polls /__klwp_audio every 33ms.
 */

export const NUM_BANDS = 24;

// Current smoothed band values (0.0 - 1.0)
let bands: Float32Array = new Float32Array(NUM_BANDS);
// Peak hold values for each band
let peaks: Float32Array = new Float32Array(NUM_BANDS);

let initialized = false;

export function getAudioBands(): Float32Array {
  return bands;
}

export function getAudioPeaks(): Float32Array {
  return peaks;
}

/** Update bands from raw array, computing smooth peaks */
function updateBands(raw: number[]) {
  const PEAK_DECAY = 0.985;
  for (let i = 0; i < NUM_BANDS && i < raw.length; i++) {
    bands[i] = raw[i];
    if (bands[i] > peaks[i]) {
      peaks[i] = bands[i];
    } else {
      peaks[i] *= PEAK_DECAY;
    }
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
    // Wallpaper/non-Tauri mode: poll /__klwp_audio endpoint
    setInterval(async () => {
      try {
        const res = await fetch("/__klwp_audio");
        const data: number[] = await res.json();
        updateBands(data);
      } catch {
        // Silently ignore — audio may not be available
      }
    }, 33); // ~30fps polling
  }
}
