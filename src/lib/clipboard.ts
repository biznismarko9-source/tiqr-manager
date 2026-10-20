// 2.113.0: copying from the mail reader - a link, a one-time code.
//
// The webview's own clipboard first; the old selection trick when that is
// refused (an older WebView, or a click that the browser does not count as
// the user's own). Returns whether it worked, so the screen never says
// "Copied" when it was not.
export async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to the selection trick.
  }
  try {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    area.remove();
    return ok;
  } catch {
    return false;
  }
}
