// Configuration constants for svgdx editor

export const STORAGE_VERSION = 1;
export const JSON_API_VERSION = 1;

// Rate limiting - WASM should handle frequent updates, server may need more time
export const RATE_LIMIT_WASM_MS = 75;
export const RATE_LIMIT_SERVER_MS = 250;
// if a call hangs for some reason, don't block the next call forever
export const IN_PROGRESS_TIMEOUT_MS = 5000;

// Zoom settings
export const ZOOM_DELAY_MS = 50;
export const ZOOM_SPEED = 0.15;
export const MAX_ZOOM_OUT = 10;

// Default content for new tabs
export const DEFAULT_CONTENT = `<svg>
  <!-- Example svgdx document -->
  <rect id="in" wh="20 10" text="input" class="d-softshadow d-fill-azure"/>
  <!-- Try changing the '|h 10' below to '|v 30' or '|V 5' -->
  <rect id="proc" xy="^|h 10" wh="^" text="process" class="d-softshadow d-fill-silver"/>
  <rect id="out" xy="^|h 10" wh="^" text="output" class="d-softshadow d-fill-skyblue"/>

  <line start="#in" end="#proc" class="d-arrow"/>
  <line start="#proc" end="#out" class="d-arrow"/>
</svg>`;

// Valid layout options
export const VALID_LAYOUTS = ['vertical', 'horizontal', 'v-text', 'h-text'];
export const DEFAULT_LAYOUT = 'vertical';

// PNG export resolutions
export const PNG_RESOLUTIONS = {
    'copy-png-big': 2048,
    'copy-png-medium': 1024,
    'copy-png-small': 512,
    'copy-png-tiny': 128
};
