// DOM utilities and element references

// Main containers
export const container = document.querySelector('.container');
export const editorContainer = document.querySelector('#editor-container');
export const outputContainer = document.querySelector('#output-container');
export const svgOutputContainer = document.querySelector('#svg-output');
export const textOutputContainer = document.querySelector('#text-output');
export const errorOutput = document.querySelector('#error-output');
export const statusbar = document.querySelector('#statusbar');

// Splitters
export const mainSplit = document.getElementById('main-split');
export const outputSplit = document.getElementById('output-split');

/**
 * Convert client coordinates to SVG user-space coordinates
 * Uses SVGPoint and inverse screen coordinate transform matrix
 * as deriving from boundingbox and viewbox ratios is tricky due
 * to potential flexbox scaling/shrinking.
 */
export function clientToSvg(svg, x, y) {
    const pt = svg.createSVGPoint();
    pt.x = x;
    pt.y = y;
    const svgPos = pt.matrixTransform(svg.getScreenCTM().inverse());
    return { x: svgPos.x, y: svgPos.y };
}
