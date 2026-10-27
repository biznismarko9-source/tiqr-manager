import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { installDiagnostics } from "./lib/diagnosticsSink";
import { IS_WEB } from "./lib/platform";

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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
