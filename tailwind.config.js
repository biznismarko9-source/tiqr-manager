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
        // 2.34.0 (Vapor): marko picked the Vapor direction out of sixteen.
        // The lavender is now pink. 600 stays "the action colour" every
        // `bg-brand-600` in the app already uses, and is chosen for white-on-it
        // contrast (~4.6:1) rather than for the brightest possible pink - the
        // saturated end of the ramp lives at 400/500, which is what dark mode
        // surfaces and where it is text-on-dark rather than text-on-accent.
        // 2.36.0: marko picked the violet out of the preview's accent
        // control - hue 253, the Onyx-lavender family, a step deeper than
        // 2.34.0's pink. Same ladder, same roles, one hue moved.
        //
        // Two steps are tuned rather than generated, and both for contrast,
        // not taste: 600 is the action colour that carries WHITE text
        // (`bg-brand-600`) and lands at 9.1:1, and 400 is what dark mode
        // uses for links and inline actions (`dark:text-brand-400`) where a
        // straight -3 lightness shift measured 4.05:1 on the card surface -
        // under the 4.5 floor. It sits at 5.3:1 now. Re-measure both if this
        // ramp is ever regenerated.
        brand: {
          50: "#e4ddfd",
          100: "#d5cbfb",
          200: "#bcacf6",
          300: "#ac99f0",
          400: "#9077e9",
          500: "#5f3fd5",
          600: "#492cb5",
          700: "#3e2791",
          800: "#311f6f",
          900: "#221358",
          950: "#140c32",
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
        // 2.34.0 (Vapor): same ladder, same lightness steps, hue pulled off
        // blue-grey onto violet so the pink accent sits in its own family
        // instead of fighting a cold ground. 400 keeps its old lightness on
        // purpose - it is the muted-text step and it cleared 4.5:1 there.
        // 3.0: the neutral ladder is re-cut for the new language. Two
        // things changed and both are deliberate.
        //
        // The DARK end drops much further and de-blues: 950 is the app
        // ground (#08080b), 900 is the card that sits ABOVE it by tone, and
        // 800 is a hairline rather than a border. That tonal step is what
        // replaces the flat 1px-box look - depth now comes from surface
        // lightness, not from outlining every element.
        //
        // The MIDS lose most of their violet so the accent is the only real
        // colour on screen. 400 is the muted-text step and moved from 4.44:1
        // to 4.56:1 on white - it was under the 4.5 floor and is not any
        // more. On the dark card it reads 4.16:1; see CURRENT_STATE.md for
        // why that one cannot be fixed here (no single value clears 4.5 on
        // both grounds - the class pairing has to differ per mode, and most
        // of the app currently pairs them the wrong way round).
        slate: {
          50: "#f4f4f8",
          100: "#e6e6ee",
          200: "#d0d0dc",
          300: "#adadbe",
          400: "#74748a",
          500: "#62626f",
          600: "#4b4b57",
          700: "#34343e",
          800: "#1e1e27",
          900: "#101016",
          950: "#08080b",
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
      // 2.34.1: back to exactly the stack that shipped in every version up to
      // 2.33.0 - marko asked for the old font back after seeing the mono one.
      //
      // Worth knowing rather than rediscovering: "Inter" at the head of this
      // list is not loaded anywhere. No @font-face, no bundled file, no
      // stylesheet link. Every screen has therefore always rendered in
      // system-ui, and this list behaves as if it started at "ui-sans-serif".
      // It is left in place because removing it would change nothing on
      // screen, and a webfont cannot be added without solving TIQR's
      // offline-by-default case first.
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
