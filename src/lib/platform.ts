// 2.117.0: true in the web build (`npm run build:web`, served by Web access
// from TIQR on the server - src-tauri/src/web_access.rs), false in the app.
// Read where a screen does what only TIQR's own window on a computer should
// do: run live sync and the mail sharing, read Gmail, check for updates.
declare const __TIQR_WEB__: boolean | undefined;

export const IS_WEB: boolean = typeof __TIQR_WEB__ !== "undefined" && __TIQR_WEB__ === true;

// 2.118.0: true in the phone build (`npm run build:phone`) - TIQR itself in the
// browser, its data on this phone (src/phone). IS_WEB is true there too: the
// screens are in a browser tab. Where the web build leaves a job to TIQR's
// window on the server, the phone does it itself - live sync above all.
declare const __TIQR_PHONE__: boolean | undefined;

export const IS_PHONE: boolean = typeof __TIQR_PHONE__ !== "undefined" && __TIQR_PHONE__ === true;
