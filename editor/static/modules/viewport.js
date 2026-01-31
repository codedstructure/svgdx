// Viewport module - handles SVG pan, zoom, and viewBox management

import { ZOOM_DELAY_MS, ZOOM_SPEED, MAX_ZOOM_OUT } from './config.js';
import { svgOutputContainer, clientToSvg } from './dom.js';

/**
 * Apply zoom to SVG viewBox centered on a point
 * @param {SVGElement} svg - The SVG element
 * @param {number} factor - Zoom factor (positive = zoom out, negative = zoom in)
 * @param {number} centerX - Client X coordinate to zoom towards
 * @param {number} centerY - Client Y coordinate to zoom towards
 * @returns {boolean} - Whether zoom was applied
 */
function applyZoom(svg, factor, centerX, centerY) {
    if (!svg) return false;

    const vb = svg.viewBox.baseVal;
    const eventPos = clientToSvg(svg, centerX, centerY);

    // Calculate new dimensions
    const newWidth = vb.width * (1 + factor);
    const newHeight = vb.height * (1 + factor);

    // Limit zoom-in to 1 user-space unit
    if (newWidth < 1 || newHeight < 1) return false;

    // Limit zoom-out to MAX_ZOOM_OUT times original size
    const origWidth = svg.dataset.origWidth ? parseFloat(svg.dataset.origWidth) : null;
    const origHeight = svg.dataset.origHeight ? parseFloat(svg.dataset.origHeight) : null;
    if (origWidth === null || origHeight === null ||
        newWidth > origWidth * MAX_ZOOM_OUT ||
        newHeight > origHeight * MAX_ZOOM_OUT) {
        return false;
    }

    // Calculate new position to keep center point fixed
    const newX = vb.x - (newWidth - vb.width) * ((eventPos.x - vb.x) / vb.width);
    const newY = vb.y - (newHeight - vb.height) * ((eventPos.y - vb.y) / vb.height);

    svg.setAttribute('viewBox', `${newX} ${newY} ${newWidth} ${newHeight}`);
    return true;
}

/**
 * Initialize zoom functionality (scroll wheel)
 */
export function initWheelZoom() {
    let busy = false;

    svgOutputContainer.addEventListener('wheel', (e) => {
        e.preventDefault();

        if (busy) return;

        const factor = Math.sign(e.deltaY) * ZOOM_SPEED;
        const svg = svgOutputContainer.querySelector('svg');

        if (applyZoom(svg, factor, e.clientX, e.clientY)) {
            busy = true;
            setTimeout(() => { busy = false; }, ZOOM_DELAY_MS);
        }
    });
}

// Module-level state to coordinate between pan and pinch handlers
let isPinching = false;

/**
 * Calculate distance between two touch points
 */
function getTouchDistance(touch1, touch2) {
    const dx = touch2.clientX - touch1.clientX;
    const dy = touch2.clientY - touch1.clientY;
    return Math.sqrt(dx * dx + dy * dy);
}

/**
 * Get midpoint between two touch points
 */
function getTouchMidpoint(touch1, touch2) {
    return {
        x: (touch1.clientX + touch2.clientX) / 2,
        y: (touch1.clientY + touch2.clientY) / 2
    };
}

/**
 * Initialize pinch-to-zoom functionality for touch devices
 */
