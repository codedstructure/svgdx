// Storage module - handles localStorage with versioned JSON blob
// Includes migration from legacy storage format

import { STORAGE_VERSION, DEFAULT_CONTENT, VALID_LAYOUTS, DEFAULT_LAYOUT, VALID_MOBILE_LAYOUTS, DEFAULT_MOBILE_LAYOUT } from './config.js';

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

    // Migrate tab contents
    for (const tabNum of ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']) {
        const content = localStorage.getItem(LEGACY_KEYS.CONTENT_PREFIX + tabNum);
        if (content !== null) {
            state.tabs[tabNum] = content;
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
 * Get content for a specific tab
 */
export function getTabContent(state, tabNum) {
    return state.tabs[tabNum] || DEFAULT_CONTENT;
}

/**
 * Set content for a specific tab and save state
 */
export function setTabContent(state, tabNum, content) {
    state.tabs[tabNum] = content;
    saveState(state);
}
