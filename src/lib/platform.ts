// 2.117.0: true in the web build (`npm run build:web`, served by Web access
// from TIQR on the server - src-tauri/src/web_access.rs), false in the app.
// Read where a screen does what only TIQR's own window on a computer should
// do: run live sync and the mail sharing, read Gmail, check for updates.
declare const __TIQR_WEB__: boolean | undefined;

export const IS_WEB: boolean = typeof __TIQR_WEB__ !== "undefined" && __TIQR_WEB__ === true;
