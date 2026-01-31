// svgdx bootstrap - Server mode
// Sets flag to use server-side transform instead of WASM

console.log("svgdx: using server transform");
window.svgdx_use_server = true;
// Dispatch event to notify that svgdx is ready
window.dispatchEvent(new Event('svgdx-ready'));
