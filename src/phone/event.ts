// 2.118.0: `listen` for the phone build - the events TIQR in this page's
// worker sends (a change arrived, a run finished), as the computer's window
// hears them from its backend.
import { onEvent } from "./backend";

export type UnlistenFn = () => void;
type Handler = (event: { event: string; id: number; payload: unknown }) => void;

let nextId = 1;

export async function listen<T>(
  event: string,
  handler: (event: { event: string; id: number; payload: T }) => void,
): Promise<UnlistenFn> {
  return onEvent((name, payload) => {
    if (name === event) (handler as Handler)({ event: name, id: nextId++, payload });
  });
}
