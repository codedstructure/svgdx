// Layout module - handles editor layout modes

import { VALID_LAYOUTS, DEFAULT_LAYOUT, MOBILE_BREAKPOINT, VALID_MOBILE_LAYOUTS, DEFAULT_MOBILE_LAYOUT } from './config.js';
import { container, editorContainer, outputContainer, svgOutputContainer, textOutputContainer } from './dom.js';
import { saveState } from './storage.js';
import { hideAllPopups } from './toolbar.js';

// Re-export for backward compatibility with other modules
export { hideAllPopups as hidePopup };

/**
 * Check if we're in mobile mode (either dimension < breakpoint)
 */
export function isMobile() {
    return window.innerWidth < MOBILE_BREAKPOINT || window.innerHeight < MOBILE_BREAKPOINT;
}

/**
 * Get mobile orientation based on dimensions
 * Only meaningful when both dimensions are < breakpoint
 */
export function getMobileOrientation() {
    if (window.innerHeight > window.innerWidth) {
        return 'portrait'; // input on top, output below
    }
    return 'landscape'; // input on left, output on right
}

/**
 * Get the orientation (horizontal/vertical) for a layout selection
 */
export function layoutOrientation(selection) {
    switch (selection) {
        case 'horizontal':
        case 'h-text':
            return 'horizontal';
        case 'vertical':
        case 'v-text':
        default:
            return 'vertical';
    }
}

/**
 * Set default width on an element (40%)
 */
function setDefaultWidth(target) {
    target.style.width = '40%';
    target.style.minWidth = '40%';
}

/**
 * Set default height on an element (40%)
 */
function setDefaultHeight(target) {
    target.style.height = '40%';
    target.style.minHeight = '40%';
}

/**
 * Clear height styling on an element
 */
function clearHeight(target) {
    target.style.height = '';
    target.style.minHeight = '';
}

/**
 * Clear width styling on an element
 */
function clearWidth(target) {
    target.style.width = '';
    target.style.minWidth = '';
}

/**
 * Reset a splitter to default position
 */
export function resetSplitter(targetContainer, otherContainer, orientation) {
    if (container.dataset.layout === orientation) {
        setDefaultWidth(targetContainer);
        clearHeight(targetContainer);
    } else {
        setDefaultHeight(targetContainer);
        clearWidth(targetContainer);
    }
    targetContainer.classList.remove('maximized', 'minimized');
    otherContainer.classList.remove('maximized', 'minimized');
}

/**
 * Update the layout based on selection (desktop mode)
 */
export function updateLayout(selection, onUpdate) {
    // Reset all containers to initial state
    for (const el of [editorContainer, outputContainer, svgOutputContainer, textOutputContainer]) {
        el.classList.remove('maximized', 'minimized');
        el.style.width = '';
        el.style.minWidth = '';
        el.style.height = '';
        el.style.minHeight = '';
    }

    // Apply layout-specific settings
    switch (selection) {
        case 'horizontal':
            setDefaultHeight(editorContainer);
            svgOutputContainer.classList.add('maximized');
            textOutputContainer.classList.add('minimized');
            break;
        case 'vertical':
            setDefaultWidth(editorContainer);
            svgOutputContainer.classList.add('maximized');
            textOutputContainer.classList.add('minimized');
            break;
        case 'h-text':
            setDefaultHeight(editorContainer);
            setDefaultWidth(svgOutputContainer);
            break;
        case 'v-text':
            setDefaultWidth(editorContainer);
            setDefaultHeight(svgOutputContainer);
            break;
    }

    // Trigger update callback (e.g., for auto-fit)
    if (onUpdate) {
        onUpdate();
    }
}

/**
 * Update the mobile layout (SVG or XML output)
 */
export function updateMobileLayout(mobileLayout, onUpdate) {
    // Reset containers
    for (const el of [editorContainer, outputContainer, svgOutputContainer, textOutputContainer]) {
        el.classList.remove('maximized', 'minimized');
        el.style.width = '';
        el.style.minWidth = '';
        el.style.height = '';
        el.style.minHeight = '';
    }

    // Determine orientation
    const orientation = getMobileOrientation();
    container.dataset.layout = orientation === 'portrait' ? 'horizontal' : 'vertical';
    container.dataset.mobile = 'true';

    // Set editor size based on orientation
    if (orientation === 'portrait') {
        setDefaultHeight(editorContainer);
    } else {
        setDefaultWidth(editorContainer);
    }

    // Show SVG or XML based on mobile layout setting
    if (mobileLayout === 'xml') {
        svgOutputContainer.classList.add('minimized');
        textOutputContainer.classList.add('maximized');
    } else {
        svgOutputContainer.classList.add('maximized');
        textOutputContainer.classList.add('minimized');
    }

    // Trigger update callback
    if (onUpdate) {
        onUpdate();
    }
}

/**
 * Apply the correct layout based on current viewport
 */
export function applyResponsiveLayout(state, onUpdate) {
    if (isMobile()) {
        container.dataset.mobile = 'true';
        updateMobileLayout(state.mobileLayout, onUpdate);
    } else {
        container.dataset.mobile = 'false';
        container.dataset.layout = layoutOrientation(state.layout);
        updateLayout(state.layout, onUpdate);
    }
}

/**
 * Initialize layout functionality
 * @param {Object} state - Application state object
 * @param {Function} onUpdate - Callback when layout changes
 */
export function initLayout(state, onUpdate) {
    // Validate and set initial layout
    if (!VALID_LAYOUTS.includes(state.layout)) {
        state.layout = DEFAULT_LAYOUT;
    }
    if (!VALID_MOBILE_LAYOUTS.includes(state.mobileLayout)) {
        state.mobileLayout = DEFAULT_MOBILE_LAYOUT;
    }

    // Apply initial layout WITHOUT triggering update callback
    // (content isn't loaded yet - caller will trigger update explicitly)
    applyResponsiveLayout(state, null);

    // Set up desktop layout button handlers
    document.querySelectorAll('#layout-popup .popup-button').forEach(el => {
        el.addEventListener('click', (e) => {
            hideAllPopups();

            const id = e.target.id;
            const selection = id.replace('layout-', '');

            if (!VALID_LAYOUTS.includes(selection)) {
                console.error(`Unknown layout: ${selection}`);
                return;
            }

            state.layout = selection;
            saveState(state);

            container.dataset.layout = layoutOrientation(selection);
            updateLayout(selection, onUpdate);
        });
    });

    // Set up mobile layout button handlers
    document.querySelectorAll('#mobile-layout-popup .popup-button').forEach(el => {
        el.addEventListener('click', (e) => {
            hideAllPopups();

            const id = e.target.id;
            const selection = id.replace('mobile-layout-', '');

            if (!VALID_MOBILE_LAYOUTS.includes(selection)) {
                console.error(`Unknown mobile layout: ${selection}`);
                return;
            }

            state.mobileLayout = selection;
            saveState(state);

            updateMobileLayout(selection, onUpdate);
        });
    });

    // Listen for window resize to switch between mobile/desktop layouts
    let resizeTimeout;
    window.addEventListener('resize', () => {
        // Debounce resize events
        clearTimeout(resizeTimeout);
        resizeTimeout = setTimeout(() => {
            applyResponsiveLayout(state, onUpdate);
        }, 100);
    });
}
