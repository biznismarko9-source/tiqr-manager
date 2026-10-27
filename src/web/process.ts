// 2.117.0: "restart" in the web build is a reload of the page. TIQR on the
// server keeps running.
export async function relaunch(): Promise<void> {
  window.location.reload();
}

export async function exit(_code?: number): Promise<void> {}
