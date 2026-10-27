// 2.117.0: the window API for the web build. A browser tab is not a TIQR
// window: there is nothing to keep open for a last sync, minimise or close.
const tab = {
  label: "main",
  async onCloseRequested(_handler: unknown): Promise<() => void> {
    return () => undefined;
  },
  async close() {},
  async minimize() {},
  async unminimize() {},
  async show() {},
  async setFocus() {},
  async setTitle(title: string) {
    document.title = title;
  },
};

export function getCurrentWindow() {
  return tab;
}
