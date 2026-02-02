// Slider module - handles $VALUE slider in toolbar

import { getTabSliderValue, setTabSliderValue, getTabSliderRange, setTabSliderRange, setTabSliderMin, setTabSliderMax, setTabSliderStep } from './storage.js';
import { SLIDER_RANGES, DEFAULT_SLIDER_RANGE } from './config.js';
import { hideAllPopups } from './toolbar.js';

let sliderElement = null;
let labelElement = null;
let currentState = null;
let onChangeCallback = null;

/**
 * Format value for display (handle floating point nicely)
 */
function formatValue(value, step) {
    if (step < 1) {
        // For floating point, show appropriate decimal places
        const decimals = Math.max(0, -Math.floor(Math.log10(step)));
        return Number(value).toFixed(decimals);
    }
    return String(Math.round(value));
}

/**
 * Update slider visibility based on range setting
 */
function updateSliderVisibility(range) {
    const isVisible = range !== 'off';

    if (labelElement) {
        labelElement.classList.toggle('slider-hidden', !isVisible);
    }
    if (sliderElement) {
        sliderElement.classList.toggle('slider-hidden', !isVisible);
    }
}

/**
 * Apply a range preset to the slider
 */
function applyRangePreset(range, tabNum) {
    const preset = SLIDER_RANGES[range];

    if (!preset) {
        // 'off' or invalid - hide slider
        updateSliderVisibility('off');
        setTabSliderRange(currentState, tabNum, 'off');
        return;
    }

    // Update storage with new range settings
    setTabSliderRange(currentState, tabNum, range);
    setTabSliderMin(currentState, tabNum, preset.min);
    setTabSliderMax(currentState, tabNum, preset.max);
    setTabSliderStep(currentState, tabNum, preset.step);

    // Update slider element
    if (sliderElement) {
        sliderElement.min = preset.min;
        sliderElement.max = preset.max;
        sliderElement.step = preset.step;

        // Clamp current value to new range
        let value = parseFloat(sliderElement.value);
        value = Math.max(preset.min, Math.min(preset.max, value));
        sliderElement.value = value;

        // Update display
        if (labelElement) {
            setValueLabel(value, preset.step);
        }

        // Save clamped value
        setTabSliderValue(currentState, tabNum, value);
    }

    updateSliderVisibility(range);

    // Trigger update callback
    if (onChangeCallback) {
        onChangeCallback();
    }
}

function setValueLabel(value, step) {
    if (labelElement) {
        labelElement.innerText = "$VALUE = " + formatValue(value, step);
    }
}

/**
 * Initialize the slider functionality
 * @param {Object} state - Application state object
 * @param {Function} callback - Callback when slider value changes
 */
export function initSlider(state, callback) {
    currentState = state;
    onChangeCallback = callback;
    sliderElement = document.getElementById('value-slider');
    labelElement = document.querySelector('#slider-controls label');

    if (!sliderElement) {
        console.warn('Slider elements not found in DOM');
        return;
    }

    // Handle slider input (live update while dragging)
    sliderElement.addEventListener('input', () => {
        const step = parseFloat(sliderElement.step) || 1;
        const value = parseFloat(sliderElement.value);
        setValueLabel(value, step);

        // Save to storage
        setTabSliderValue(state, state.activeTab, value);

        // Trigger update callback
        if (onChangeCallback) {
            onChangeCallback();
        }
    });

    // Set up range preset buttons
    const rangeButtons = {
        'slider-off': 'off',
        'slider-0-1': '0-1',
        'slider-0-100': '0-100',
        'slider-0-255': '0-255',
        'slider-0-360': '0-360',
        'slider-0-1000': '0-1000'
    };

    for (const [buttonId, range] of Object.entries(rangeButtons)) {
        const button = document.getElementById(buttonId);
        if (button) {
            button.addEventListener('click', () => {
                applyRangePreset(range, state.activeTab);
                hideAllPopups();
            });
        }
    }
}

/**
 * Update the slider to reflect a specific tab's settings
 * Sets min, max, step, value, and display from storage
 * @param {Object} state - Application state object
 * @param {string} tabNum - Tab number to load settings from
 */
export function updateSlider(state, tabNum) {
    if (!sliderElement) {
        return;
    }

    const range = getTabSliderRange(state, tabNum);
    const preset = SLIDER_RANGES[range];

    if (!preset) {
        // 'off' - hide slider
        updateSliderVisibility('off');
        return;
    }

    // Apply preset settings to slider
    sliderElement.min = preset.min;
    sliderElement.max = preset.max;
    sliderElement.step = preset.step;

    // Get stored value, clamped to current range
    let value = getTabSliderValue(state, tabNum);
    value = Math.max(preset.min, Math.min(preset.max, value));

    sliderElement.value = value;
    setValueLabel(value, preset.step);

    updateSliderVisibility(range);
}
