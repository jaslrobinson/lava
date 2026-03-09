import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import fs from "node:fs";
import path from "node:path";

/** Serve local asset files at /__klwp_assets/<absolute-path> for the wallpaper WebView */
function klwpAssetsPlugin(): Plugin {
  return {
    name: "klwp-assets",
    configureServer(server) {
      server.middlewares.use("/__klwp_assets", (req, res, next) => {
        const filePath = decodeURIComponent(req.url || "");
        if (!filePath || !path.isAbsolute(filePath)) {
          res.statusCode = 400;
          res.end("Bad path");
          return;
        }
        fs.stat(filePath, (err, stat) => {
          if (err || !stat.isFile()) {
            res.statusCode = 404;
            res.end("Not found");
            return;
          }
          const ext = path.extname(filePath).toLowerCase();
          const mimeTypes: Record<string, string> = {
            ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
            ".gif": "image/gif", ".webp": "image/webp", ".svg": "image/svg+xml",
            ".bmp": "image/bmp",
          };
          res.setHeader("Content-Type", mimeTypes[ext] || "application/octet-stream");
          res.setHeader("Cache-Control", "max-age=3600");
          fs.createReadStream(filePath).pipe(res);
        });
      });
    },
  };
}

/** Serve provider data from temp file so wallpaper WebKitGTK view can poll it */
function klwpProviderPlugin(): Plugin {
  return {
    name: "klwp-providers",
    configureServer(server) {
      server.middlewares.use("/__klwp_providers", (_req, res) => {
        // Must match Rust's std::env::temp_dir() which uses TMPDIR or /tmp
        const filePath = path.join(
          process.env.TMPDIR || "/tmp",
          "klwp-provider-data.json",
        );
        fs.readFile(filePath, "utf-8", (err, data) => {
          res.setHeader("Content-Type", "application/json");
          res.setHeader("Cache-Control", "no-cache");
          res.setHeader("Access-Control-Allow-Origin", "*");
          if (err) {
            res.end("{}");
          } else {
            res.end(data);
          }
        });
      });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), klwpAssetsPlugin(), klwpProviderPlugin()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/*.klwp", "**/*_assets/**"],
    },
  },
});
