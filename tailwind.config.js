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
        // 2.29.0 (Onyx): the blue becomes the lavender marko picked. Same
        // job, same 600-is-the-action-colour convention - every `bg-brand-600`
        // across the pages picks this up with no page edit.
        // 2.29.1: a little more pigment. The first pass was so desaturated
        // that a primary button read as another grey panel - the accent is
        // the ONE saturated thing on screen and has to earn that.
        brand: {
          50: "#f1effc",
          100: "#dfdaf8",
          200: "#c7c0f1",
          300: "#ada3e9",
          400: "#9a8ee3",
          500: "#8878dc",
          600: "#7563cf",
          700: "#6151b4",
          800: "#4e418f",
          900: "#3e346f",
          950: "#271f47",
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
        // 2.29.0 (Onyx): retuned again, and this time the ramp is deliberately
        // FLATTER at the dark end. Soft UI is one material - the card and the
        // ground are the same sheet, separated by light rather than by a
        // colour step - so 900 and 950 sit two values apart instead of the
        // eight-value gap the 2.6.0 ramp used. The depth that step used to
        // carry now comes from the two-shadow pair in index.css.
        // The hue also loses the blue: these are near-neutral greys so the
        // lavender brand below is the only colour on screen.
        // 2.29.2: darker and higher-contrast, because the design is FLAT now -
        // with the lighting gone, every bit of separation has to come from the
        // fills and the lines themselves. The dark end drops further and the
        // ground-to-surface step opens back up; 800 is lifted so a hairline is
        // actually visible against 900, which is what now draws every edge.
        slate: {
          50: "#eff1f5",
          100: "#e4e7ed",
          200: "#d3d7e1",
          300: "#b6bbca",
          400: "#767b8e",
          500: "#5d6274",
          600: "#464b5c",
          700: "#313543",
          800: "#262935",
          900: "#15171d",
          950: "#0d0e12",
        },
        // 2.29.1: the surface a card actually sits on, as its own token
        // rather than a literal. `bg-white` was the single biggest reason the
        // real app looked nothing like the preview - 39 places painted a pure
        // white box on the new grey ground, which is the exact opposite of
        // "the card and the page are one sheet". These resolve to the same
        // `--surface` variables index.css already defines per theme, so one
        // class is correct in both.
        surface: "var(--surface)",
        "surface-muted": "var(--surface-muted)",
        "surface-raised": "var(--surface-raised)",
        "surface-sunken": "var(--surface-sunken)",
        // 2.29.3: the hairline, as a colour. Now that a 1px line draws every
        // edge in the flat design, `border-line` is worth having rather than
        // repeating `border-slate-200 dark:border-slate-800` each time.
        line: "var(--line)",
        "line-soft": "var(--line-soft)",
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
      // 2.29.0 (Onyx): the scale is now the neumorphic PAIR - one darker
      // shadow and one lighter highlight, both derived from the surface the
      // element sits on. That pair cannot be one fixed value, because it is
      // different in light and dark, so these resolve to variables set per
      // theme in index.css. The names and the card -> raised -> overlay
      // hierarchy are unchanged, so every existing `shadow-card` keeps
      // working and simply looks soft now.
      boxShadow: {
        card: "var(--sh-card)",
        raised: "var(--sh-raised)",
        overlay: "var(--sh-overlay)",
        inset: "var(--sh-inset)",
        // Focus ring used by inputs/selects/textareas - a soft brand halo
        // rather than Tailwind's hard 2px ring, so a focused field in a
        // dense form doesn't shout.
        focus: "0 0 0 3px rgb(136 120 220 / 0.34)",
        "focus-danger": "0 0 0 3px rgb(220 38 38 / 0.16)",
      },
      borderRadius: {
        // 2.6.0: the app's radius rhythm. Controls (buttons/inputs/badges)
        // sit at `lg`, containers (cards, table shells, modals) at `xl`.
        //
        // 2.29.0 (Onyx): both grew. This REVERSES marko's own 2.6.0
        // instruction about "obrovské rounded cards" - noted here so nobody
        // treats it as drift. He chose the Clay/Onyx preview, and a soft
        // extruded surface at a 10px radius reads as a mistake rather than a
        // material: the corner has to be round enough for the two shadows to
        // travel around it. The ratio between the two steps is unchanged.
        lg: "0.8125rem",
        xl: "1.25rem",
        "2xl": "1.5rem",
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
