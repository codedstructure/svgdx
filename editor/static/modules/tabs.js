// Tabs module - handles tab switching and content management

import { DEFAULT_CONTENT } from './config.js';
import { getTabContent, saveState } from './storage.js';

const tabDocs = new Map();
const tabCursorPositions = new Map();
const tabScrollPositions = new Map();

function getTabDocument(tabNum, content) {
    if (!tabDocs.has(tabNum)) {
        tabDocs.set(tabNum, new CodeMirror.Doc(content || DEFAULT_CONTENT, 'xml'));
    }
    return tabDocs.get(tabNum);
}

function saveEditorState(editor, tabNum) {
    if (typeof editor.getCursor === 'function') {
        tabCursorPositions.set(tabNum, editor.getCursor());
    }
    if (typeof editor.getScrollTop === 'function') {
        tabScrollPositions.set(tabNum, editor.getScrollTop());
    }
}

function restoreEditorState(editor, tabNum) {
    const cursor = tabCursorPositions.get(tabNum);
    if (cursor && typeof editor.setCursor === 'function') {
        editor.setCursor(cursor.line, cursor.ch);
    } else if (typeof editor.setCursor === 'function') {
        editor.setCursor(0, 0);
    }

    const scrollTop = tabScrollPositions.get(tabNum);
    if (scrollTop !== undefined && typeof editor.setScrollTop === 'function') {
        editor.setScrollTop(scrollTop);
    }

    if (typeof editor.focus === 'function') {
        editor.focus();
    }
}

/**
 * Initialize tabs functionality
 * @param {Object} state - Application state object
 * @param {Object} editor - Editor adapter instance
 * @param {Function} onTabChange - Callback when tab changes
 */
export function initTabs(state, editor, onTabChange) {
    const tabButtons = document.querySelectorAll('#tabs button');

    // Set initial active tab from state
    updateActiveTabUI(state.activeTab);

    const initialTabNum = state.activeTab || '1';
    const initialTabContent = getTabContent(state, initialTabNum);
    editor.swapDoc(getTabDocument(initialTabNum, initialTabContent));
    restoreEditorState(editor, initialTabNum);

    // Set up click handlers for all tab buttons
    tabButtons.forEach(button => {
        button.addEventListener('click', () => {
            const currentTab = state.activeTab;
            const tabNum = button.dataset.tabNum;

            saveEditorState(editor, currentTab);

            // Update state
            state.activeTab = tabNum;
            saveState(state);

            // Update UI
            updateActiveTabUI(tabNum);

            // Load content for this tab while preserving that tab's local undo history.
            const content = getTabContent(state, tabNum);
            editor.swapDoc(getTabDocument(tabNum, content));
            restoreEditorState(editor, tabNum);

            // Notify of tab change
            if (onTabChange) {
                onTabChange(tabNum);
            }
        });
    });
}

/**
 * Update the visual state of tab buttons
 */
function updateActiveTabUI(activeTabNum) {
    document.querySelectorAll('#tabs button').forEach(btn => {
        btn.dataset.checked = btn.dataset.tabNum === activeTabNum ? 'true' : 'false';
    });
}

/**
 * Get the currently active tab number
 */
export function getActiveTab(state) {
    return state.activeTab;
}
