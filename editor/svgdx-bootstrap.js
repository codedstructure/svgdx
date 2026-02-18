// svgdx bootstrap - WASM mode
// Loads the WASM module and sets up the transform function

import init, { transform_json } from "./pkg/svgdx.js";

console.log("svgdx: using local transform (WASM)");

async function run() {
    await init();
    window.svgdx_transform_json = transform_json;
    window.svgdx_use_server = false;
    // Dispatch event to notify that svgdx is ready
    window.dispatchEvent(new Event('svgdx-ready'));
}

run();
