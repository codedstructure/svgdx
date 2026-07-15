// Statusbar module - handles status display and source line navigation

import { svgOutputContainer, statusbar, clientToSvg } from './dom.js';

export function getDefaultStatusText() {
    return window.svgdx_version_label || 'svgdx';
}

export function formatStatusError(message) {
    return `${getDefaultStatusText()} - error: ${message}`;
}

/**
 * Initialize statusbar functionality
 * @param {Object} editor - Editor adapter instance
 */
export function initStatusbar(editor) {
    statusbar.innerText = getDefaultStatusText();

    // Mouse/pointer move updates statusbar with context info
    document.addEventListener('pointermove', (e) => {
        const svg = svgOutputContainer.querySelector('svg');

        // Show tooltip for elements with data-info attribute
        if (typeof e.target.dataset.info !== 'undefined') {
            statusbar.innerText = e.target.dataset.info;
            return;
        }

        // Show SVG position and element info when over SVG
        if (svg !== null && e.target.closest('div > svg') === svg) {
            // Clear previous highlights
            editor.clearHighlight();

            // Display mouse position in SVG user-space coordinates
            const svgPos = clientToSvg(svg, e.clientX, e.clientY);
            const posText = `${svgPos.x.toFixed(2)}, ${svgPos.y.toFixed(2)}`;
            let statusText = posText.padEnd(20, ' ');

            // Don't report on the background
            if (e.target !== svg) {
                let hoverElement = e.target;
                // Handle tspan as part of text element
                if (e.target.tagName === 'tspan') {
                    hoverElement = e.target.closest('text');
                }

                // Highlight source line in editor
                if (hoverElement.dataset.srcLine) {
                    const lineNumber = parseInt(hoverElement.dataset.srcLine);
                    editor.highlightLine(lineNumber);
                    statusText += ` ${lineNumber}:`;
                }

                // Show element info
                const tag = hoverElement.tagName;
                if (tag) {
                    statusText += ` ${tag}`;
                }

                const id = hoverElement.getAttribute('id');
                if (id) {
                    statusText += ` id="${id}"`;
                }

                const href = hoverElement.getAttribute('href');
                if (href) {
                    statusText += ` href="${href}"`;
                }

                const className = hoverElement.getAttribute('class');
                if (className) {
                    statusText += ` class="${className}"`;
                }
            }

            statusbar.innerText = statusText;
        } else {
            statusbar.innerText = getDefaultStatusText();
        }
    });

    // Double-click on SVG element to jump to source line
    document.addEventListener('dblclick', (e) => {
        const svg = svgOutputContainer.querySelector('svg');

        // Must be an element within the SVG output, not the background
        if (svg !== null && e.target.closest('div > svg') === svg && e.target !== svg) {
            let targetElement = e.target;
            if (e.target.tagName === 'tspan') {
                targetElement = e.target.closest('text');
            }

            const srcLineData = targetElement.dataset.srcLine;
            if (srcLineData) {
                const lineNumber = parseInt(srcLineData);
                editor.setCursorLine(lineNumber);
                editor.focus();
            }
        }
    });
}

/**
 * Set statusbar message
 */
export function setStatus(message, isError = false) {
    statusbar.style.color = isError ? 'darkred' : null;
    statusbar.innerText = message;
}

/**
 * Clear error styling from statusbar
 */
export function clearStatusError() {
    statusbar.style.color = null;
}
