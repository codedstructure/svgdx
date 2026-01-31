// Transform module - handles document transformation via WASM or server

import {
    JSON_API_VERSION,
    RATE_LIMIT_WASM_MS,
    RATE_LIMIT_SERVER_MS,
    IN_PROGRESS_TIMEOUT_MS
} from './config.js';
import { statusbar } from './dom.js';

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
async function transformViaServer(input, addMetadata) {
    try {
        statusbar.style.opacity = '0.3';

        const request = {
            version: JSON_API_VERSION,
            input: input,
            config: {
                add_metadata: addMetadata
            }
        };

        const response = await fetch('api/transform_json', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(request)
        });

        statusbar.style.opacity = null;
        statusbar.style.color = null;

        const result = await response.json();

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
    } catch (e) {
        statusbar.style.color = 'darkred';
        statusbar.innerText = `svgdx editor - error: ${e.message}`;
        console.error('Error sending data to /api/transform_json', e);
        return {
            ok: false,
            error: e.message,
            warnings: []
        };
    }
}

/**
 * Transform input via local WASM
 * Returns { ok: boolean, svg?: string, error?: string, warnings: string[] }
 */
function transformViaWasm(input, addMetadata) {
    try {
        if (!window.svgdx_transform_json) {
            return {
                ok: false,
                error: 'loading svgdx...',
                warnings: []
            };
        }

        const request = {
            version: JSON_API_VERSION,
            input: input,
            config: {
                add_metadata: addMetadata
            }
        };

        const resultJson = window.svgdx_transform_json(JSON.stringify(request));
        const result = JSON.parse(resultJson);

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
    } catch (e) {
        return {
            ok: false,
            error: e.toString(),
            warnings: []
        };
    }
}

/**
 * Transform input document
 * Automatically routes to server or WASM based on svgdx_use_server flag
 * Returns { ok: boolean, svg?: string, error?: string, warnings: string[] }
 */
export async function transform(input, addMetadata) {
    if (window.svgdx_use_server) {
        return await transformViaServer(input, addMetadata);
    } else {
        return transformViaWasm(input, addMetadata);
    }
}

/**
 * Check if svgdx is ready (bootstrap has completed)
 */
export function isReady() {
    return window.hasOwnProperty('svgdx_use_server');
}
