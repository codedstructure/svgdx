// Preprocess module - handles input transformations before sending to svgdx

/**
 * Preprocess input document before transformation
 * - Replaces unescaped $VALUE and ${VALUE} with the slider value
 * - Converts escaped \$VALUE and \${VALUE} to literal $VALUE and ${VALUE}
 *
 * @param {string} input - Raw input document
 * @param {number} sliderValue - Current slider value to substitute
 * @returns {string} - Preprocessed input
 */
export function preprocessInput(input, sliderValue) {
    // First, replace unescaped $VALUE and ${VALUE} with the slider value
    // Use negative lookbehind to avoid matching escaped versions
    // Match ${VALUE} first (more specific), then $VALUE

    // Replace ${VALUE} (not preceded by \)
    let result = input.replace(/(?<!\\)\$\{VALUE\}/g, String(sliderValue));

    // Replace $VALUE (not preceded by \)
    result = result.replace(/(?<!\\)\$VALUE\b/g, String(sliderValue));

    // Now convert escaped versions to their literal forms
    // \${VALUE} -> ${VALUE}
    result = result.replace(/\\\$\{VALUE\}/g, '${VALUE}');

    // \$VALUE -> $VALUE
    result = result.replace(/\\\$VALUE\b/g, '$VALUE');

    return result;
}
