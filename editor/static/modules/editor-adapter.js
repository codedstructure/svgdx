// Editor adapter - abstracts the text editor implementation
// Currently wraps CodeMirror 5, but designed to allow swapping to other editors

/* global CodeMirror */

/**
 * Editor interface definition (for documentation):
 * {
 *   getValue(): string                    - Get current editor content
 *   setValue(value: string): void         - Set editor content
 *   onChange(callback: () => void): void  - Register change callback
 *   setCursorLine(lineNum: number): void  - Move cursor to line (1-based)
 *   highlightLine(lineNum: number): void  - Highlight a line (1-based)
 *   clearHighlight(): void                - Remove all line highlights
 *   focus(): void                         - Focus the editor
 *   refresh(): void                       - Refresh/redraw the editor
 *   getScrollTop(): number                - Get scroll position
 *   setScrollTop(pos: number): void       - Set scroll position
 * }
 */

/**
 * Create a CodeMirror 5 editor instance wrapped with our adapter interface
 * @param {HTMLElement} element - DOM element to attach editor to
 * @param {Object} options - Editor options
 * @param {boolean} options.readOnly - Whether editor is read-only
 * @returns {Object} Editor adapter interface
 */
export function createCodeMirror5Editor(element, options = {}) {
    const config = {
        mode: 'xml',
        lineNumbers: true,
        autoRefresh: true,
        foldGutter: true,
        lineWrapping: true,
        gutters: ['CodeMirror-linenumbers', 'CodeMirror-foldgutter'],
        ...options
    };

    if (!options.readOnly) {
        config.autofocus = true;
    }

    const cm = CodeMirror(element, config);

    // Track the currently highlighted line for efficient clearing
    let highlightedLine = null;

    return {
        /**
         * Get current editor content
         */
        getValue() {
            return cm.getValue();
        },

        /**
         * Set editor content
         */
        setValue(value) {
            cm.setValue(value);
        },

        /**
         * Register a callback for content changes
         */
        onChange(callback) {
            cm.on('change', callback);
        },

        /**
         * Move cursor to specified line (1-based line numbers)
         */
        setCursorLine(lineNum) {
            cm.setCursor(lineNum - 1, 0);
        },

        /**
         * Highlight a line (1-based line numbers)
         */
        highlightLine(lineNum) {
            // Clear previous highlight first
            this.clearHighlight();
            highlightedLine = lineNum - 1;
            cm.addLineClass(highlightedLine, 'background', 'hover-line');
        },

        /**
         * Clear all line highlights
         */
        clearHighlight() {
            if (highlightedLine !== null) {
                cm.removeLineClass(highlightedLine, 'background', 'hover-line');
                highlightedLine = null;
            }
        },

        /**
         * Focus the editor
         */
        focus() {
            cm.focus();
        },

        /**
         * Refresh/redraw the editor
         */
        refresh() {
            cm.refresh();
        },

        /**
         * Get vertical scroll position
         */
        getScrollTop() {
            return cm.getScrollInfo().top;
        },

        /**
         * Set vertical scroll position
         */
        setScrollTop(pos) {
            cm.scrollTo(null, pos);
        },

        /**
         * Get number of lines in the document
         */
        lineCount() {
            return cm.lineCount();
        }
    };
}
