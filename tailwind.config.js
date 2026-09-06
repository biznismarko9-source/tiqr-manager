/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // 2.0.58: reverted 2.0.56's "Brown & beige" back to the original
        // blue-accent/white-light/navy-dark look - marko tried the brown in
        // production and didn't like it. Both ramps below are full 11-stop
        // Tailwind-shaped scales (matching the same shape 2.0.56 used) so
        // every existing bg-/text-/border-/ring-/divide-/placeholder:-slate-N
        // and -brand-N class across the whole app picks up the reverted
        // colors automatically - same one-file mechanism that made 2.0.56
        // apply everywhere at once now makes reverting it a one-file change
        // too.
        //
        // 2.6.0 (visual redesign): this same one-file mechanism is what the
        // whole redesign's color work rides on - see the `slate` note below.
        // `brand` itself is DELIBERATELY UNCHANGED, byte for byte: 600
        // (#4a68f7) is the accent marko confirmed by name in 2.0.56 and kept
        // through the 2.0.58 revert, so the redesign treats it as the app's
        // fixed identity and only changes how much of it is used, never the
        // hue. Do not "modernize" this ramp without asking him first - a
        // palette change has already been tried and rejected once.
        brand: {
          50: "#eef4ff",
          100: "#d2e0ff",
          200: "#b6cbfe",
          300: "#9ab4fd",
          400: "#7f9cfb",
          500: "#6483f9",
          600: "#4a68f7",
          700: "#213fe9",
          800: "#1e30b5",
          900: "#1d277f",
          950: "#181c4d",
        },
        // 2.6.0 (visual redesign): retuned away from Tailwind's stock slate.
        // This is the one place the redesign's light/dark surface hierarchy
        // is defined - every `bg-slate-N`/`border-slate-N`/`text-slate-N`
        // already spelled out across ~23k lines of pages picks it up with no
        // page edit, the same mechanism 2.0.56/2.0.58 used.
        //
        // What changed and why:
        //   - The hue is pulled off Tailwind's fairly blue slate toward a
        //     quieter blue-grey, so the brand blue above is the only
        //     saturated color on screen. Same family, less competition.
        //   - Dark mode is no longer "inverted light": 950 (app background)
        //     and 900 (card/sidebar surface) are lifted and de-blued from
        //     #020617/#0f172a, giving a real background -> surface step
        //     instead of near-black on near-black, and 800/700 give borders
        //     that are actually visible on top of it.
        //   - 50/100 (light app background, hover fill) are warmed a touch
        //     off pure blue-white so large light surfaces don't read cold.
        //   - 400/500 (secondary + muted text) were nudged for contrast:
        //     both now clear 4.5:1 on their own surface in both modes.
        // Endpoints are no longer Tailwind's published defaults; that is
        // intentional and is the redesign, not drift.
        slate: {
          50: "#f7f8fa",
          100: "#eff1f5",
          200: "#e3e7ed",
          300: "#c8ced9",
          400: "#8d96a7",
          500: "#68717f",
          600: "#4c5563",
          700: "#333c4a",
          800: "#1d2430",
          900: "#11161f",
          950: "#080b11",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
      },
      // 2.6.0: one shadow scale for the whole app, replacing the mix of
      // Tailwind's stock shadow-sm/shadow-lg/shadow-xl. Stock shadows are
      // a single soft blur; these are the two-layer (tight contact shadow +
      // wider ambient) form that reads as real depth at small sizes without
      // looking heavy. Kept deliberately short: card -> raised -> overlay,
      // nothing else. Dark mode gets its own set (below) because a black
      // shadow on a dark surface is invisible - there, depth comes from a
      // subtle top highlight instead, applied via .card/.overlay in
      // index.css rather than from these utilities.
      boxShadow: {
        card: "0 1px 2px 0 rgb(15 23 42 / 0.04), 0 1px 3px 0 rgb(15 23 42 / 0.04)",
        raised:
          "0 1px 2px 0 rgb(15 23 42 / 0.05), 0 4px 12px -2px rgb(15 23 42 / 0.08)",
        overlay:
          "0 2px 4px -1px rgb(15 23 42 / 0.06), 0 12px 32px -8px rgb(15 23 42 / 0.18)",
        // Focus ring used by inputs/selects/textareas - a soft brand halo
        // rather than Tailwind's hard 2px ring, so a focused field in a
        // dense form doesn't shout.
        focus: "0 0 0 3px rgb(74 104 247 / 0.16)",
        "focus-danger": "0 0 0 3px rgb(220 38 38 / 0.16)",
      },
      borderRadius: {
        // 2.6.0: the app's radius rhythm. Controls (buttons/inputs/badges)
        // sit at `lg`, containers (cards, table shells, modals) at `xl`.
        // Both are SMALLER than Tailwind's stock values on purpose - marko
        // explicitly asked for no "obrovské rounded cards".
        lg: "0.5rem",
        xl: "0.625rem",
        "2xl": "0.875rem",
      },
      transitionDuration: {
        // 2.6.0: the app's motion budget - marko asked for 120-180ms and
        // nothing longer. DEFAULT is what an unqualified `transition`
        // resolves to, so every shared control lands inside that band
        // without each call site restating it.
        DEFAULT: "150ms",
        120: "120ms",
        150: "150ms",
        180: "180ms",
      },
      transitionTimingFunction: {
        DEFAULT: "cubic-bezier(0.32, 0.72, 0, 1)",
      },
    },
  },
  plugins: [],
};
