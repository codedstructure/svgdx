function stripMetadata(element) {
    const attributesToStrip = ['data-src-line'];

    for (const attr of attributesToStrip) {
        element.removeAttribute(attr);
    }

    for (const child of element.children) {
        stripMetadata(child);
    }
}

function normalizeWhitespace(text) {
    return text.replace(/\s+/g, ' ').trim();
}

export function describeSvgParseError(parseError) {
    const detail = parseError.querySelector('div')?.textContent;
    if (detail) {
        return normalizeWhitespace(detail);
    }

    const fallback = normalizeWhitespace(parseError.textContent || '');
    // The strings below are present in Edge and Safari error fragments;
    // Firefox contains more detailed info that is reported as-is.
    return fallback
        .replace(/^This page contains the following errors:\s*/i, '')
        .replace(/\s*Below is a rendering of the page up to the first error\.?$/i, '')
        .trim() || 'Error parsing SVG output';
}

export function getCleanSvgText(svgData, options = {}) {
    const {
        strict = false,
        logContext = 'metadata stripping'
    } = options;

    const parser = new DOMParser();
    const doc = parser.parseFromString(svgData, 'image/svg+xml');

    // if DOMParser.parseFromString() fails, a <parsererror> element is inserted into
    // the generated document.
    const parseError = doc.querySelector('parsererror');
    if (parseError) {
        const detail = describeSvgParseError(parseError);
        console.error(`Error parsing SVG for ${logContext}: ${detail}`);

        if (strict) {
            // strict mode: used for preparing SVG for copy/download
            throw new Error(`Invalid SVG output: ${detail}`);
        }

        // non-strict mode: return the original SVG data.
        // Used for displaying SVG output text.
        return svgData;
    }

    const svg = doc.documentElement;
    stripMetadata(svg);

    const serializer = new XMLSerializer();
    return serializer.serializeToString(svg);
}