export function initPinchZoom() {
    // Track pinch state
    let pinchActive = false;
    let lastDistance = null;
    // Track the SVG-space anchor point to keep stable during zoom
    let anchorSvgPoint = null;
    let anchorClientPoint = null;

    svgOutputContainer.addEventListener('touchstart', (e) => {
        if (e.touches.length === 2) {
            e.preventDefault();
            // Two fingers down - prepare for pinch but don't set values yet
            // We'll initialize on first move to avoid jump when second finger lands
            pinchActive = true;
            isPinching = true; // Module-level flag to stop pan handler
            lastDistance = null;
            anchorSvgPoint = null;
            anchorClientPoint = null;
        }
    }, { passive: false });

    svgOutputContainer.addEventListener('touchmove', (e) => {
        if (e.touches.length === 2 && pinchActive) {
            e.preventDefault();

            const svg = svgOutputContainer.querySelector('svg');
            if (!svg) return;

            const currentDistance = getTouchDistance(e.touches[0], e.touches[1]);
            const currentMidpoint = getTouchMidpoint(e.touches[0], e.touches[1]);

            // Initialize on first move after two fingers are down
            if (lastDistance === null) {
                lastDistance = currentDistance;
                anchorClientPoint = currentMidpoint;
                anchorSvgPoint = clientToSvg(svg, currentMidpoint.x, currentMidpoint.y);
                return; // Skip this frame to avoid initial jump
            }

            // Calculate zoom factor from distance change
            // Pinch in (distance decreasing) = zoom out (positive factor)
            // Pinch out (distance increasing) = zoom in (negative factor)
            const scaleFactor = lastDistance / currentDistance;
            const factor = (scaleFactor - 1);

            // Apply zoom centered on the original anchor point (in client coords)
            // This keeps the point between fingers stable
            if (applyZoom(svg, factor, anchorClientPoint.x, anchorClientPoint.y)) {
                // After zoom, the anchor may have drifted - correct by panning
                // to keep the SVG anchor point under the current midpoint
                const currentAnchorClient = getTouchMidpoint(e.touches[0], e.touches[1]);
                const vb = svg.viewBox.baseVal;
                const nowSvgPoint = clientToSvg(svg, currentAnchorClient.x, currentAnchorClient.y);

                // Pan to keep anchor stable
                const dx = nowSvgPoint.x - anchorSvgPoint.x;
                const dy = nowSvgPoint.y - anchorSvgPoint.y;
                svg.setAttribute('viewBox', `${vb.x - dx} ${vb.y - dy} ${vb.width} ${vb.height}`);

                // Update client anchor to track finger movement
                anchorClientPoint = currentAnchorClient;
            }

            lastDistance = currentDistance;
        }
    }, { passive: false });

    svgOutputContainer.addEventListener('touchend', (e) => {
        if (e.touches.length < 2) {
            // Less than two fingers - end pinch
            pinchActive = false;
            isPinching = false;
            lastDistance = null;
            anchorSvgPoint = null;
            anchorClientPoint = null;
        }
    });

    svgOutputContainer.addEventListener('touchcancel', () => {
        pinchActive = false;
        isPinching = false;
        lastDistance = null;
        anchorSvgPoint = null;
        anchorClientPoint = null;
    });
}

/**
 * Initialize pan functionality (mouse drag)
 */
export function initPan() {
    let isDragging = false;
    let startX, startY;
    let activePointerId = null;

    // Use pointer events for unified touch + mouse support
    svgOutputContainer.addEventListener('pointerdown', (e) => {
        // Left mouse button or touch
        if (e.button !== 0 && e.pointerType === 'mouse') return;
        // Don't start pan if we're already pinching
        if (isPinching) return;

        const svg = svgOutputContainer.querySelector('svg');
        if (e.target.closest('#svg-output > svg') === svg) {
            e.preventDefault();
            isDragging = true;
            activePointerId = e.pointerId;
            startX = e.clientX;
            startY = e.clientY;
            document.body.style.cursor = 'move';
            // Capture pointer to receive events outside element
            svgOutputContainer.setPointerCapture(e.pointerId);
        }
    });

    svgOutputContainer.addEventListener('pointermove', (e) => {
        // Stop pan if pinch has started (second finger arrived)
        if (isPinching && isDragging) {
            isDragging = false;
            document.body.style.cursor = 'default';
            if (activePointerId !== null) {
                try { svgOutputContainer.releasePointerCapture(activePointerId); } catch (_) {}
            }
            activePointerId = null;
            return;
        }

        if (!isDragging) return;
        // Only respond to the pointer that started the drag
        if (e.pointerId !== activePointerId) return;
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
        if (isDragging && e.pointerId === activePointerId) {
            isDragging = false;
            activePointerId = null;
            document.body.style.cursor = 'default';
            try { svgOutputContainer.releasePointerCapture(e.pointerId); } catch (_) {}
        }
    });

    svgOutputContainer.addEventListener('pointercancel', (e) => {
        if (isDragging && e.pointerId === activePointerId) {
            isDragging = false;
            activePointerId = null;
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
    initWheelZoom();
    initPinchZoom();
    initPan();
    initViewportControls(state, onUpdate);
}
