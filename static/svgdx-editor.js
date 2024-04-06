// svgdx editor script

// Features:
// - CodeMirror editor configured for XML
// - Continuous save / load editor content to/from localstorage
// - Continuously sends to /transform endpoint for conversion to SVG
// - Valid SVG is displayed in #svg-output container; the only modification is to make it fill the container
// - Zoom and pan SVG with mouse wheel / drag
// - Split between edit and output panes
// TODO:
// - use a WASM build rather than needing external /transform endpoint
// - Highlight lines with errors
// - Ability to load examples
// - Ability to select SVG elements and get info about them (in status bar?)
// - Editor shortcuts for folding etc
// - Save multiple documents in localStorage

/* global CodeMirror */

/** helper function to convert client coordinates to SVG user-space */
function clientToSvg(svg, x, y) {
    // Use SVGPoint and inverse screen coordinate transform matrix
    // as deriving from boundingbox and viewbox ratios is tricky due
    // to potential flexbox scaling/shrinking.
    let pt = svg.createSVGPoint();
    pt.x = x;
    pt.y = y;

    const svgPos = pt.matrixTransform(svg.getScreenCTM().inverse());
    return {x: svgPos.x, y: svgPos.y};
}

/** #svg-output element - used in many other functions */
const container = document.querySelector('#svg-output'); // Assuming that your SVG is inside a container with id="container"

const editor = CodeMirror(document.getElementById('editor'), {
    mode: 'xml',
    lineNumbers: true,
    autoRefresh: true,
    autofocus: true,
    foldGutter: true,
    lineWrapping: true,
    gutters: ['CodeMirror-linenumbers', 'CodeMirror-foldgutter']
});

