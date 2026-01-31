// svgdx editor - main entry point
// Initializes all modules and wires them together

import { loadState, saveState, getTabInput, setTabInput, getTabSliderValue, setTabSliderValue, getTabSliderMin, getTabSliderMax, getTabSliderRange } from './modules/storage.js';
import { createCodeMirror5Editor } from './modules/editor-adapter.js';
import { transform, rateLimited, isReady } from './modules/transform.js';
import { initTabs, saveCurrentTabInput } from './modules/tabs.js';
import { initLayout } from './modules/layout.js';
import { initViewport, getCurrentViewBox } from './modules/viewport.js';
import { initSplitters } from './modules/splitter.js';
import { initStatusbar, setStatus } from './modules/statusbar.js';
import { initClipboard } from './modules/clipboard.js';
import { initToolbar } from './modules/toolbar.js';
import { initSlider, updateSlider } from './modules/slider.js';
import { preprocessInput } from './modules/preprocess.js';
import {
    editorContainer,
    outputContainer,
    svgOutputContainer,
    textOutputContainer,
    errorOutput
} from './modules/dom.js';

// Application state
let state = null;
let editor = null;
let textViewer = null;
let lastViewbox = null;

/**
 * Strip metadata attributes from SVG for clean text output
 * Operates on a DOM element and removes attributes in-place
 * @param {Element} element - Root element to strip metadata from
 */
function stripMetadata(element) {
    // List of attributes to strip - add more here as needed
    const attributesToStrip = ['data-src-line'];

    // Process this element
    for (const attr of attributesToStrip) {
        element.removeAttribute(attr);
    }

    // Recursively process children
    for (const child of element.children) {
        stripMetadata(child);
    }
}

/**
 * Create a clean SVG string with metadata stripped
 * @param {string} svgData - Raw SVG string with metadata
 * @returns {string} - SVG string with metadata removed
 */
function getCleanSvgText(svgData) {
    const parser = new DOMParser();
    const doc = parser.parseFromString(svgData, 'image/svg+xml');
    const svg = doc.documentElement;

    // Check for parse errors
    const parseError = doc.querySelector('parsererror');
    if (parseError) {
        console.error('Error parsing SVG for metadata stripping');
        return svgData; // Return original if parsing fails
    }

    stripMetadata(svg);

    // Serialize back to string
    const serializer = new XMLSerializer();
    return serializer.serializeToString(svg);
}

/**
 * Update the SVG output display
 */
function updateSvgOutput(svgData) {
    svgOutputContainer.innerHTML = svgData;
    const svg = svgOutputContainer.querySelector('svg');

    if (svg === null) {
        throw new Error('No SVG returned');
    }

    // Save original dimensions for reset and PNG export
    svg.dataset.origWidth = svg.width.baseVal.value;
    svg.dataset.origHeight = svg.height.baseVal.value;
    svg.dataset.origViewbox = svg.getAttribute('viewBox');

    // Make SVG fill container
    svg.width.baseVal.valueAsString = '100%';
    svg.height.baseVal.valueAsString = '100%';

    // Preserve viewBox if auto-fit is disabled
    if (!state.autoViewbox && lastViewbox) {
        svg.setAttribute('viewBox', lastViewbox);
    }

    editorContainer.classList.remove('error');
    errorOutput.innerText = '';
}

/**
 * Update the text output display (raw SVG)
 */
function updateTextOutput(svgData) {
    if (textOutputContainer.style.display === 'none') {
        // Don't update hidden CodeMirror - it's ineffective
        return;
    }

    outputContainer.classList.remove('error');
    const scrollTop = textViewer.getScrollTop();
    textViewer.setValue(svgData);
    textViewer.setScrollTop(scrollTop);
}

/**
 * Main update function - transforms input and updates displays
 */
async function update() {
    // Wait for bootstrap to complete
    if (!isReady()) {
        errorOutput.innerText = 'loading svgdx...';
        setTimeout(update, 100);
        return;
    }

    try {
        const input = editor.getValue();

        // Save to storage
        setTabInput(state, state.activeTab, input);

        // Preprocess input only if slider is active (substitute $VALUE with slider value)
        const sliderRange = getTabSliderRange(state, state.activeTab);
        let processedInput = input;
        if (sliderRange !== 'off') {
            const sliderValue = getTabSliderValue(state, state.activeTab);
            processedInput = preprocessInput(input, sliderValue);
        }

        // Transform with metadata
        const result = await transform(processedInput, true);

        if (result.ok) {
            // Save current viewBox before updating
            const oldSvg = svgOutputContainer.querySelector('svg');
            if (oldSvg) {
                lastViewbox = oldSvg.getAttribute('viewBox');
            }

            // Display SVG with metadata (for source line highlighting etc)
            updateSvgOutput(result.svg);

            // Strip metadata for clean text output
            const cleanSvg = getCleanSvgText(result.svg);
            updateTextOutput(cleanSvg);

            // Handle warnings (for future use)
            if (result.warnings && result.warnings.length > 0) {
                console.log('Transform warnings:', result.warnings);
            }
        } else {
            outputContainer.classList.add('error');
            editorContainer.classList.add('error');
            errorOutput.innerText = result.error;
            setStatus('svgdx editor');
        }
    } catch (e) {
        setStatus(`svgdx editor - error: ${e.message}`, true);
        console.error('Error during transform', e);
    }
}

/**
 * Initialize the application
 */
function init() {
    // Load state from storage (handles migration from legacy format)
    state = loadState();

    // Create main editor
    editor = createCodeMirror5Editor(document.getElementById('editor'), {
        readOnly: false
    });

    // Create read-only text viewer for SVG output
    textViewer = createCodeMirror5Editor(document.getElementById('text-output'), {
        readOnly: true
    });

    // Create rate-limited update function
    const rateLimitedUpdate = rateLimited(update, window.svgdx_use_server);

    // Initialize all modules
    initToolbar();
    initTabs(state, editor, (tabNum) => {
        updateSlider(state, tabNum);
        update();
    });
    initLayout(state, () => update());
    initViewport(state, () => update());
    initSplitters();
    initStatusbar(editor);
    initClipboard(editor, textViewer);
    initSlider(state, rateLimitedUpdate);

    // Load initial content
    const initialContent = getTabInput(state, state.activeTab);
    editor.setValue(initialContent);

    // Initialize slider for current tab
    updateSlider(state, state.activeTab);

    // Set up change handler
    editor.onChange(rateLimitedUpdate);

    // Initial update
    update();
}

// Wait for svgdx bootstrap to complete before initializing
if (isReady()) {
    init();
} else {
    window.addEventListener('svgdx-ready', init);
}
