import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { useLocation, useNavigate } from "react-router-dom";

/* ==========================================================================
   The guided tour (2.25.0)

   Marko: "ten guide nieje dobry treba ho prerobit na poriadny guide, mozno
   taky ze realne ti poukazuje veci ze to prejde s tebou napr na dashboarde ta
   vezme cez tie hlavne kolonky a takto prejde kazdu cast."

   So this is not a list of paragraphs in a card any more. It walks the app:
   each step NAVIGATES to the real page, finds a real element on it, dims
   everything else and puts the explanation next to the thing it is talking
   about. The seven-paragraph guide it replaces said true things about an app
   the reader could not see.

   HOW IT FINDS THINGS

   Elements opt in with `data-tour="<key>"`. Four files carry those attributes
   and every one of them is a single attribute with no logic attached:

     ui.tsx        PageHeader  -> "page-header" and "page-actions", which puts
                                 an anchor on EVERY page at once rather than
                                 editing a dozen page files.
     Layout.tsx    the sidebar -> "nav", and "nav:<route>" per link.
     Dashboard.tsx             -> "dash-stats", "dash-period".
     Settings.tsx              -> "settings-sections".

   NOTHING IS ASSUMED TO BE THERE

   A step lists its anchors in order of preference and the first one that
   actually exists wins. If none do - the page is still loading, the Tickets
   group in the sidebar is collapsed, a page has no action buttons - the step
   still runs, just centred and without a spotlight. The tour is never allowed
   to point at nothing, and it is never allowed to stall waiting for an
   element that is not coming.
   ========================================================================== */

interface TourStep {
  /** Where this step happens. The tour navigates there before measuring. */
  route: string;
  /** `data-tour` keys, best first. Missing ones are skipped, not waited on. */
  anchors?: string[];
  title: string;
  body: string;
}

const STEPS: TourStep[] = [
  {
    route: "/",
    anchors: ["nav"],
    title: "This is the whole app",
    body: "Everything is in this one list, and it is in the order the work actually happens: events, then orders, then tickets, then sales. Nothing is hidden behind a menu.",
  },
  {
    route: "/",
    anchors: ["dash-stats"],
    title: "Your six headline numbers",
    body: "Profit is what you actually made on tickets that sold. Revenue is what came in, purchase cost is what those same tickets cost you. Margin is profit against revenue; ROI is profit against cost - they answer different questions, which is why both are here.",
  },
  {
    route: "/",
    anchors: ["dash-period", "page-header"],
    title: "And they are all for one period",
    body: "Change this and every figure above changes with it. Nothing on this page is all-time unless it says so, so a quiet week is a quiet week and not a broken number.",
  },
  {
    route: "/events",
    anchors: ["page-actions", "page-header"],
    title: "Start here: the event",
    body: "Everything hangs off an event - orders, tickets, sales. If the date is not settled yet, leave it empty instead of guessing: the app shows TBD and sorts those separately rather than pretending you know.",
  },
  {
    route: "/orders",
    anchors: ["page-actions", "page-header"],
    title: "Then the order - it creates the tickets for you",
    body: "Enter what you paid for the whole order and how many tickets it was. The app splits that cost across them to the exact cent, so you never type a per-ticket price. Section, row and seat are labels on a ticket, not prices.",
  },
  {
    route: "/tickets",
    anchors: ["page-header"],
    title: "Every ticket the orders created",
    body: "This is where a ticket gets its listing price. A ticket without one cannot sell, and the Dashboard counts those separately instead of quietly leaving them out - so an empty listing price is visible rather than invisible.",
  },
  {
    route: "/sales",
    anchors: ["page-actions", "page-header"],
    title: "When one sells, record it here",
    body: "With the platform and the fee that platform took. The fee is the part people skip, and it is the part that makes your margin real instead of optimistic.",
  },
  {
    route: "/tickets",
    anchors: ["page-header"],
    title: "What is still yours",
    body: "Everything bought and not yet sold, and what it cost you. This is capital sitting still - the app never counts any of it as profit, no matter what it is listed at.",
  },
  {
    route: "/pulls",
    anchors: ["page-actions", "page-header"],
    title: "Pulls - tickets you owe someone",
    body: "Tickets promised to a buyer with a transfer deadline. The Dashboard watches these dates, because this is the one part of the job where being late costs you the sale.",
  },
  {
    route: "/finance",
    anchors: ["page-header"],
    title: "Money that is not a ticket",
    body: "Accounts, transactions and recurring costs - subscriptions, fees, anything that does not belong to one order. It is kept apart from ticket profit on purpose, so neither one flatters the other.",
  },
  {
    route: "/settings/data",
    anchors: ["page-header"],
    title: "Turn sync on once, then forget it",
    body: "Your two computers keep themselves level on their own after that: changes go up every few minutes and come down when you open the app. If both changed, they get combined - nothing is thrown away. Backups, restore points and every earlier version in Drive are on this same screen.",
  },
  {
    route: "/settings/insights",
    anchors: ["page-header"],
    title: "And when you want the story",
    body: "Ticket and Finance recaps play as a short sequence and then open the full report: what you bought, what it returned, what is still owed to you and what is still sitting in stock. That is the end of the tour - you can start it again from Support whenever you like.",
  },
];

