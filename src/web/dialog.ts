// 2.117.0: file dialogs for the web build. A file chosen here would be a file
// on the server, not on this device, so these say so instead of pretending.
export const FILES_ON_A_COMPUTER = "Files are saved and opened in TIQR on a computer, not on the web.";

export async function open(_options?: unknown): Promise<null> {
  throw FILES_ON_A_COMPUTER;
}

export async function save(_options?: unknown): Promise<null> {
  throw FILES_ON_A_COMPUTER;
}

export async function message(text: string): Promise<void> {
  window.alert(text);
}

export async function ask(text: string): Promise<boolean> {
  return window.confirm(text);
}

export async function confirm(text: string): Promise<boolean> {
  return window.confirm(text);
}
