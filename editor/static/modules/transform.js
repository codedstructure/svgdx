// Transform module - handles document transformation via WASM or server

import {
    JSON_API_VERSION,
    RATE_LIMIT_WASM_MS,
    RATE_LIMIT_SERVER_MS,
    IN_PROGRESS_TIMEOUT_MS
} from './config.js';
import { statusbar } from './dom.js';
import { formatStatusError } from './statusbar.js';

function buildRequest(input, config = {}) {
    return {
        version: JSON_API_VERSION,
        input: input,
        config: config
    };
}

function parseJsonResult(result) {
    if (result.error) {
        return {
            ok: false,
            error: result.error,
            warnings: result.warnings || []
        };
    }

    return {
        ok: true,
        svg: result.svg,
        warnings: result.warnings || []
    };
}

/**
 * Create a rate-limited wrapper for a function
 * Prevents excessive calls while ensuring eventual consistency
 */
export function rateLimited(target, useServer) {
    const maxCallRate = useServer ? RATE_LIMIT_SERVER_MS : RATE_LIMIT_WASM_MS;
    let lastCallTime = 0;
    let callInProgress = false;
    let pendingCall = false;

    return async function() {
        const now = Date.now();

        // Prevent new requests if already in progress, unless they
        // were a very long time ago.
        if (callInProgress && lastCallTime + IN_PROGRESS_TIMEOUT_MS > now) {
            pendingCall = true;
            return;
        }

        if (now - lastCallTime >= maxCallRate) {
            // Call target immediately if last call was a while ago
            // to avoid latency on infrequent calls
            lastCallTime = now;
            callInProgress = true;
            await target();
            callInProgress = false;

            if (pendingCall) {
                // Another call came in while running target, schedule it
                // so eventual state is up-to-date
                pendingCall = false;
                rateLimited(target, useServer)();
            }
        } else {
            // Schedule next call to target after maxCallRate since last
            setTimeout(async () => {
                if (!callInProgress) {
                    lastCallTime = Date.now();
                    callInProgress = true;
                    await target();
                    callInProgress = false;

                    if (pendingCall) {
                        pendingCall = false;
                        rateLimited(target, useServer)();
                    }
                }
            }, maxCallRate - (now - lastCallTime));
        }
    };
}

/**
 * Transform input via server API
 * Returns { ok: boolean, svg?: string, error?: string, warnings: string[] }
 */
async function callServerJson(path, input, config) {
    try {
        statusbar.style.opacity = '0.3';

        const response = await fetch(path, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(buildRequest(input, config))
        });

        statusbar.style.opacity = null;
        statusbar.style.color = null;

        return parseJsonResult(await response.json());
    } catch (e) {
        statusbar.style.color = 'darkred';
        statusbar.innerText = formatStatusError(e.message);
        console.error(`Error sending data to ${path}`, e);
        return {
            ok: false,
            error: e.message,
            warnings: []
        };
    }
}

async function transformViaServer(input, config) {
    return callServerJson('api/transform_json', input, config);
}

async function reformatViaServer(input) {
    return callServerJson('api/reformat_json', input, {});
}

/**
 * Transform input via local WASM
 * Returns { ok: boolean, svg?: string, error?: string, warnings: string[] }
 */
function callWasmJson(functionName, input, config) {
    try {
        const handler = window[functionName];

        if (!handler) {
            return {
                ok: false,
                error: 'loading svgdx...',
                warnings: []
            };
        }

        const resultJson = handler(JSON.stringify(buildRequest(input, config)));
        return parseJsonResult(JSON.parse(resultJson));
    } catch (e) {
        return {
            ok: false,
            error: e.toString(),
            warnings: []
        };
    }
}

function transformViaWasm(input, config) {
    return callWasmJson('svgdx_transform_json', input, config);
}

function reformatViaWasm(input) {
    return callWasmJson('svgdx_reformat_json', input, {});
}

/**
 * Transform input document
 * Automatically routes to server or WASM based on svgdx_use_server flag
 * Returns { ok: boolean, svg?: string, error?: string, warnings: string[] }
 */
export async function transform(input, config) {
    if (window.svgdx_use_server) {
        return await transformViaServer(input, config);
    } else {
        return transformViaWasm(input, config);
    }
}

export async function reformat(input) {
    if (window.svgdx_use_server) {
        return await reformatViaServer(input);
    } else {
        return reformatViaWasm(input);
    }
}

/**
 * Check if svgdx is ready (bootstrap has completed)
 */
export function isReady() {
    return window.hasOwnProperty('svgdx_use_server');
}
