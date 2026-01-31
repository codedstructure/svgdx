// Storage module - handles localStorage with versioned JSON blob
// Includes migration from legacy storage format

import { STORAGE_VERSION, DEFAULT_CONTENT, VALID_LAYOUTS, DEFAULT_LAYOUT, VALID_MOBILE_LAYOUTS, DEFAULT_MOBILE_LAYOUT, DEFAULT_SLIDER_VALUE, DEFAULT_SLIDER_MIN, DEFAULT_SLIDER_MAX, DEFAULT_SLIDER_RANGE, DEFAULT_SLIDER_STEP } from './config.js';

const STORAGE_KEY = 'svgdx-state';

// Legacy keys that will be migrated and then removed
const LEGACY_KEYS = {
    ACTIVE_TAB: 'svgdx-active-tab',
    AUTO_VIEWBOX: 'svgdx-auto-viewbox',
    LAYOUT: 'svgdx-layout',
    CONTENT_PREFIX: 'svgdx-editor-value-'
};

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
 * Check if legacy storage keys exist
 */
function hasLegacyStorage() {
    // Check for any of the legacy keys
    if (localStorage.getItem(LEGACY_KEYS.ACTIVE_TAB) !== null) return true;
    if (localStorage.getItem(LEGACY_KEYS.AUTO_VIEWBOX) !== null) return true;
    if (localStorage.getItem(LEGACY_KEYS.LAYOUT) !== null) return true;

    // Check for any tab content keys
    for (const key of ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']) {
        if (localStorage.getItem(LEGACY_KEYS.CONTENT_PREFIX + key) !== null) {
            return true;
        }
    }
    return false;
}

/**
 * Migrate from legacy storage format to new JSON blob format
 * Only called if legacy keys exist, and only deletes legacy keys after successful migration
 */
function migrateFromLegacyStorage() {
    const state = createDefaultState();

    // Migrate active tab
    const activeTab = localStorage.getItem(LEGACY_KEYS.ACTIVE_TAB);
    if (activeTab !== null) {
        state.activeTab = activeTab;
    }

    // Migrate auto viewbox setting
    const autoViewbox = localStorage.getItem(LEGACY_KEYS.AUTO_VIEWBOX);
    if (autoViewbox !== null) {
        state.autoViewbox = autoViewbox === 'true';
    }

    // Migrate layout setting
    const layout = localStorage.getItem(LEGACY_KEYS.LAYOUT);
    if (layout !== null && VALID_LAYOUTS.includes(layout)) {
        state.layout = layout;
    }

    // Migrate tab contents to new object structure
    for (const tabNum of ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']) {
        const content = localStorage.getItem(LEGACY_KEYS.CONTENT_PREFIX + tabNum);
        if (content !== null) {
            state.tabs[tabNum] = {
                input: content,
                sliderValue: DEFAULT_SLIDER_VALUE,
                sliderMin: DEFAULT_SLIDER_MIN,
                sliderMax: DEFAULT_SLIDER_MAX
            };
        }
    }

    // Save the migrated state
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state));

        // Only delete legacy keys after successful save
        localStorage.removeItem(LEGACY_KEYS.ACTIVE_TAB);
        localStorage.removeItem(LEGACY_KEYS.AUTO_VIEWBOX);
        localStorage.removeItem(LEGACY_KEYS.LAYOUT);
        for (const tabNum of ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']) {
            localStorage.removeItem(LEGACY_KEYS.CONTENT_PREFIX + tabNum);
        }

        console.log('svgdx: migrated settings from legacy localStorage format');
        return state;
    } catch (e) {
        console.error('svgdx: failed to migrate legacy storage', e);
        // Return the migrated state anyway, just don't delete the legacy keys
        return state;
    }
}

/**
 * Load state from localStorage
 * Handles migration from legacy format if needed
 */
export function loadState() {
    // Check for legacy storage first
    if (hasLegacyStorage()) {
        return migrateFromLegacyStorage();
    }

    // Try to load the new format
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
            sliderMin: DEFAULT_SLIDER_MIN,
            sliderMax: DEFAULT_SLIDER_MAX,
            sliderRange: DEFAULT_SLIDER_RANGE,
            sliderStep: DEFAULT_SLIDER_STEP
        };
    }
    return state.tabs[tabNum];
}

/**
 * Get input content for a specific tab
 */
export function getTabInput(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        return tab.input || DEFAULT_CONTENT;
    }
    return DEFAULT_CONTENT;
}

/**
 * Set input content for a specific tab and save state
 */
export function setTabInput(state, tabNum, input) {
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
        const min = tab.sliderMin ?? DEFAULT_SLIDER_MIN;
        const max = tab.sliderMax ?? DEFAULT_SLIDER_MAX;
        const value = tab.sliderValue ?? DEFAULT_SLIDER_VALUE;
        // Clamp to current min/max bounds
        return Math.max(min, Math.min(max, value));
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

/**
 * Get slider min for a specific tab
 */
export function getTabSliderMin(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        return tab.sliderMin ?? DEFAULT_SLIDER_MIN;
    }
    return DEFAULT_SLIDER_MIN;
}

/**
 * Get slider max for a specific tab
 */
export function getTabSliderMax(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        return tab.sliderMax ?? DEFAULT_SLIDER_MAX;
    }
    return DEFAULT_SLIDER_MAX;
}

/**
 * Get slider range preset for a specific tab
 */
export function getTabSliderRange(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        return tab.sliderRange ?? DEFAULT_SLIDER_RANGE;
    }
    return DEFAULT_SLIDER_RANGE;
}

/**
 * Set slider range preset for a specific tab and save state
 */
export function setTabSliderRange(state, tabNum, range) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].sliderRange = range;
    saveState(state);
}

/**
 * Get slider step for a specific tab
 */
export function getTabSliderStep(state, tabNum) {
    const tab = state.tabs[tabNum];
    if (tab && typeof tab === 'object') {
        return tab.sliderStep ?? DEFAULT_SLIDER_STEP;
    }
    return DEFAULT_SLIDER_STEP;
}

/**
 * Set slider step for a specific tab and save state
 */
export function setTabSliderStep(state, tabNum, step) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].sliderStep = step;
    saveState(state);
}

/**
 * Set slider min for a specific tab and save state
 */
export function setTabSliderMin(state, tabNum, min) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].sliderMin = min;
    saveState(state);
}

/**
 * Set slider max for a specific tab and save state
 */
export function setTabSliderMax(state, tabNum, max) {
    ensureTab(state, tabNum);
    state.tabs[tabNum].sliderMax = max;
    saveState(state);
}
