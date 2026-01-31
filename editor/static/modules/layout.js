// Layout module - handles editor layout modes

import { VALID_LAYOUTS, DEFAULT_LAYOUT } from './config.js';
import { container, editorContainer, outputContainer, svgOutputContainer, textOutputContainer } from './dom.js';
import { saveState } from './storage.js';

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
 * Update the layout based on selection
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
 * Initialize layout functionality
 * @param {Object} state - Application state object
 * @param {Function} onUpdate - Callback when layout changes
 */
export function initLayout(state, onUpdate) {
    // Validate and set initial layout
    if (!VALID_LAYOUTS.includes(state.layout)) {
        state.layout = DEFAULT_LAYOUT;
    }

    container.dataset.layout = layoutOrientation(state.layout);
    updateLayout(state.layout, onUpdate);

    // Set up layout button handlers
    document.querySelectorAll('#layout-popup .popup-button').forEach(el => {
        el.addEventListener('click', (e) => {
            hidePopup();

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
}

/**
 * Hide popup menus after an action
 * This is a workaround for pure-CSS popups not having a close mechanism
 */
export function hidePopup() {
    setTimeout(() => {
        // We make all the inner elements invisible, which will (should!) cause
        // the popup to no longer be :hover, at which point it will be hidden...
        document.querySelectorAll('.popup-buttons').forEach(e => {
            e.style.display = 'none';
        });
        // but then we need to remove the display:none to allow it to be used again
        setTimeout(() => {
            document.querySelectorAll('.popup-buttons').forEach(e => {
                e.style.display = null;
            });
        }, 200);
    }, 200);
}
