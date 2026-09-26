import { PNG_RESOLUTIONS } from './config.js';

const ICON_PATHS = Object.freeze({
    download: 'M8 2v8m-3 -4l3 4l3 -4M3 10v3h10v-3',
    copy: 'M3 3h8v11h-8zM5 3v-1.5h7.5v11h-1.5',
    reformat: 'M3 2.5h5M5 5h8M7 8h5M5 11h4M3 14h5',
    layoutVertical: 'M8 0v16M1 2h3M1 3.5h3.5M2 5h3.5M2 6.5h5M1 8h3M1 9.5h5M1 11h4M11 3h2v2h-2zM12 5v5h-1v2h2v-2h-1',
    layoutVerticalText: 'M8 0v16M1 2h3M1 3.5h3.5M2 5h3.5M2 6.5h5M1 8h3M1 9.5h5M1 11h4M11 3h2v2h-2zM12 5v3M8 8h8M10 10h3m-3 1.5h4m-4 1.5h3m-3 1.5h4',
    layoutHorizontal: 'M0 8h16 M1 1h3M1 2.5h5M2 4h7M2 5.5h5M1 7h4M6 12v-1h-2v2h2v-1h4v-1h2v2h-2v-1',
    layoutHorizontalText: 'M0 8h16 M1 1h3M1 2.5h5M2 4h7M2 5.5h5M1 7h4M6 12v-1h-2v2h2v-1h2M8 8v8M10 10h3m-3 1.5h4m-4 1.5h3m-3 1.5h4',
});

const DESKTOP_LAYOUT_ROWS = [
    {
        label: 'Vertical',
        actions: [
            {
                id: 'layout-vertical',
                icon: 'layoutVertical',
                title: 'Vertical split',
                ariaLabel: 'Vertical split',
                info: 'Show the editor beside the SVG output',
                classNames: ['popup-layout-action'],
                dataset: { layoutSelection: 'vertical' }
            },
            {
                id: 'layout-v-text',
                icon: 'layoutVerticalText',
                title: 'Vertical split with SVG and text',
                ariaLabel: 'Vertical split with SVG and text',
                info: 'Show the editor beside SVG and text output',
                classNames: ['popup-layout-action'],
                dataset: { layoutSelection: 'v-text' }
            }
        ]
    },
    {
        label: 'Horizontal',
        actions: [
            {
                id: 'layout-horizontal',
                icon: 'layoutHorizontal',
                title: 'Horizontal split',
                ariaLabel: 'Horizontal split',
                info: 'Show the editor above the SVG output',
                classNames: ['popup-layout-action'],
                dataset: { layoutSelection: 'horizontal' }
            },
            {
                id: 'layout-h-text',
                icon: 'layoutHorizontalText',
                title: 'Horizontal split with SVG and text',
                ariaLabel: 'Horizontal split with SVG and text',
                info: 'Show the editor above SVG and text output',
                classNames: ['popup-layout-action'],
                dataset: { layoutSelection: 'h-text' }
            }
        ]
    }
];

const MOBILE_LAYOUT_BUTTONS = [
    {
        id: 'mobile-layout-svg',
        text: 'SVG output',
        info: 'Show the SVG output',
        dataset: { mobileLayoutSelection: 'svg' }
    },
    {
        id: 'mobile-layout-xml',
        text: 'XML output',
        info: 'Show the XML output',
        dataset: { mobileLayoutSelection: 'xml' }
    }
];

const INPUT_ROWS = [
    {
        label: 'Reformat',
        actions: [
            {
                id: 'reformat-input',
                icon: 'reformat',
                title: 'Reformat input document',
                ariaLabel: 'Reformat input document',
                info: 'Reformat input document',
                classNames: ['popup-input-action'],
                dataset: { inputAction: 'reformat' }
            }
        ]
    },
    {
        label: 'Input',
        actions: [
            {
                id: 'save-input',
                icon: 'download',
                title: 'Download the input',
                ariaLabel: 'Download the input',
                info: 'Download the input',
                classNames: ['popup-input-action'],
                dataset: { inputAction: 'download' }
            },
            {
                id: 'copy-input',
                icon: 'copy',
                title: 'Copy input to clipboard',
                ariaLabel: 'Copy input to clipboard',
                info: 'Copy input to clipboard',
                classNames: ['popup-input-action'],
                dataset: { inputAction: 'copy' }
            }
        ]
    }
];