/** Editor updates */
(function () {
    let last_viewbox = null;
    let original_viewbox = null;
    let original_width = null;
    let original_height = null;

    async function update() {
        try {
            // save editor content to localStorage
            localStorage.setItem('svgdx-editor-value', editor.getValue());

            const response = await fetch('/transform', {
                method: 'POST',
                headers: {
                    'Content-Type': 'text/xml'
                },
                body: editor.getValue()
            });

            if (response.ok) {
                const oldSvg = container.querySelector('svg');
                if (oldSvg) {
                    last_viewbox = oldSvg.getAttribute('viewBox');
                }

                const svgData = await response.text();
                container.innerHTML = svgData;
                const svg = container.querySelector('svg');
                // tweak the SVG to make it fill the container
                // save first so we can restore during save operations
                original_width = svg.width.baseVal.value;
                original_height = svg.height.baseVal.value;
                svg.width.baseVal.valueAsString = '100%';
                svg.height.baseVal.valueAsString = '100%';
                original_viewbox = svg.getAttribute('viewBox');
                if (document.getElementById('auto-viewbox').dataset.checked !== "true" && last_viewbox) {
                    svg.setAttribute('viewBox', last_viewbox);
                }

                document.getElementById('editor').style.backgroundColor = "white";
                document.getElementById('error-output').innerText = "";

                // TODO: return error line numbers info in response to highlight
                // for (let i = 0; i < editor.lineCount(); i++) {
                //     editor.removeLineClass(i, "background", "error-line");
                // }
                // for (const lineNumber of linesWithErrors) {
                //     editor.addLineClass(lineNumber, "background", "error-line");
                // }

            } else {
                const responseText = await response.text();
                document.getElementById('error-output').innerText = responseText;
                document.getElementById('editor').style.backgroundColor = 'red';
            }
        } catch (e) {
            console.error('Error sending data to /transform', e);
        }
    }

    // restore from localstorage on load
    const savedValue = localStorage.getItem('svgdx-editor-value');
    if (savedValue) {
        editor.setValue(savedValue);
        update();
    } else {
        editor.setValue(`<svg>
  <!-- Example svgdx document -->
  <rect id="in" wh="20 10" text="input" />
  <rect id="proc" xy="^:h 10" wh="^" text="process" />
  <rect id="out" xy="^:h 10" wh="^" text="output" />

  <line start="#in" end="#proc" class="d-arrow"/>
  <line start="#proc" end="#out" class="d-arrow"/>
</svg>`);
        update();
    }

    editor.on('change', update);

    const resetButton = document.getElementById('reset-view');
    resetButton.addEventListener('click', () => {
        const svg = container.querySelector('svg');
        svg.setAttribute('viewBox', original_viewbox);
    });

    const autoViewbox = document.getElementById('auto-viewbox');
    let autoViewboxChecked = localStorage.getItem('svgdx-auto-viewbox') || "true";
    autoViewbox.dataset.checked = autoViewboxChecked;

    autoViewbox.addEventListener('click', () => {
        autoViewboxChecked = autoViewboxChecked === "true" ? "false" : "true";
        autoViewbox.dataset.checked = autoViewboxChecked;
        localStorage.setItem('svgdx-auto-viewbox', autoViewbox.dataset.checked);
        update();
    });

    // save input button
    document.getElementById('save-input').addEventListener('click', () => {
        // trigger download
        const blob = new Blob([editor.getValue()], { type: 'application/xml' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'svgdx-editor.svgdx';
        a.click();
        URL.revokeObjectURL(url);
    });

    // save output button
    document.getElementById('save-output').addEventListener('click', () => {
        // download svg as file
        const svg = document.querySelector('#svg-output svg');
        const saved_viewbox = svg.getAttribute('viewBox');
        // temporarily set width, height, and viewBox to original values
        svg.setAttribute('width', original_width);
        svg.setAttribute('height', original_height);
        svg.setAttribute('viewBox', original_viewbox);
        // trigger download
        const blob = new Blob([svg.outerHTML], { type: 'image/svg+xml' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'output.svg';
        a.click();
        URL.revokeObjectURL(url);
        // and back to our 'normal'
        svg.setAttribute('width', '100%');
        svg.setAttribute('height', '100%');
        svg.setAttribute('viewBox', saved_viewbox);
    });
})();

/** Scroll wheel: zoom SVG */
(function () {
    container.addEventListener('wheel', (e) => {
        // Prevent default scrolling behavior
        e.preventDefault();

        // zoom multiplier per wheel click
        const ZOOM_SPEED = 0.15;
        const factor = Math.sign(e.deltaY) * ZOOM_SPEED;

        // initial viewBox
        const svg = container.querySelector('svg');
        const x = svg.viewBox.baseVal.x;
        const y = svg.viewBox.baseVal.y;
        const width = svg.viewBox.baseVal.width;
        const height = svg.viewBox.baseVal.height;

        const eventPos = clientToSvg(svg, e.clientX, e.clientY);

        // calculate new viewBox
        const newWidth = width * (1 + factor);
        const newHeight = height * (1 + factor);
        const newX = x - (newWidth - width) * ((eventPos.x - x) / width);
        const newY = y - (newHeight - height) * ((eventPos.y - y) / height);

        svg.setAttribute('viewBox', `${newX} ${newY} ${newWidth} ${newHeight}`);
    });
})();

/** mouse-button drag: pan SVG */
(function () {
    let isDragging = false;
    let startX, startY;

    container.addEventListener('mousedown', (e) => {
        // we're only interested in the left mouse button
        if (e.button !== 0) return;

        // set cursor to xy move
        document.body.style.cursor = 'move';
        const svg = container.querySelector('svg');
        if (e.target.closest('#svg-output > svg') === svg) {
            isDragging = true;
            startX = e.clientX;
            startY = e.clientY;
        }
    });

    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;

        const svg = container.querySelector('svg');
        // Note stores mouse *client* position rather than SVG position
        // for accuracy, since mouse moves in integer pixel steps, and
        // converts only to calculate the delta for viewBox updates.
        const oldPos = clientToSvg(svg, startX, startY);
        const newPos = clientToSvg(svg, e.clientX, e.clientY);
        const dx = oldPos.x - newPos.x;
        const dy = oldPos.y - newPos.y;

        svg.setAttribute('viewBox', `${svg.viewBox.baseVal.x + dx} ${svg.viewBox.baseVal.y + dy} ${svg.viewBox.baseVal.width} ${svg.viewBox.baseVal.height}`);

        // Update for next mousemove
        startX = e.clientX;
        startY = e.clientY;
    });

    document.addEventListener('mouseup', () => {
        isDragging = false;
        // reset cursor to default
        document.body.style.cursor = 'default';
    });
}());

/** status bar updates */
(function () {
    document.addEventListener('mousemove', (e) => {
        const svg = container.querySelector('svg');
        const statusbar = document.querySelector('#statusbar');

        const tooltips = {
            "auto-viewbox": "When active, auto-resize and center the SVG on update",
            "reset-view": "Resize and center the SVG",
            "save-input": "Download the input",
            "save-output": "Download the SVG"
        };

        if (e.target.id in tooltips) {
            // show tooltip in status bar
            statusbar.innerText = tooltips[e.target.id];
        } else if (e.target.closest('div > svg') === svg) {
            // highlight source of this element in editor
            for (let i= 0; i < editor.lineCount(); i++) {
                editor.removeLineClass(i, "background", "hover-line");
            }
            let hover_element = e.target;
            if (e.target.tagName === 'tspan') {
                hover_element = e.target.closest('text');
            }
            if (hover_element.dataset.sourceLine) {
                const lineNumber = parseInt(hover_element.dataset.sourceLine);
                editor.addLineClass(lineNumber - 1, "background", "hover-line");
            }
            // display mouse position in SVG user-space coordinates
            const svgPos = clientToSvg(svg, e.clientX, e.clientY);
            const pos_text = `${svgPos.x.toFixed(2)}, ${svgPos.y.toFixed(2)}`;
            let status_text = pos_text.padEnd(20, ' ');
            const target_tag = hover_element.tagName;
            if (target_tag !== null) {
                status_text += ` ${target_tag}`;
            }
            const target_id = hover_element.getAttribute('id');
            if (target_id !== null) {
                status_text += ` id="${target_id}"`;
            }
            const target_href = hover_element.getAttribute('href');
            if (target_href !== null) {
                status_text += ` href="${target_href}"`;
            }
            const target_class = hover_element.getAttribute('class');
            if (target_class !== null) {
                status_text += ` class="${target_class}"`;
            }
            statusbar.innerText = status_text;
        }
        else {
            statusbar.innerText = 'svgdx';
        }
    });
})();

/** Splitter for resizing editor and output */
(function () {
    let splitter = document.getElementById('splitter');
    let editorContainer = document.getElementById('editor-container');

    let initialClientX, initialWidth;

    splitter.addEventListener('mousedown', function(e) {
        e.preventDefault();
        initialClientX = e.clientX;
        initialWidth = editorContainer.getBoundingClientRect().width;
        document.addEventListener('mousemove', mousemove);
        document.addEventListener('mouseup', mouseup);
    });

    // double-click to reset split
    splitter.addEventListener('dblclick', function(e) {
        editorContainer.style.width = '';
        editorContainer.style.minWidth = '';
    });

    function mousemove(e) {
        const dx = e.clientX - initialClientX;
        let newWidth = initialWidth + dx;

        // Convert min (25em) and max (80%) widths to pixels
        const minPixels = parseFloat(getComputedStyle(editorContainer).fontSize) * 25;
        const maxPixels = window.innerWidth * 0.8;

        // Enforce min and max widths
        newWidth = Math.max(newWidth, minPixels);
        newWidth = Math.min(newWidth, maxPixels);

        // Set both width and min-width to improve cross-browser compatibility
        editorContainer.style.width = newWidth + 'px';
        editorContainer.style.minWidth = newWidth + 'px';
    }

    function mouseup() {
        document.removeEventListener('mousemove', mousemove);
        document.removeEventListener('mouseup', mouseup);
    }
})();
