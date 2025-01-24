// svgdx editor script

// Features:
// - CodeMirror editor configured for XML
// - Continuous save / load editor content to/from localstorage
// - Continuously sends to /transform endpoint for conversion to SVG
// - Valid SVG is displayed in #svg-output container; the only modification is to make it fill the container
// - Zoom and pan SVG with mouse wheel / drag
// - Split between edit and output panes
// TODO:
// - Highlight lines with errors
// - Ability to load examples
// - Ability to select SVG elements and get info about them (in status bar?)
// - Editor shortcuts for folding etc

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

const container = document.querySelector('.container');
const editorContainer = document.querySelector('#editor-container');
const svg_container = document.querySelector('#svg-output');
const error_output = document.querySelector('#error-output');
const statusbar = document.querySelector('#statusbar');

function resetLayout() {
    if (container.dataset.layout === "vertical") {
        editorContainer.style.minWidth = "40%";
        editorContainer.style.width = "40%";
        editorContainer.style.minHeight = "";
        editorContainer.style.height = "";
    } else {
        editorContainer.style.minWidth = "";
        editorContainer.style.width = "";
        editorContainer.style.minHeight = "40%";
        editorContainer.style.height = "40%";
    }
}

const DEFAULT_CONTENT = `<svg>
  <!-- Example svgdx document -->
  <rect id="in" wh="20 10" text="input" class="d-softshadow d-fill-azure"/>
  <!-- Try changing the '|h 10' below to '|v 30' or '|V 5' -->
  <rect id="proc" xy="^|h 10" wh="^" text="process" class="d-softshadow d-fill-silver"/>
  <rect id="out" xy="^|h 10" wh="^" text="output" class="d-softshadow d-fill-skyblue"/>

  <line start="#in" end="#proc" class="d-arrow"/>
  <line start="#proc" end="#out" class="d-arrow"/>
</svg>`;

const editor = CodeMirror(document.getElementById('editor'), {
    mode: 'xml',
    lineNumbers: true,
    autoRefresh: true,
    autofocus: true,
    foldGutter: true,
    lineWrapping: true,
    gutters: ['CodeMirror-linenumbers', 'CodeMirror-foldgutter']
});

const textViewer = CodeMirror(document.getElementById('text-output'), {
    mode: 'xml',
    readOnly: true,
    lineNumbers: true,
    foldGutter: true,
    lineWrapping: true,
    gutters: ['CodeMirror-linenumbers', 'CodeMirror-foldgutter']
});

