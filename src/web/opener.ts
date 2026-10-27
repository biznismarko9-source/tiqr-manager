// 2.117.0: links for the web build open in a new tab of this browser.
import { FILES_ON_A_COMPUTER } from "./dialog";

export async function openUrl(url: string): Promise<void> {
  window.open(url, "_blank", "noopener,noreferrer");
}

export async function openPath(_path: string): Promise<void> {
  throw FILES_ON_A_COMPUTER;
}

export async function revealItemInDir(_path: string): Promise<void> {
  throw FILES_ON_A_COMPUTER;
}
