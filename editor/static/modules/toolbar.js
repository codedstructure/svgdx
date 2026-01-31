// Toolbar module - handles toolbar scrolling and popup positioning

const toolbar = document.getElementById('toolbar');
const toolbarScroll = document.getElementById('toolbar-scroll');
const scrollLeftBtn = toolbar.querySelector('.scroll-left');
const scrollRightBtn = toolbar.querySelector('.scroll-right');

/**
 * Update scroll button visibility based on scroll position
 */
function updateScrollButtons() {
    const scrollLeft = toolbarScroll.scrollLeft;
    const scrollWidth = toolbarScroll.scrollWidth;
    const clientWidth = toolbarScroll.clientWidth;
    const maxScroll = scrollWidth - clientWidth;

    // Can scroll left if not at the start
    const canScrollLeft = scrollLeft > 1; // 1px tolerance
    // Can scroll right if not at the end
    const canScrollRight = scrollLeft < maxScroll - 1; // 1px tolerance

    toolbar.classList.toggle('can-scroll-left', canScrollLeft);
    toolbar.classList.toggle('can-scroll-right', canScrollRight);
}

/**
 * Scroll the toolbar by a given amount, clamped to valid range
 */
function scrollToolbar(delta) {
    const scrollLeft = toolbarScroll.scrollLeft;
    const maxScroll = toolbarScroll.scrollWidth - toolbarScroll.clientWidth;
    const newScroll = Math.max(0, Math.min(maxScroll, scrollLeft + delta));

    toolbarScroll.scrollTo({ left: newScroll, behavior: 'smooth' });
}

/**
 * Position a popup menu below its trigger button, centered
 */
function positionPopup(container) {
    const trigger = container.querySelector('button:not(.popup-button)');
    const popup = container.querySelector('.popup-buttons');
    if (!trigger || !popup) return;

    const triggerRect = trigger.getBoundingClientRect();

    // Temporarily show popup to measure its width
    popup.style.visibility = 'hidden';
    popup.style.display = 'block';
    const popupWidth = popup.offsetWidth;
    popup.style.display = '';
    popup.style.visibility = '';

    // Center popup below trigger button
    const triggerCenter = triggerRect.left + triggerRect.width / 2;
    let left = triggerCenter - popupWidth / 2;

    // Clamp to viewport edges with some padding
    const padding = 8;
    const maxLeft = window.innerWidth - popupWidth - padding;
    left = Math.max(padding, Math.min(maxLeft, left));

    popup.style.top = `${triggerRect.bottom}px`;
    popup.style.left = `${left}px`;
    popup.style.minWidth = `${triggerRect.width}px`;
}

/**
 * Show a popup menu
 */
function showPopup(container) {
    // Hide any other open popups first
    document.querySelectorAll('.popup-container.show-popup').forEach(c => {
        if (c !== container) c.classList.remove('show-popup');
    });

    positionPopup(container);
    container.classList.add('show-popup');
}

/**
 * Hide a popup menu
 */
function hidePopup(container) {
    container.classList.remove('show-popup');
}

/**
 * Hide all popup menus
 */
function hideAllPopups() {
    document.querySelectorAll('.popup-container.show-popup').forEach(c => {
        c.classList.remove('show-popup');
    });
}

/**
 * Initialize toolbar functionality
 */
export function initToolbar() {
    // Scroll buttons
    scrollLeftBtn.addEventListener('click', () => scrollToolbar(-100));
    scrollRightBtn.addEventListener('click', () => scrollToolbar(100));

    // Update scroll button visibility on scroll, resize, and load
    toolbarScroll.addEventListener('scroll', updateScrollButtons);
    window.addEventListener('resize', updateScrollButtons);
    // Initial check (defer to allow layout to settle)
    setTimeout(updateScrollButtons, 0);

    // Popup handling - show on click/hover, hide on leave
    document.querySelectorAll('.popup-container').forEach(container => {
        const trigger = container.querySelector('button:not(.popup-button)');
        if (!trigger) return;

        // Show popup on hover (desktop) or click (touch)
        trigger.addEventListener('mouseenter', () => showPopup(container));
        trigger.addEventListener('click', (e) => {
            e.stopPropagation();
            if (container.classList.contains('show-popup')) {
                hidePopup(container);
            } else {
                showPopup(container);
            }
        });

        // Keep popup visible while hovering over it
        const popup = container.querySelector('.popup-buttons');
        if (popup) {
            popup.addEventListener('mouseenter', () => showPopup(container));
            popup.addEventListener('mouseleave', () => hidePopup(container));
        }

        // Hide popup when leaving the trigger (with delay to allow moving to popup)
        trigger.addEventListener('mouseleave', () => {
            setTimeout(() => {
                if (!container.matches(':hover') && !popup?.matches(':hover')) {
                    hidePopup(container);
                }
            }, 100);
        });
    });

    // Hide popups when clicking outside
    document.addEventListener('click', (e) => {
        if (!e.target.closest('.popup-container')) {
            hideAllPopups();
        }
    });

    // Hide popups on scroll (they'd be mispositioned)
    toolbarScroll.addEventListener('scroll', hideAllPopups);

    // Reposition popups on window scroll/resize
    window.addEventListener('scroll', hideAllPopups);
}

// Export hideAllPopups for use by other modules (e.g., after selecting an option)
export { hideAllPopups };
