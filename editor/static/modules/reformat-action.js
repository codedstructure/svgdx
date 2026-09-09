import { errorOutput } from './dom.js';
import { hidePopup } from './layout.js';
import { setTabContent } from './storage.js';
import { formatStatusError, setStatus } from './statusbar.js';
import { reformat } from './transform.js';

async function runReformat(state, editor) {
    const result = await reformat(editor.getValue());

    if (!result.ok) {
        errorOutput.innerText = result.error;
        setStatus(formatStatusError(result.error), true);
        return;
    }

    editor.setValue(result.svg);
    setTabContent(state, state.activeTab, result.svg);
    editor.setCursorLine(1);
    editor.setScrollTop(0);
    editor.focus();
    errorOutput.innerText = '';
    setStatus('Input reformatted');
}

export function initReformatAction(state, editor) {
    const button = document.getElementById('reformat-input');

    button.addEventListener('click', async (event) => {
        event.preventDefault();
        hidePopup();
        await runReformat(state, editor);
    });
}
