// 2.117.0: `tiqrmanager://` links open TIQR on a computer; a browser tab never
// receives them.
export async function getCurrent(): Promise<string[] | null> {
  return null;
}

export async function onOpenUrl(_handler: (urls: string[]) => void): Promise<() => void> {
  return () => undefined;
}
