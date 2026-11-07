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

// Default content for new tabs, loaded from svgdx-bootstrap.js
export const DEFAULT_CONTENT = window.svgdx_example_document;

// Valid layout options
export const VALID_LAYOUTS = ['vertical', 'horizontal', 'v-text', 'h-text'];
export const DEFAULT_LAYOUT = 'vertical';

// PNG export resolutions
export const PNG_RESOLUTIONS = [2048, 1024, 512, 128];

// Mobile layout settings
export const MOBILE_BREAKPOINT = 768;
export const VALID_MOBILE_LAYOUTS = ['svg', 'xml'];
export const DEFAULT_MOBILE_LAYOUT = 'svg';

// Slider defaults
export const DEFAULT_SLIDER_VALUE = 0.5;
export const SLIDER_STEP = 0.01;
export const SLIDER_MIN = 0;
export const SLIDER_MAX = 1;
