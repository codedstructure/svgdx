// svgdx editor script

// Features:
// - CodeMirror editor configured for XML
// - Continuous save / load editor content to/from localstorage
// - Continuously sends to /transform endpoint for conversion to SVG
// - Valid SVG is displayed in #svg-output container; the only modification is to make it fill the container
// - Zoom and pan SVG with mouse wheel and middle-button drag
// TODO:
// - Highlight lines with errors
// - Status bar with mouse position and zoom level
// - Default content for new editor, or examples to choose from
// - Ability to select SVG elements and get info about them (in status bar?)
// - Editor shortcuts for folding etc
// - Adjustable split between editor and output

/*global CodeMirror*/


(function () {

    let last_viewbox = null;
    let original_viewbox = null;
    let original_width = null;
    let original_height = null;

    async function update() {
        try {
            const response = await fetch('/transform', {
                method: 'POST',
                headers: {
                    'Content-Type': 'text/xml'
                },
                body: editor.getValue()
            });

            if (response.ok) {
                const container = document.querySelector('#svg-output');
                const oldSvg = container.querySelector('svg');
                if (oldSvg) {
                    last_viewbox = oldSvg.getAttribute('viewBox');
                }

                const svgData = await response.text();
                container.innerHTML = svgData;
                const svg = container.querySelector('svg');
                // tweak to the SVG to make it fill the container
                // save first for downloading.
                original_width = svg.width.baseVal.value;
                original_height = svg.height.baseVal.value;
                svg.width.baseVal.valueAsString = '100%';
                svg.height.baseVal.valueAsString = '100%';
                original_viewbox = svg.getAttribute('viewBox');
                if (!document.getElementById('auto-viewbox').checked && last_viewbox) {
                    svg.setAttribute('viewBox', last_viewbox);
                }

                document.getElementById('editor').style.backgroundColor = "white";
                document.getElementById('error-output').innerText = "";
                // save to localStorage
                localStorage.setItem('editorValue', editor.getValue());

                // TODO: return error line numbers info in response to highlight
                // for (let i = 0; i < editor.lineCount(); i++) {
                //     editor.removeLineClass(i, "background", "error-line");
                // }
                // for (const lineNumber of linesWithErrors) {
                //     editor.addLineClass(lineNumber, "background", "error-line");
                // }

            } else {
                let responseText = await response.text();
                document.getElementById('error-output').innerText = responseText;
                document.getElementById('editor').style.backgroundColor = 'red';
            }
        } catch (e) {
            console.error('Error sending data to /transform', e);
        }
    }
    const editor = CodeMirror(document.getElementById('editor'), {
        mode: 'xml',
        lineNumbers: true,
        autoRefresh: true,
        foldGutter: true,
        lineWrapping: true,
        gutters: ['CodeMirror-linenumbers', 'CodeMirror-foldgutter']
    });
    // focus editor window and start cursor at beginning
    editor.focus();
    editor.setCursor({ line: 0, ch: 0 });

    // restore from localstorage on load
    const savedValue = localStorage.getItem('editorValue');
    if (savedValue) {
        editor.setValue(savedValue);
        update();
    }

    editor.on('change', update);

    const resetButton = document.getElementById('reset-view');
    resetButton.addEventListener('click', () => {
        const svg = container.querySelector('svg');
        svg.setAttribute('viewBox', original_viewbox);
    });

    const autoViewbox = document.getElementById('auto-viewbox');
    const savedAutoViewbox = localStorage.getItem('autoViewbox');
    if (savedAutoViewbox) {
        autoViewbox.checked = savedAutoViewbox === 'true';
    }

    autoViewbox.addEventListener('change', () => {
        localStorage.setItem('autoViewbox', autoViewbox.checked);
        update();
    });

    // save button
    const saveButton = document.getElementById('save-svg');
    saveButton.addEventListener('click', () => {
        // download svg as file
        const svg = document.querySelector('#svg-output svg');
        const saved_viewbox = svg.getAttribute('viewBox');
        // temporarily set width and height to actual size and reset viewBox to original
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
        // reset things we tweaked
        svg.setAttribute('width', '100%');
        svg.setAttribute('height', '100%');
        svg.setAttribute('viewBox', saved_viewbox);
    });
})();

const container = document.querySelector('#svg-output'); // Assuming that your SVG is inside a container with id="container"

/// Scroll wheel: zoom SVG
container.addEventListener('wheel', (e) => {
    // Prevent default scrolling behavior
    e.preventDefault();

    // zoom 0.1 for each wheel tick; wheel up for zoom in.
    const factor = Math.sign(e.deltaY) * 0.1;

    // initial viewBox
    const svg = container.querySelector('svg');
    const x = svg.viewBox.baseVal.x;
    const y = svg.viewBox.baseVal.y;
    const width = svg.viewBox.baseVal.width;
    const height = svg.viewBox.baseVal.height;

    // get SVG user-space coordinates of mouse event
    const rect = svg.getBoundingClientRect();
    const eventSvgX = x + (e.clientX - rect.left) * (width / rect.width);
    const eventSvgY = y + (e.clientY - rect.top) * (height / rect.height);

    // calculate new viewBox
    const newWidth = width * (1 + factor);
    const newHeight = height * (1 + factor);
    const newX = x - (newWidth - width) * ((eventSvgX - x) / width);
    const newY = y - (newHeight - height) * ((eventSvgY - y) / height);

    svg.setAttribute('viewBox', `${newX} ${newY} ${newWidth} ${newHeight}`);
});

/// Middle-button drag: pan SVG
(function () {
    let isDragging = false;
    let startX, startY;

    container.addEventListener('mousedown', (e) => {
        // we're only interested in the middle mouse button
        if (e.button !== 1) return;

        // set cursor to xy move
        document.body.style.cursor = 'move';
        const svg = container.querySelector('svg');
        if (e.target === svg || e.target.parentNode === svg) {
            isDragging = true;
            startX = e.clientX;
            startY = e.clientY;
        }
    });

    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;

        const svg = container.querySelector('svg');
        const rect = svg.getBoundingClientRect();

        const scaleX = svg.viewBox.baseVal.width / rect.width;
        const scaleY = svg.viewBox.baseVal.height / rect.height;
        const dx = (startX - e.clientX) * scaleX;
        const dy = (startY - e.clientY) * scaleY;

        svg.setAttribute('viewBox', `${svg.viewBox.baseVal.x + dx} ${svg.viewBox.baseVal.y + dy} ${svg.viewBox.baseVal.width} ${svg.viewBox.baseVal.height}`);

        // Update position for next move while dragging
        startX = e.clientX;
        startY = e.clientY;
    });

    document.addEventListener('mouseup', () => {
        isDragging = false;
        // reset cursor to default
        document.body.style.cursor = 'default';
    });
}());
