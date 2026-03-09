export interface FontEntry {
  name: string;
  family: string;
  category: "sans-serif" | "serif" | "mono" | "display";
}

// Common fonts available on Linux (Arch) desktops
export const SYSTEM_FONTS: FontEntry[] = [
  // Sans-serif
  { name: "Sans Serif", family: "sans-serif", category: "sans-serif" },
  { name: "Arial", family: "Arial, sans-serif", category: "sans-serif" },
  { name: "Helvetica", family: "Helvetica, Arial, sans-serif", category: "sans-serif" },
  { name: "Inter", family: "Inter, sans-serif", category: "sans-serif" },
  { name: "Roboto", family: "Roboto, sans-serif", category: "sans-serif" },
  { name: "Noto Sans", family: "Noto Sans, sans-serif", category: "sans-serif" },
  { name: "Open Sans", family: "Open Sans, sans-serif", category: "sans-serif" },
  { name: "Ubuntu", family: "Ubuntu, sans-serif", category: "sans-serif" },
  { name: "Cantarell", family: "Cantarell, sans-serif", category: "sans-serif" },
  { name: "DejaVu Sans", family: "DejaVu Sans, sans-serif", category: "sans-serif" },
  { name: "Liberation Sans", family: "Liberation Sans, sans-serif", category: "sans-serif" },
  { name: "Lato", family: "Lato, sans-serif", category: "sans-serif" },
  { name: "Poppins", family: "Poppins, sans-serif", category: "sans-serif" },
  { name: "Montserrat", family: "Montserrat, sans-serif", category: "sans-serif" },
  { name: "Oswald", family: "Oswald, sans-serif", category: "sans-serif" },
  // Serif
  { name: "Serif", family: "serif", category: "serif" },
  { name: "Times New Roman", family: "Times New Roman, serif", category: "serif" },
  { name: "Georgia", family: "Georgia, serif", category: "serif" },
  { name: "Noto Serif", family: "Noto Serif, serif", category: "serif" },
  { name: "DejaVu Serif", family: "DejaVu Serif, serif", category: "serif" },
  { name: "Liberation Serif", family: "Liberation Serif, serif", category: "serif" },
  { name: "Playfair Display", family: "Playfair Display, serif", category: "serif" },
  // Monospace
  { name: "Monospace", family: "monospace", category: "mono" },
  { name: "JetBrains Mono", family: "JetBrains Mono, monospace", category: "mono" },
  { name: "Fira Code", family: "Fira Code, monospace", category: "mono" },
  { name: "Source Code Pro", family: "Source Code Pro, monospace", category: "mono" },
  { name: "DejaVu Sans Mono", family: "DejaVu Sans Mono, monospace", category: "mono" },
  { name: "Liberation Mono", family: "Liberation Mono, monospace", category: "mono" },
  { name: "Cascadia Code", family: "Cascadia Code, monospace", category: "mono" },
  // Display
  { name: "Impact", family: "Impact, sans-serif", category: "display" },
  { name: "Comic Sans MS", family: "Comic Sans MS, cursive", category: "display" },
  { name: "Permanent Marker", family: "Permanent Marker, cursive", category: "display" },
  { name: "Bebas Neue", family: "Bebas Neue, sans-serif", category: "display" },
];
