// Slider module - handles $VALUE slider in toolbar

import { getTabSliderValue, setTabSliderValue } from './storage.js';
import { SLIDER_STEP } from './config.js';
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

function setValueLabel(value, step) {
    if (labelElement) {
        labelElement.innerHTML = "$VALUE<br/>&nbsp;" + formatValue(value, step);
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
        const step = parseFloat(sliderElement.step) || SLIDER_STEP;
        const value = parseFloat(sliderElement.value);
        setValueLabel(value, step);

        // Save to storage
        setTabSliderValue(state, state.activeTab, value);

        // Trigger update callback
        if (onChangeCallback) {
            onChangeCallback();
        }
    });
}

/**
 * Update the slider to reflect a specific tab's settings
 * Sets value and display from storage
 * @param {Object} state - Application state object
 * @param {string} tabNum - Tab number to load settings from
 */
export function updateSlider(state, tabNum) {
    if (!sliderElement) {
        return;
    }

    // Get stored value, clamped to current range
    let value = getTabSliderValue(state, tabNum);
    if (value !== null) {
        sliderElement.value = value;
        setValueLabel(value, SLIDER_STEP);
    }
}
