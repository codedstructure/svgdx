// Viewport module - handles SVG pan, zoom, and viewBox management

import { ZOOM_DELAY_MS, ZOOM_SPEED, MAX_ZOOM_OUT } from './config.js';
import { svgOutputContainer, clientToSvg } from './dom.js';

/**
 * Initialize zoom functionality (scroll wheel)
 */
export function initZoom() {
    let busy = false;

    svgOutputContainer.addEventListener('wheel', (e) => {
        e.preventDefault();

        if (busy) return;

        const factor = Math.sign(e.deltaY) * ZOOM_SPEED;
        const svg = svgOutputContainer.querySelector('svg');
        if (!svg) return;

        const vb = svg.viewBox.baseVal;
        const eventPos = clientToSvg(svg, e.clientX, e.clientY);

        // Calculate new dimensions
        const newWidth = vb.width * (1 + factor);
        const newHeight = vb.height * (1 + factor);

        // Limit zoom-in to 1 user-space unit
        if (newWidth < 1 || newHeight < 1) return;

        // Limit zoom-out to MAX_ZOOM_OUT times original size
        const origWidth = svg.dataset.origWidth ? parseFloat(svg.dataset.origWidth) : null;
        const origHeight = svg.dataset.origHeight ? parseFloat(svg.dataset.origHeight) : null;
        if (origWidth === null || origHeight === null ||
            newWidth > origWidth * MAX_ZOOM_OUT ||
            newHeight > origHeight * MAX_ZOOM_OUT) {
            return;
        }

        // Calculate new position to keep mouse point fixed
        const newX = vb.x - (newWidth - vb.width) * ((eventPos.x - vb.x) / vb.width);
        const newY = vb.y - (newHeight - vb.height) * ((eventPos.y - vb.y) / vb.height);

        svg.setAttribute('viewBox', `${newX} ${newY} ${newWidth} ${newHeight}`);

        busy = true;
        setTimeout(() => { busy = false; }, ZOOM_DELAY_MS);
    });
}

/**
 * Initialize pan functionality (mouse drag)
 */
export function initPan() {
    let isDragging = false;
    let startX, startY;

    // Use pointer events for unified touch + mouse support
    svgOutputContainer.addEventListener('pointerdown', (e) => {
        // Left mouse button or touch
        if (e.button !== 0 && e.pointerType === 'mouse') return;

        const svg = svgOutputContainer.querySelector('svg');
        if (e.target.closest('#svg-output > svg') === svg) {
            e.preventDefault();
            isDragging = true;
            startX = e.clientX;
            startY = e.clientY;
            document.body.style.cursor = 'move';
            // Capture pointer to receive events outside element
            svgOutputContainer.setPointerCapture(e.pointerId);
        }
    });

    svgOutputContainer.addEventListener('pointermove', (e) => {
        if (!isDragging) return;
        e.preventDefault();

        const svg = svgOutputContainer.querySelector('svg');
        if (!svg) return;

        const oldPos = clientToSvg(svg, startX, startY);
        const newPos = clientToSvg(svg, e.clientX, e.clientY);
        const dx = oldPos.x - newPos.x;
        const dy = oldPos.y - newPos.y;

        const vb = svg.viewBox.baseVal;
        svg.setAttribute('viewBox', `${vb.x + dx} ${vb.y + dy} ${vb.width} ${vb.height}`);

        startX = e.clientX;
        startY = e.clientY;
    });

    svgOutputContainer.addEventListener('pointerup', (e) => {
        if (isDragging) {
            isDragging = false;
            document.body.style.cursor = 'default';
            svgOutputContainer.releasePointerCapture(e.pointerId);
        }
    });

    svgOutputContainer.addEventListener('pointercancel', (e) => {
        if (isDragging) {
            isDragging = false;
            document.body.style.cursor = 'default';
        }
    });
}

/**
 * Reset the SVG viewBox to original values
 */
export function resetView() {
    const svg = svgOutputContainer.querySelector('svg');
    if (svg && svg.dataset.origViewbox) {
        svg.setAttribute('viewBox', svg.dataset.origViewbox);
    }
}

/**
 * Get the current viewBox string
 */
export function getCurrentViewBox() {
    const svg = svgOutputContainer.querySelector('svg');
    return svg ? svg.getAttribute('viewBox') : null;
}

/**
 * Initialize viewport controls (reset view, auto-fit buttons)
 */
export function initViewportControls(state, onUpdate) {
    // Reset view button
    document.getElementById('reset-view').addEventListener('click', resetView);

    // Auto-fit button
    const autoViewbox = document.getElementById('auto-viewbox');
    autoViewbox.dataset.checked = state.autoViewbox ? 'true' : 'false';

    autoViewbox.addEventListener('click', () => {
        state.autoViewbox = !state.autoViewbox;
        autoViewbox.dataset.checked = state.autoViewbox ? 'true' : 'false';

        // Import saveState here to avoid circular dependency
        import('./storage.js').then(({ saveState }) => {
            saveState(state);
        });

        if (onUpdate) {
            onUpdate();
        }
    });
}

/**
 * Initialize all viewport functionality
 */
export function initViewport(state, onUpdate) {
    initZoom();
    initPan();
    initViewportControls(state, onUpdate);
}
