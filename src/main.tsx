import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// 2.2.10 (marko): the webview's own native right-click menu (Back/Reload/
// Save as/Print/More tools - Tauri's default WRY/Chromium context menu) has
// no use in a desktop app that is not a general-purpose browser, and marko
// asked for right-click to do nothing at all everywhere. A single
// `document`-level listener, added once here before the app ever renders,
// covers every page/component without touching any of them individually -
// there is no Tauri config flag for this (as of Tauri 2, the context menu is
// suppressed from the web content side, not the window/webview config side).
document.addEventListener("contextmenu", (e) => e.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
