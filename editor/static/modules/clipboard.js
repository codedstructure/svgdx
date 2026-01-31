// Clipboard module - handles copy, download, and PNG export

import { PNG_RESOLUTIONS } from './config.js';
import { statusbar } from './dom.js';
import { hidePopup } from './layout.js';
import { transform } from './transform.js';

/**
 * Copy data to clipboard
 * Note: Safari requires clipboard actions happen in an event handler triggered
 * by a user action; resolving a promise using await prior to clipboard.write()
 * defeats that, so always use .write() (which takes a ClipboardItem which *can*
 * resolve a Promise) even for text.
 */
function copyToClipboard(mimeType, dataPromise) {
    try {
        navigator.clipboard.write([
            new ClipboardItem({
                [mimeType]: dataPromise
            })
        ]);
        statusbar.style.color = null;
        statusbar.innerText = 'Copied to clipboard';
    } catch (e) {
        console.error('Error copying to clipboard', e);
        statusbar.style.color = 'darkred';
        statusbar.innerText = 'Failed to copy to clipboard';
    }
}

/**
 * Get clean SVG output (without metadata)
 */
async function getCleanSvg(editor) {
    const result = await transform(editor.getValue(), false);
    if (result.ok) {
        return cleanText(result.svg);
    } else {
        statusbar.style.color = 'darkred';
        statusbar.innerText = `Error retrieving SVG: ${result.error}`;
        throw new Error(result.error);
    }
}

/**
 * Clean text by removing trailing whitespace on each line
 * and ensuring it ends with a single newline
 */
export function cleanText(text) {
    return text.split('\n').map(line => line.trimEnd()).join('\n').trimEnd() + '\n';
}

/**
 * Generate a timestamp string for filenames (YYYY-MM-DD-HHMMSS)
 */
export function getTimestamp() {
    const date = new Date();
    const pad2 = n => String(n).padStart(2, '0');
    return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}-${pad2(date.getHours())}${pad2(date.getMinutes())}${pad2(date.getSeconds())}`;
}

/**
 * Generate PNG from SVG at specified resolution
 */
async function generatePng(maxDim = 2048) {
    // Clone the SVG to avoid visual glitches
    const svg = document.querySelector('#svg-output svg').cloneNode(true);

    // Restore original dimensions
    svg.setAttribute('width', svg.dataset.origWidth);
    svg.setAttribute('height', svg.dataset.origHeight);
    svg.setAttribute('viewBox', svg.dataset.origViewbox);

    // Scale to maximum dimension
    let pxWidth = svg.width.baseVal.value;
    let pxHeight = svg.height.baseVal.value;

    if (pxWidth > pxHeight) {
        pxHeight = (maxDim / pxWidth) * pxHeight;
        pxWidth = maxDim;
    } else {
        pxWidth = (maxDim / pxHeight) * pxWidth;
        pxHeight = maxDim;
    }
    svg.setAttribute('width', pxWidth);
    svg.setAttribute('height', pxHeight);

    // Create image from SVG
    const img = new Image();
    img.src = URL.createObjectURL(new Blob([svg.outerHTML], { type: 'image/svg+xml' }));
    img.width = pxWidth;
    img.height = pxHeight;

    await new Promise(resolve => { img.onload = resolve; });

    // Draw to canvas
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    canvas.width = img.width;
    canvas.height = img.height;
    context.drawImage(img, 0, 0);

    URL.revokeObjectURL(img.src);

    // Convert to PNG blob
    return new Promise(resolve => {
        canvas.toBlob(blob => resolve(blob), 'image/png');
    });
}

/**
 * Initialize clipboard functionality
 */
export function initClipboard(editor) {
    // Save input button
    document.getElementById('save-input').addEventListener('click', () => {
        hidePopup();
        const blob = new Blob([cleanText(editor.getValue())], { type: 'application/xml' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `svgdx-editor-${getTimestamp()}.xml`;
        a.click();
        URL.revokeObjectURL(url);
    });

    // Copy input button
    document.getElementById('copy-input').addEventListener('click', () => {
        hidePopup();
        copyToClipboard('text/plain', Promise.resolve(cleanText(editor.getValue())));
    });

    // Save output button
    document.getElementById('save-output').addEventListener('click', async () => {
        hidePopup();
        try {
            const svg = await getCleanSvg(editor);
            const blob = new Blob([svg], { type: 'image/svg+xml' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `svgdx-output-${getTimestamp()}.svg`;
            a.click();
            URL.revokeObjectURL(url);
        } catch (e) {
            // Error already displayed by getCleanSvg
        }
    });

    // Copy output button
    document.getElementById('copy-output').addEventListener('click', () => {
        hidePopup();
        copyToClipboard('text/plain', getCleanSvg(editor));
    });

    // Copy PNG buttons
    document.querySelectorAll('#copy-popup .popup-button').forEach(el => {
        el.addEventListener('click', async (e) => {
            hidePopup();
            const id = e.target.id;
            const resolution = PNG_RESOLUTIONS[id];

            if (resolution === undefined) {
                console.error(`Unknown copy PNG button: ${id}`);
                return;
            }

            try {
                copyToClipboard('image/png', generatePng(resolution));
                console.log(`PNG image copied to clipboard (${resolution}px)`);
            } catch (error) {
                console.error('Error copying PNG image to clipboard:', error);
            }
        });
    });
}
