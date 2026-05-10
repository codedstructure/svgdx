// Storage module - handles localStorage with versioned JSON blob
// Includes migration from legacy storage format

import { STORAGE_VERSION, DEFAULT_CONTENT, VALID_LAYOUTS, DEFAULT_LAYOUT, VALID_MOBILE_LAYOUTS, DEFAULT_MOBILE_LAYOUT, DEFAULT_SLIDER_VALUE, SLIDER_MIN, SLIDER_MAX } from './config.js';

const STORAGE_KEY = 'svgdx-state';

/**
 * Default state structure
 */
function createDefaultState() {
    return {
        version: STORAGE_VERSION,
        activeTab: '1',
        autoViewbox: true,
        layout: DEFAULT_LAYOUT,
        mobileLayout: DEFAULT_MOBILE_LAYOUT,
        tabs: {}
    };
}

/**
 * Load state from localStorage
 */
export function loadState() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === null) {
        return createDefaultState();
    }

    try {
        const state = JSON.parse(stored);
        // Validate version and structure
        if (typeof state !== 'object' || state.version !== STORAGE_VERSION) {
            console.warn('svgdx: invalid storage version, using defaults');
            return createDefaultState();
        }
        // Ensure required fields exist with defaults
        return {
            version: STORAGE_VERSION,
            activeTab: state.activeTab || '1',
            autoViewbox: state.autoViewbox !== false, // default true
            layout: VALID_LAYOUTS.includes(state.layout) ? state.layout : DEFAULT_LAYOUT,
            mobileLayout: VALID_MOBILE_LAYOUTS.includes(state.mobileLayout) ? state.mobileLayout : DEFAULT_MOBILE_LAYOUT,
            tabs: state.tabs || {}
        };
    } catch (e) {
        console.error('svgdx: failed to parse storage', e);
        return createDefaultState();
    }
}

/**
 * Save state to localStorage
 */
export function saveState(state) {
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify({
            ...state,
            version: STORAGE_VERSION
        }));
    } catch (e) {
        console.error('svgdx: failed to save state', e);
    }
}

/**
 * Ensure a tab exists with proper structure
 */
function ensureTab(state, tabNum) {
    if (!state.tabs[tabNum] || typeof state.tabs[tabNum] !== 'object') {
        state.tabs[tabNum] = {
            input: DEFAULT_CONTENT,
            sliderValue: DEFAULT_SLIDER_VALUE,
        };
    }
    return state.tabs[tabNum];
}

/**
 * Get input content for a specific tab
 */
export function getTabContent(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (!tab) {
        // If tab doesn't exist, create it with defaults
        return DEFAULT_CONTENT;
    }
    if (typeof tab === 'object') {
        return tab.input || DEFAULT_CONTENT;
    } else if (typeof tab === 'string') {
        // Handle legacy case where tab content was just a string
        return tab;
    }
    return DEFAULT_CONTENT;
}

/**
 * Set input content for a specific tab and save state
 */
export function setTabContent(state, tabNum, input) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].input = input;
    saveState(state);
}

/**
 * Get slider value for a specific tab (clamped to min/max)
 */
export function getTabSliderValue(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        const value = tab.sliderValue ?? DEFAULT_SLIDER_VALUE;
        // Clamp to min/max bounds
        return Math.max(SLIDER_MIN, Math.min(SLIDER_MAX, value));
    }
    return DEFAULT_SLIDER_VALUE;
}

/**
 * Set slider value for a specific tab and save state
 */
export function setTabSliderValue(state, tabNum, value) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].sliderValue = value;
    saveState(state);
}