/* A tour can be started from anywhere (today: Settings -> Support). Rather
   than threading a context through Layout for one button, the component
   subscribes to this set - which, unlike a single module-level callback,
   survives React re-mounting the component. */
const listeners = new Set<() => void>();

/** Starts the guided tour from wherever the user currently is. */
export function startTour() {
  listeners.forEach((l) => l());
}

/** How long to keep looking for a step's anchor before giving up and showing
 *  the step centred instead. Long enough for a page to fetch and paint, short
 *  enough that a missing element never feels like a hang. */
const ANCHOR_TIMEOUT_MS = 1600;

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

export function Tour() {
  const navigate = useNavigate();
  const location = useLocation();
  const [active, setActive] = useState(false);
  const [i, setI] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  /** Where the user was when they pressed Start, so the tour can put them
   *  back instead of abandoning them on whatever page it ended on. */
  const cameFrom = useRef<string>("/");
  /** The live pathname, so `start` below reads where the user IS rather than
   *  the value that was current when the subscription was created. */
  const herePath = useRef(location.pathname);
  herePath.current = location.pathname;

  useEffect(() => {
    const start = () => {
      cameFrom.current = herePath.current || "/settings/support";
      setI(0);
      setBox(null);
      setActive(true);
    };
    listeners.add(start);
    return () => {
      listeners.delete(start);
    };
  }, []);

  const step = active ? STEPS[Math.min(i, STEPS.length - 1)] : null;

  const finish = useCallback(() => {
    setActive(false);
    setBox(null);
    navigate(cameFrom.current || "/settings/support");
  }, [navigate]);

  // Get to the step's page first. Measuring an element on a page we have not
  // navigated to yet would find nothing every time.
  useEffect(() => {
    if (!step) return;
    if (location.pathname !== step.route) navigate(step.route);
  }, [step, location.pathname, navigate]);

  // Then find the anchor. Polled rather than done once: the page has to fetch
  // and paint, and how long that takes is not something this component can
  // know. Gives up after ANCHOR_TIMEOUT_MS and shows the step centred.
  useEffect(() => {
    if (!step) return;
    let done = false;
    const started = Date.now();
    setBox(null);

    const measure = (el: Element): Box => {
      const r = el.getBoundingClientRect();
      return { left: r.left, top: r.top, width: r.width, height: r.height };
    };

    const find = (): Element | null => {
      for (const key of step.anchors ?? []) {
        const el = document.querySelector(`[data-tour="${key}"]`);
        // A zero-size element is present but not laid out yet - keep looking.
        if (el && el.getBoundingClientRect().height > 0) return el;
      }
      return null;
    };

    const tick = () => {
      if (done) return;
      if (location.pathname === step.route) {
        const el = find();
        if (el) {
          el.scrollIntoView({ block: "center", behavior: "smooth" });
          // One frame later, so the measurement is of where it ENDED UP.
          window.setTimeout(() => {
            if (!done) setBox(measure(el));
          }, 320);
          done = true;
          return;
        }
      }
      if (Date.now() - started > ANCHOR_TIMEOUT_MS) {
        done = true; // centred card, no spotlight - see this file's header.
        return;
      }
      window.requestAnimationFrame(tick);
    };
    tick();
    return () => {
      done = true;
    };
  }, [step, location.pathname]);

  // Keep the spotlight on the element if the window moves under it.
  useEffect(() => {
    if (!step || !box) return;
    const recompute = () => {
      for (const key of step.anchors ?? []) {
        const el = document.querySelector(`[data-tour="${key}"]`);
        if (el) {
          const r = el.getBoundingClientRect();
          if (r.height > 0) {
            setBox({ left: r.left, top: r.top, width: r.width, height: r.height });
            return;
          }
        }
      }
    };
    window.addEventListener("resize", recompute);
    window.addEventListener("scroll", recompute, true);
    return () => {
      window.removeEventListener("resize", recompute);
      window.removeEventListener("scroll", recompute, true);
    };
  }, [step, box]);

  useEffect(() => {
    if (!active) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish();
      else if (e.key === "ArrowRight") setI((v) => Math.min(STEPS.length - 1, v + 1));
      else if (e.key === "ArrowLeft") setI((v) => Math.max(0, v - 1));
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [active, finish]);

  if (!active || !step) return null;

  const last = i === STEPS.length - 1;
  const pad = 8;

  // Card placement: under the highlight if it fits, over it if not, centred
  // when there is no highlight at all. Clamped so it can never hang off the
  // edge of a small window - marko runs this app at less than full screen.
  const CARD_W = 384;
  const below = box ? box.top + box.height + pad + 16 : 0;
  const roomBelow = box ? window.innerHeight - below - 200 > 0 : false;
  const cardStyle: CSSProperties = box
    ? {
        position: "fixed",
        width: CARD_W,
        left: Math.max(16, Math.min(window.innerWidth - CARD_W - 16, box.left + box.width / 2 - CARD_W / 2)),
        ...(roomBelow
          ? { top: below }
          : { bottom: Math.max(16, window.innerHeight - box.top + pad + 12) }),
      }
    : {
        position: "fixed",
        width: CARD_W,
        left: Math.max(16, window.innerWidth / 2 - CARD_W / 2),
        top: Math.max(16, window.innerHeight / 2 - 130),
      };

  return (
    <>
      {/* Blocks the app underneath. When there IS a highlight the dimming is
          done by the ring's own huge spread shadow (one element, real cutout),
          so this layer stays transparent and only catches clicks. */}
      <div
        className="fixed inset-0 z-[90]"
        style={{ background: box ? "transparent" : "rgba(2,6,23,0.66)" }}
      />
      {box && (
        <div
          className="pointer-events-none fixed z-[91] rounded-xl"
          style={{
            left: box.left - pad,
            top: box.top - pad,
            width: box.width + pad * 2,
            height: box.height + pad * 2,
            boxShadow: "0 0 0 9999px rgba(2,6,23,0.66)",
            outline: "2px solid rgb(100,131,249)",
            outlineOffset: "0px",
            transition: "left 220ms ease, top 220ms ease, width 220ms ease, height 220ms ease",
          }}
        />
      )}

      <div
        style={cardStyle}
        className="z-[92] rounded-2xl border border-slate-200 bg-surface p-5 shadow-raised dark:border-slate-700 dark:bg-slate-900"
      >
        <div className="flex items-center justify-between gap-3">
          <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-brand-600 dark:text-brand-400">
            Step {i + 1} of {STEPS.length}
          </p>
          <button
            type="button"
            onClick={finish}
            className="text-xs font-medium text-slate-400 underline-offset-2 hover:underline dark:text-slate-500"
          >
            End tour
          </button>
        </div>
        <h3 className="mt-2 text-[15px] font-semibold text-slate-900 dark:text-slate-50">{step.title}</h3>
        <p className="mt-1.5 text-[13px] leading-relaxed text-slate-600 dark:text-slate-300">{step.body}</p>

        {/* Progress as a hairline rather than a row of dots - fourteen dots in
            a 384px card is a decoration, not information. */}
        <div className="mt-4 h-[3px] w-full overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800">
          <div
            className="h-full rounded-full bg-brand-500 transition-[width] duration-300"
            style={{ width: `${((i + 1) / STEPS.length) * 100}%` }}
          />
        </div>

        <div className="mt-4 flex items-center justify-between gap-2">
          <button
            type="button"
            disabled={i === 0}
            onClick={() => setI((v) => Math.max(0, v - 1))}
            className="rounded-lg px-3 py-1.5 text-xs font-medium text-slate-500 hover:bg-slate-100 disabled:opacity-40 dark:text-slate-400 dark:hover:bg-slate-800"
          >
            Back
          </button>
          <button
            type="button"
            onClick={() => (last ? finish() : setI((v) => v + 1))}
            className="rounded-lg bg-brand-600 px-4 py-1.5 text-xs font-semibold text-white hover:bg-brand-700"
          >
            {last ? "Done" : "Next"}
          </button>
        </div>
      </div>
    </>
  );
}