/** Editor updates */
(function () {
    // used to preserve viewbox when updating SVG and Auto Fit is disabled,
    // keeping a changing SVG 'fixed' on screen.
    let last_viewbox = null;

    function update_response(svgData) {
        if (document.getElementById('text-output').style.display !== "none") {
            // Updating the codemirror editor while hidden is ineffective;
            // we set if visible or when it becomes visible.
            textViewer.setValue(svgData);
        }
        svg_container.innerHTML = svgData;
        const svg = svg_container.querySelector('svg');
        if (svg === null) {
            throw new Error("No SVG returned");
        }
        // tweak the SVG to make it fill the container
        // save first so we can restore during save operations
        svg.dataset.origWidth = svg.width.baseVal.value;
        svg.dataset.origHeight = svg.height.baseVal.value;
        svg.dataset.origViewbox = svg.getAttribute('viewBox');
        svg.width.baseVal.valueAsString = '100%';
        svg.height.baseVal.valueAsString = '100%';
        if (document.getElementById('auto-viewbox').dataset.checked !== "true" && last_viewbox) {
            svg.setAttribute('viewBox', last_viewbox);
        }

        document.getElementById('editor').style.backgroundColor = "white";
        error_output.innerText = "";
        error_output.style.display = "none";

        // TODO: return error line numbers info in response to highlight
        // for (let i = 0; i < editor.lineCount(); i++) {
        //     editor.removeLineClass(i, "background", "error-line");
        // }
        // for (const lineNumber of linesWithErrors) {
        //     editor.addLineClass(lineNumber, "background", "error-line");
        // }
    }

    function want_metadata() {
        // We want metadata if the SVG is showing rather than the text output
        return document.getElementById('svg-output').style.display !== "none";
    }

    async function svgdx_transform_server(svgdx_input, add_metadata) {
        try {
            statusbar.style.opacity = "0.3";
            let md_param = add_metadata ? "true" : "false";
            const response = await fetch(`api/transform?add_metadata=${md_param}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'text/xml'
                },
                body: svgdx_input
            });
            statusbar.style.opacity = null;
            statusbar.style.color = null;
            return [response.ok, await response.text()]
        } catch (e) {
            statusbar.style.color = "darkred";
            statusbar.innerText = `svgdx editor - error: ${e.message}`;
            console.error('Error sending data to /transform', e);
            return [false, ""];
        }
    }

    function svgdx_transform_local(svgdx_input, add_metadata) {
        let result, ok;
        try {
            if (!window.hasOwnProperty('svgdx_transform')) {
                result = "loading svgdx...";
                ok = false;
                setTimeout(update, 100);
            } else {
                result = svgdx_transform(svgdx_input, add_metadata);
                ok = true;
            }
        } catch (e) {
            result = e.toString();
            ok = false;
        }
        return Promise.resolve([ok, result]);
    }

    function rateLimited(target) {
        // WASM should be able to handle frequent updates, server maybe not
        // but could be localhost, so don't want to be too slow
        let MAX_CALL_RATE = window.svgdx_use_server ? 250 : 75;
        // if a call hangs for some reason, don't block the next call forever
        let IN_PROGRESS_TIMEOUT = 5000;
        let lastCallTime = 0;
        let callInProgress = false;
        let pendingCall = false;

        return async function() {
            const now = Date.now();

            // Prevent new requests if already in progress, unless they
            // were a very long time ago.
            if (callInProgress && lastCallTime + IN_PROGRESS_TIMEOUT > now) {
                pendingCall = true;
                return;
            }

            if (now - lastCallTime >= MAX_CALL_RATE) {
                // call target immediately if last call was a while ago
                // to avoid latency on infrequent calls
                lastCallTime = now;
                callInProgress = true;
                await target();
                callInProgress = false;

                if (pendingCall) {
                    // another call came in while running target, schedule it
                    // so eventual state is up-to-date
                    pendingCall = false;
                    rateLimited(target)();
                }
            } else {
                // schedule next call to target after MAX_CALL_RATE since last
                setTimeout(async () => {
                    if (!callInProgress) {
                        lastCallTime = Date.now();
                        callInProgress = true;
                        await target();
                        callInProgress = false;

                        if (pendingCall) {
                            pendingCall = false;
                            rateLimited(target)();
                        }
                    }
                }, MAX_CALL_RATE - (now - lastCallTime));
            }
        };
    }

    async function get_transform(input, add_metadata) {
        if (window.svgdx_use_server) {
            return await svgdx_transform_server(input, add_metadata);
        } else {
            return await svgdx_transform_local(input, add_metadata);
        }
    }

    async function update() {
        // svgdx-bootstrap.js sets svgdx_use_server as appropriate
        // to toggle between local (WASM) and server-side transform
        if (!window.hasOwnProperty('svgdx_use_server')) {
            error_output.innerText = "loading svgdx...";
            error_output.style.display = "";
            setTimeout(update, 100);
            return;
        }

        try {
            const svgdx_input = editor.getValue();
            // save editor content to localStorage
            localStorage.setItem(`svgdx-editor-value-${activeTab()}`, svgdx_input);

            let result = await get_transform(svgdx_input, want_metadata());

            const responseOk = result[0];
            const responseText = result[1];

            if (responseOk) {
                const oldSvg = svg_container.querySelector('svg');
                if (oldSvg) {
                    last_viewbox = oldSvg.getAttribute('viewBox');
                }

                update_response(responseText);
            } else {
                document.getElementById('editor').style.backgroundColor = 'red';
                error_output.innerText = responseText;
                error_output.style.display = "";
                statusbar.innerText = "svgdx editor";
            }
        } catch (e) {
            statusbar.style.color = "darkred";
            statusbar.innerText = `svgdx editor - error: ${e.message}`;
            console.error('Error sending data to /transform', e);
        }
    }

    // restore from localstorage on load
    const savedValue = localStorage.getItem(`svgdx-editor-value-${activeTab()}`);
    // Pre-tab implementation just used `svgdx-editor-value`
    //const savedValue = localStorage.getItem(`svgdx-editor-value`);
    if (savedValue) {
        editor.setValue(savedValue);
        update();
    } else {
        editor.setValue(DEFAULT_CONTENT);
        update();
    }

    editor.on('change', rateLimited(update));

    function activeTab() {
        let stored = localStorage.getItem("svgdx-active-tab") || "1";
        let active = document.querySelector('#tabs button[data-checked="true"]');
        if (active) {
            let tabNum = active.dataset.tabNum;
            if (stored != tabNum) {
                localStorage.setItem("svgdx-active-tab", tabNum);
            }
            return tabNum;
        }
        const selected = document.querySelector(`#tabs button[data-tab-num="${stored}"]`);
        if (selected) {
            selected.dataset.checked = "true";
        } else {
            console.log("Oops: svgdx-active-tab doesn't refer to a valid button");
            localStorage.setItem("svgdx-active-tab", "1");
            return activeTab();
        }
        return stored;
    }

    document.querySelectorAll('#tabs button').forEach((button) => {
        button.addEventListener('click', () => {
            document.querySelectorAll('#tabs button').forEach((clearTab) => {
                clearTab.dataset.checked = "false";
            });
            button.dataset.checked = "true";
            localStorage.setItem("svgdx-active-tab", button.dataset.tabNum);
            const loadValue = localStorage.getItem(`svgdx-editor-value-${activeTab()}`) || DEFAULT_CONTENT;
            editor.setValue(loadValue);
            update();
        });
    });

    const resetButton = document.getElementById('reset-view');
    resetButton.addEventListener('click', () => {
        const svg = svg_container.querySelector('svg');
        svg.setAttribute('viewBox', svg.dataset.origViewbox);
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

    function pad2(n) {
        return String(n).padStart(2, '0');
    }

    function getTimestamp() {
        const date = new Date();
        // format date as YYYY-MM-DD-HHMMSS
        return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}-${pad2(date.getHours())}${pad2(date.getMinutes())}${pad2(date.getSeconds())}`;
    }

    // save input button
    document.getElementById('save-input').addEventListener('click', () => {
        // trigger download
        const blob = new Blob([editor.getValue()], { type: 'application/xml' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `svgdx-editor-${getTimestamp()}.xml`;
        a.click();
        URL.revokeObjectURL(url);
    });

    // save output button
    document.getElementById('save-output').addEventListener('click', async () => {
        // download svg as file
        // start by getting a fresh output without metadata
        const svgdx_input = editor.getValue();
        let [ok, svg_output] = await get_transform(svgdx_input, false);
        if (!ok) {
            // update status bar with error message
            statusbar.style.color = "darkred";
            statusbar.innerText = `Error saving SVG output ${svg_output}`;
            return;
        }
        // trigger download
        const blob = new Blob([svg_output], { type: 'image/svg+xml' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `svgdx-output-${getTimestamp()}.svg`;
        a.click();
        URL.revokeObjectURL(url);
    });

    // copy SVG output
    document.getElementById('copy-output').addEventListener('click', async () => {
        // start by getting a fresh output without metadata
        const svgdx_input = editor.getValue();
        let [ok, svg_output] = await get_transform(svgdx_input, false);
        if (!ok) {
            // update status bar with error message
            statusbar.style.color = "darkred";
            statusbar.innerText = `Error retrieving SVG: ${svg_output}`;
            return;
        }
        // copy to clipboard
        try {
            await navigator.clipboard.writeText(svg_output);
        } catch (e) {
            console.error('Error copying SVG to clipboard', e);
            statusbar.style.color = "darkred";
            statusbar.innerText = "Error copying SVG to clipboard";
            return;
        }
        statusbar.style.color = null;
        statusbar.innerText = "SVG output copied to clipboard";
    });

    // copy PNG buttons
    document.querySelectorAll('#copy-popup .popup-button').forEach(
        el => el.addEventListener('click', async (e) => {
            let id = e.target.id;
            const res = id === "copy-png-big" ? 2048 : (id === "copy-png-medium" ? 1024 : 512);
            try {
                navigator.clipboard.write([
                    new ClipboardItem({
                        // Note for Safari on MacOS requires clipboard actions to happen in
                        // an event handler triggered by a user action; having `await` in here
                        // seems to defeat that, so resolve things directly here.
                        ["image/png"]: Promise.resolve(generatePng(res)),
                    }),
                ]);
                console.log(`PNG image copied to clipboard (${res}px)`);
                // Hide the buttons again after copying. This is quite hacky (including
                // the timeout values), due to pure-CSS popup not having a way to close.
                // We make all the inner elements invisible, which will (should!) cause
                // the popup to no longer be :hover, at which point it will be hidden,
                // but then we need to remove the display:none to allow it to be used again...
                setTimeout(() => {
                    document.querySelectorAll(".popup-buttons").forEach((e) => {e.style.display = "none";});
                    setTimeout(() => {
                        document.querySelectorAll(".popup-buttons").forEach((e) => {e.style.display = null;});
                    }, 200);
                }, 200);
            } catch (error) {
                console.error("Error copying PNG image to clipboard:", error);
            }
        })
    );

    async function generatePng(maxDim = 2048) {
        // Since we're async, clone the SVG to avoid glitching on resize
        const svg = document.querySelector('#svg-output svg').cloneNode(true);
        // temporarily set width, height, and viewBox to original values
        svg.setAttribute('width', svg.dataset.origWidth);
        svg.setAttribute('height', svg.dataset.origHeight);
        svg.setAttribute('viewBox', svg.dataset.origViewbox);

        // scale to the given resolution in the maximum dimension
        let pxWidth = svg.width.baseVal.value;
        let pxHeight = svg.height.baseVal.value;
        if (pxWidth > pxHeight) {
            pxHeight = (maxDim / pxWidth) * pxHeight;
            pxWidth = maxDim;
        } else {
            pxWidth = (maxDim / pxHeight) * pxWidth;
            pxHeight = maxDim;
        }
        svg.setAttribute('width', pxWidth);
        svg.setAttribute('height', pxHeight);

        const img = new Image();
        img.src = URL.createObjectURL(new Blob([svg.outerHTML], { type: "image/svg+xml" }));
        img.width = pxWidth;
        img.height = pxHeight;

        await new Promise((resolve) => {
            img.onload = resolve;
        });

        const canvas = document.createElement("canvas");
        const context = canvas.getContext("2d");
        canvas.width = img.width;
        canvas.height = img.height;
        context.drawImage(img, 0, 0);

        // Release the object URL now it's in the canvas
        URL.revokeObjectURL(img.src);

        const pngBlob = await new Promise((resolve) => {
            canvas.toBlob((blob) => resolve(blob), "image/png");
        });

        return pngBlob;
    }

    // toggle layout between horizontal and vertical
    const layoutButton = document.getElementById('toggle-layout');
    let layoutButtonChecked = localStorage.getItem('svgdx-layout') || "false";
    layoutButton.dataset.checked = layoutButtonChecked;
    container.dataset.layout = layoutButtonChecked === "true" ? "vertical" : "horizontal";
    resetLayout();
    layoutButton.addEventListener('click', () => {
        layoutButtonChecked = layoutButtonChecked === "true" ? "false" : "true";
        layoutButton.dataset.checked = layoutButtonChecked;
        container.dataset.layout = layoutButtonChecked === "true" ? "vertical" : "horizontal";
        localStorage.setItem('svgdx-layout', layoutButton.dataset.checked);

        // Reset any manual resizing via the splitter
        resetLayout();
        // opportunity for auto-fit to take effect
        update();
    });

    // Toggle Output button: checked => text, unchecked => image
    const toggleOutput = document.getElementById('toggle-output');
    let toggleOutputChecked = localStorage.getItem('svgdx-toggle-output') || "false";
    toggleOutput.dataset.checked = toggleOutputChecked;

    function updateOutputMode() {
        if (toggleOutputChecked === "true") {
            document.getElementById('svg-output').style.display = "none";
            document.getElementById('text-output').style.display = "";
            update();
        } else {
            document.getElementById('svg-output').style.display = "";
            document.getElementById('text-output').style.display = "none";
            update();
        }
    }

    updateOutputMode();

    toggleOutput.addEventListener('click', () => {
        toggleOutputChecked = toggleOutputChecked === "true" ? "false" : "true";
        toggleOutput.dataset.checked = toggleOutputChecked;
        localStorage.setItem('svgdx-toggle-output', toggleOutput.dataset.checked);
        updateOutputMode();
    });
})();

/** Scroll wheel: zoom SVG */
(function () {
    // Trackpads and some mice can create many scroll events in a short space
    // of time. This can make zooming difficult, and is also hard on the CPU
    // due to recalculating the SVG image due to viewBox changes. Limit the
    // number of zoom operations done each second by ignoring new events for
    // 50ms after a change in scale.
    let busy = false;
    const zoomDelayMs = 50;

    svg_container.addEventListener('wheel', (e) => {
        // Prevent default scrolling behavior
        e.preventDefault();

        // We've done this too recently; ignore this event
        if (busy) { return; }

        // zoom multiplier per wheel click
        const ZOOM_SPEED = 0.15;
        const factor = Math.sign(e.deltaY) * ZOOM_SPEED;

        // initial viewBox
        const svg = svg_container.querySelector('svg');
        const x = svg.viewBox.baseVal.x;
        const y = svg.viewBox.baseVal.y;
        const width = svg.viewBox.baseVal.width;
        const height = svg.viewBox.baseVal.height;

        const eventPos = clientToSvg(svg, e.clientX, e.clientY);

        // calculate new viewBox
        const newWidth = width * (1 + factor);
        const newHeight = height * (1 + factor);

        // Limit zoom-in to 1 user-space unit regardless of original size
        if (newWidth < 1 || newHeight < 1) {
            return;
        }
        // Limit zoom-out to 1/10 original size
        const MAX_ZOOM_OUT = 10;
        let original_width = svg.dataset.origWidth ? parseFloat(svg.dataset.origWidth) : null;
        let original_height = svg.dataset.origHeight ? parseFloat(svg.dataset.origHeight) : null;
        if (original_width === null ||
             original_height === null ||
             newWidth > original_width * MAX_ZOOM_OUT ||
             newHeight > original_height * MAX_ZOOM_OUT) {
            return;
        }

        const newX = x - (newWidth - width) * ((eventPos.x - x) / width);
        const newY = y - (newHeight - height) * ((eventPos.y - y) / height);

        svg.setAttribute('viewBox', `${newX} ${newY} ${newWidth} ${newHeight}`);

        // flag that we've just done a zoom operation and should hold off for a moment
        busy = true;
        setTimeout(() => { busy = false; }, zoomDelayMs);
    });
})();

/** mouse-button drag: pan SVG */
(function () {
    let isDragging = false;
    let startX, startY;

    svg_container.addEventListener('mousedown', (e) => {
        // we're only interested in the left mouse button
        if (e.button !== 0) return;

        // set cursor to xy move
        document.body.style.cursor = 'move';
        const svg = svg_container.querySelector('svg');
        if (e.target.closest('#svg-output > svg') === svg) {
            isDragging = true;
            startX = e.clientX;
            startY = e.clientY;
        }
    });

    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;

        const svg = svg_container.querySelector('svg');
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
        const svg = svg_container.querySelector('svg');

        const tooltips = {
            "toggle-layout": "Toggle layout between horizontal and vertical",
            "toggle-output": "View output as text rather than image",
            "auto-viewbox": "When active, auto-resize and center the SVG on update",
            "reset-view": "Resize and center the SVG",
            "save-input": "Download the input",
            "save-output": "Download the SVG",
            "copy-output": "Copy the SVG to clipboard",
            "copy-png": "Copy as PNG to clipboard",
            "help": "Show help links"
        };

        if (e.target.id in tooltips) {
            // show tooltip in status bar
            statusbar.innerText = tooltips[e.target.id];
        } else if (svg !== null && e.target.closest('div > svg') === svg) {
            // highlight source of this element in editor
            for (let i= 0; i < editor.lineCount(); i++) {
                editor.removeLineClass(i, "background", "hover-line");
            }
            let hover_element = e.target;
            if (e.target.tagName === 'tspan') {
                hover_element = e.target.closest('text');
            }
            if (hover_element.dataset.srcLine) {
                const lineNumber = parseInt(hover_element.dataset.srcLine);
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
        } else {
            statusbar.innerText = "svgdx editor";
        }
    });
})();

/** Splitter for resizing editor and output */
(function () {
    let splitter = document.getElementById('splitter');

    let initialClientPos, initialSize;

    splitter.addEventListener('mousedown', function(e) {
        e.preventDefault();
        if (container.dataset.layout === "vertical") {
            initialClientPos = e.clientX;
            initialSize = editorContainer.getBoundingClientRect().width;
        } else {
            initialClientPos = e.clientY;
            initialSize = editorContainer.getBoundingClientRect().height;
        }
        document.addEventListener('mousemove', mousemove);
        document.addEventListener('mouseup', mouseup);
    });

    // double-click to reset split
    splitter.addEventListener('dblclick', function(e) {
        resetLayout();
    });

    function mousemove(e) {
        if (container.dataset.layout === "vertical") {
            const dx = e.clientX - initialClientPos;
            let newWidth = initialSize + dx;

            // Convert min (25em) and max (80%) widths to pixels
            const minPixels = parseFloat(getComputedStyle(editorContainer).fontSize) * 25;
            const maxPixels = window.innerWidth * 0.8;

            // Enforce min and max widths
            newWidth = Math.max(newWidth, minPixels);
            newWidth = Math.min(newWidth, maxPixels);

            // Set both width and min-width to improve cross-browser compatibility
            editorContainer.style.width = newWidth + 'px';
            editorContainer.style.minWidth = newWidth + 'px';
        } else {
            const dy = e.clientY - initialClientPos;
            let newHeight = initialSize + dy;

            // Convert min (20%) and max (80%) height to pixels
            const minPixels = window.innerHeight * 0.2;
            const maxPixels = window.innerHeight * 0.8;

            // Enforce min and max widths
            newHeight = Math.max(newHeight, minPixels);
            newHeight = Math.min(newHeight, maxPixels);

            // Set both width and min-width to improve cross-browser compatibility
            editorContainer.style.height = newHeight + 'px';
            editorContainer.style.minHeight = newHeight + 'px';
        }
    }

    function mouseup() {
        document.removeEventListener('mousemove', mousemove);
        document.removeEventListener('mouseup', mouseup);
    }
})();
