// svgdx editor - main entry point
// Initializes all modules and wires them together

import { loadState, saveState, getTabContent, setTabContent } from './modules/storage.js';
import { createCodeMirror5Editor } from './modules/editor-adapter.js';
import { transform, rateLimited, isReady } from './modules/transform.js';
import { initTabs, saveCurrentTabContent } from './modules/tabs.js';
import { initLayout } from './modules/layout.js';
import { initViewport, getCurrentViewBox } from './modules/viewport.js';
import { initSplitters } from './modules/splitter.js';
import { initStatusbar, setStatus } from './modules/statusbar.js';
import { initClipboard } from './modules/clipboard.js';
import { initToolbar } from './modules/toolbar.js';
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
    errorOutput.style.display = 'none';
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
        errorOutput.style.display = '';
        setTimeout(update, 100);
        return;
    }

    try {
        const input = editor.getValue();

        // Save to storage
        setTabContent(state, state.activeTab, input);

        // Transform with metadata for display
        const result = await transform(input, true);

        if (result.ok) {
            // Save current viewBox before updating
            const oldSvg = svgOutputContainer.querySelector('svg');
            if (oldSvg) {
                lastViewbox = oldSvg.getAttribute('viewBox');
            }

            updateSvgOutput(result.svg);

            // Get version without metadata for text output
            const textResult = await transform(input, false);
            if (textResult.ok) {
                updateTextOutput(textResult.svg);
            } else {
                setStatus(`Error retrieving SVG: ${textResult.error}`, true);
            }

            // Handle warnings (for future use)
            if (result.warnings && result.warnings.length > 0) {
                console.log('Transform warnings:', result.warnings);
            }
        } else {
            outputContainer.classList.add('error');
            editorContainer.classList.add('error');
            errorOutput.innerText = result.error;
            errorOutput.style.display = '';
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
    initTabs(state, editor, () => update());
    initLayout(state, () => update());
    initViewport(state, () => update());
    initSplitters();
    initStatusbar(editor);
    initClipboard(editor);

    // Load initial content
    const initialContent = getTabContent(state, state.activeTab);
    editor.setValue(initialContent);

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
