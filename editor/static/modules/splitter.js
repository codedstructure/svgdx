// Splitter module - handles resizable panel dividers

import { container, editorContainer, outputContainer, svgOutputContainer, textOutputContainer } from './dom.js';
import { resetSplitter } from './layout.js';

/**
 * Set up a splitter for resizing panels
 * @param {HTMLElement} splitter - The splitter element
 * @param {string} orientation - The orientation this splitter works in ('vertical' or 'horizontal')
 * @param {HTMLElement} targetContainer - The container that gets resized
 * @param {HTMLElement} otherContainer - The adjacent container
 */
export function setupSplitter(splitter, orientation, targetContainer, otherContainer) {
    let initialClientPos, initialSize, pointerId;

    splitter.addEventListener('pointerdown', (e) => {
        e.preventDefault();
        pointerId = e.pointerId;
        splitter.setPointerCapture(pointerId);

        if (container.dataset.layout === orientation) {
            initialClientPos = e.clientX;
            initialSize = targetContainer.getBoundingClientRect().width;
        } else {
            initialClientPos = e.clientY;
            initialSize = targetContainer.getBoundingClientRect().height;
        }

        splitter.addEventListener('pointermove', pointermove);
        splitter.addEventListener('pointerup', pointerup);
        splitter.addEventListener('pointercancel', pointerup);
    });

    // Double-click to reset split
    splitter.addEventListener('dblclick', () => {
        resetSplitter(targetContainer, otherContainer, orientation);
    });

    function pointermove(e) {
        if (container.dataset.layout === orientation) {
            handleWidthResize(e);
        } else {
            handleHeightResize(e);
        }
        e.preventDefault();
    }

    function handleWidthResize(e) {
        const dx = e.clientX - initialClientPos;
        let newWidth = initialSize + dx;

        const edgeMin = 100;
        const collapseAt = 40;
        const uncollapseAt = 20;
        const minPixels = Math.max(edgeMin, container.clientWidth * 0.2);
        const maxPixels = Math.max(edgeMin, container.clientWidth * 0.8);

        let resetStyleSize = false;

        // Allow splitter to hide/show containers at edges
        if (newWidth < minPixels - collapseAt) {
            targetContainer.classList.add('minimized');
            resetStyleSize = true;
        } else if (newWidth > minPixels - uncollapseAt) {
            targetContainer.classList.remove('minimized');
        }

        if (newWidth > maxPixels + collapseAt) {
            otherContainer.classList.add('minimized');
            targetContainer.classList.add('maximized');
            resetStyleSize = true;
        } else if (newWidth < maxPixels + uncollapseAt) {
            otherContainer.classList.remove('minimized');
        }

        if (resetStyleSize) {
            targetContainer.style.width = '';
            targetContainer.style.minWidth = '';
            return;
        }

        // Enforce min and max widths
        newWidth = Math.max(newWidth, minPixels);
        newWidth = Math.min(newWidth, maxPixels);

        targetContainer.classList.remove('maximized', 'minimized');
        otherContainer.classList.remove('maximized', 'minimized');

        targetContainer.style.width = newWidth + 'px';
        targetContainer.style.minWidth = newWidth + 'px';
    }

    function handleHeightResize(e) {
        const dy = e.clientY - initialClientPos;
        let newHeight = initialSize + dy;

        const edgeMin = 50;
        const collapseAt = 40;
        const uncollapseAt = 20;
        const minPixels = Math.max(edgeMin, container.clientHeight * 0.2);
        const maxPixels = Math.max(edgeMin, container.clientHeight * 0.8);

        let resetStyleSize = false;

        // Allow splitter to hide/show containers at edges
        if (newHeight < minPixels - collapseAt) {
            targetContainer.classList.add('minimized');
            resetStyleSize = true;
        } else if (newHeight > minPixels - uncollapseAt) {
            targetContainer.classList.remove('minimized');
        }

        if (newHeight > maxPixels + collapseAt) {
            otherContainer.classList.add('minimized');
            targetContainer.classList.add('maximized');
            resetStyleSize = true;
        } else if (newHeight < maxPixels + uncollapseAt) {
            otherContainer.classList.remove('minimized');
        }

        if (resetStyleSize) {
            targetContainer.style.height = '';
            targetContainer.style.minHeight = '';
            return;
        }

        // Enforce min and max heights
        newHeight = Math.max(newHeight, minPixels);
        newHeight = Math.min(newHeight, maxPixels);

        targetContainer.classList.remove('maximized', 'minimized');
        otherContainer.classList.remove('maximized', 'minimized');

        targetContainer.style.height = newHeight + 'px';
        targetContainer.style.minHeight = newHeight + 'px';
    }

    function pointerup(e) {
        splitter.releasePointerCapture(pointerId);
        splitter.removeEventListener('pointermove', pointermove);
        splitter.removeEventListener('pointerup', pointerup);
        splitter.removeEventListener('pointercancel', pointerup);
    }
}

/**
 * Initialize all splitters
 */
export function initSplitters() {
    setupSplitter(
        document.getElementById('main-split'),
        'vertical',
        editorContainer,
        outputContainer
    );
    setupSplitter(
        document.getElementById('output-split'),
        'horizontal',
        svgOutputContainer,
        textOutputContainer
    );
}