function getOutputRows() {
    return [
        {
            label: 'SVG',
            actions: [
                {
                    icon: 'download',
                    title: 'Download SVG output',
                    ariaLabel: 'Download SVG output',
                    info: 'Download SVG output',
                    classNames: ['popup-output-action'],
                    dataset: { format: 'svg', action: 'download' }
                },
                {
                    icon: 'copy',
                    title: 'Copy SVG output',
                    ariaLabel: 'Copy SVG output',
                    info: 'Copy SVG output',
                    classNames: ['popup-output-action'],
                    dataset: { format: 'svg', action: 'copy' }
                }
            ]
        },
        ...PNG_RESOLUTIONS.map(resolution => ({
            label: `PNG (${resolution}px)`,
            actions: [
                {
                    icon: 'download',
                    title: `Download PNG output at ${resolution}px`,
                    ariaLabel: `Download PNG output at ${resolution}px`,
                    info: `Download PNG output at ${resolution}px`,
                    classNames: ['popup-output-action'],
                    dataset: { format: 'png', action: 'download', resolution: String(resolution) }
                },
                {
                    icon: 'copy',
                    title: `Copy PNG output at ${resolution}px`,
                    ariaLabel: `Copy PNG output at ${resolution}px`,
                    info: `Copy PNG output at ${resolution}px`,
                    classNames: ['popup-output-action'],
                    dataset: { format: 'png', action: 'copy', resolution: String(resolution) }
                }
            ]
        }))
    ];
}

function createSvgIcon(iconName) {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 16 16');
    svg.setAttribute('aria-hidden', 'true');

    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', ICON_PATHS[iconName]);
    svg.appendChild(path);

    return svg;
}

function createIconButton(definition) {
    const button = document.createElement('button');
    button.type = 'button';
    button.classList.add('popup-menu-button');

    if (definition.id) {
        button.id = definition.id;
    }
    if (definition.classNames) {
        button.classList.add(...definition.classNames);
    }
    if (definition.title) {
        button.title = definition.title;
    }
    if (definition.ariaLabel) {
        button.setAttribute('aria-label', definition.ariaLabel);
    }
    if (definition.info) {
        button.dataset.info = definition.info;
    }
    if (definition.dataset) {
        for (const [key, value] of Object.entries(definition.dataset)) {
            button.dataset[key] = value;
        }
    }

    button.appendChild(createSvgIcon(definition.icon));
    return button;
}

function createRow(rowDefinition) {
    const row = document.createElement('div');
    row.className = 'popup-menu-row';

    const label = document.createElement('span');
    label.className = 'popup-menu-label';
    label.textContent = rowDefinition.label;

    const actions = document.createElement('div');
    actions.className = 'popup-menu-actions';
    for (const action of rowDefinition.actions) {
        actions.appendChild(createIconButton(action));
    }

    row.append(label, actions);
    return row;
}

function createTextButton(definition) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'popup-button';
    button.id = definition.id;
    button.textContent = definition.text;

    if (definition.info) {
        button.dataset.info = definition.info;
    }
    if (definition.dataset) {
        for (const [key, value] of Object.entries(definition.dataset)) {
            button.dataset[key] = value;
        }
    }

    return button;
}

function populateRowMenu(containerId, rows) {
    const popup = document.querySelector(`#${containerId} .popup-buttons`);
    popup.classList.add('popup-menu');
    popup.replaceChildren(...rows.map(createRow));
}

function populateTextMenu(containerId, definitions) {
    const popup = document.querySelector(`#${containerId} .popup-buttons`);
    popup.classList.remove('popup-menu');
    popup.replaceChildren(...definitions.map(createTextButton));
}

export function initPopupMenus() {
    populateRowMenu('layout-popup', DESKTOP_LAYOUT_ROWS);
    populateTextMenu('mobile-layout-popup', MOBILE_LAYOUT_BUTTONS);
    populateRowMenu('input-popup', INPUT_ROWS);
    populateRowMenu('output-popup', getOutputRows());
}
