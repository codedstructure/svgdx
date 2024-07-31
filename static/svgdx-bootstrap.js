import init, {transform_string} from "./pkg/svgdx.js";
console.log("svgdx: using local transform");
async function run() {
    await init();
    window.svgdx_transform = transform_string;
    window.svgdx_use_server = false;
}
run();
