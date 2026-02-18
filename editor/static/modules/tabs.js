// Tabs module - handles tab switching and content management

import { DEFAULT_CONTENT } from './config.js';
import { getTabContent, setTabContent, saveState } from './storage.js';

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

    // Set up click handlers for all tab buttons
    tabButtons.forEach(button => {
        button.addEventListener('click', () => {
            const tabNum = button.dataset.tabNum;

            // Update state
            state.activeTab = tabNum;
            saveState(state);

            // Update UI
            updateActiveTabUI(tabNum);

            // Load content for this tab
            const content = getTabContent(state, tabNum);
            editor.setValue(content);

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

/**
 * Save content to the current tab
 */
export function saveCurrentTabContent(state, editor) {
    setTabContent(state, state.activeTab, editor.getValue());
}
