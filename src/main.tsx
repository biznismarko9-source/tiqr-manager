import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { installDiagnostics } from "./lib/diagnosticsSink";
import { quietly } from "./lib/diagnostics";
import { IS_PHONE, IS_WEB } from "./lib/platform";

// 2.116.0: from the very first moment, every failure is kept - including
// the ones nothing on the screen catches (lib/diagnosticsSink.ts).
installDiagnostics();

// 2.2.10 (marko): the webview's own native right-click menu (Back/Reload/
// Save as/Print/More tools - Tauri's default WRY/Chromium context menu) has
// no use in a desktop app that is not a general-purpose browser, and marko
// asked for right-click to do nothing at all everywhere. A single
// `document`-level listener, added once here before the app ever renders,
// covers every page/component without touching any of them individually -
// there is no Tauri config flag for this (as of Tauri 2, the context menu is
// suppressed from the web content side, not the window/webview config side).
// 2.117.0: a browser keeps its own menu - copying a link or text on a
// phone needs it.
if (!IS_WEB) document.addEventListener("contextmenu", (e) => e.preventDefault());

// 2.118.0: TIQR on a phone opens without a connection (phone-host/sw.js), and
// asks the browser to keep its data even when the phone runs short of space -
// an iPhone's home-screen app always keeps it.
if (IS_PHONE) {
  if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(quietly("main: service worker"));
  navigator.storage?.persist?.().catch(quietly("main: keep this phone's data"));
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
