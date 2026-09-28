\---

name: Obsidian Frost

colors:

&#x20; surface: '#131316'

&#x20; surface-dim: '#131316'

&#x20; surface-bright: '#39393c'

&#x20; surface-container-lowest: '#0e0e11'

&#x20; surface-container-low: '#1b1b1e'

&#x20; surface-container: '#1f1f22'

&#x20; surface-container-high: '#2a2a2d'

&#x20; surface-container-highest: '#353438'

&#x20; on-surface: '#e4e1e6'

&#x20; on-surface-variant: '#c7c4d7'

&#x20; inverse-surface: '#e4e1e6'

&#x20; inverse-on-surface: '#303033'

&#x20; outline: '#908fa0'

&#x20; outline-variant: '#464554'

&#x20; surface-tint: '#c0c1ff'

&#x20; primary: '#c0c1ff'

&#x20; on-primary: '#1000a9'

&#x20; primary-container: '#8083ff'

&#x20; on-primary-container: '#0d0096'

&#x20; inverse-primary: '#494bd6'

&#x20; secondary: '#89ceff'

&#x20; on-secondary: '#00344d'

&#x20; secondary-container: '#00a2e6'

&#x20; on-secondary-container: '#00344e'

&#x20; tertiary: '#4edea3'

&#x20; on-tertiary: '#003824'

&#x20; tertiary-container: '#00885d'

&#x20; on-tertiary-container: '#000703'

&#x20; error: '#ffb4ab'

&#x20; on-error: '#690005'

&#x20; error-container: '#93000a'

&#x20; on-error-container: '#ffdad6'

&#x20; primary-fixed: '#e1e0ff'

&#x20; primary-fixed-dim: '#c0c1ff'

&#x20; on-primary-fixed: '#07006c'

&#x20; on-primary-fixed-variant: '#2f2ebe'

&#x20; secondary-fixed: '#c9e6ff'

&#x20; secondary-fixed-dim: '#89ceff'

&#x20; on-secondary-fixed: '#001e2f'

&#x20; on-secondary-fixed-variant: '#004c6e'

&#x20; tertiary-fixed: '#6ffbbe'

&#x20; tertiary-fixed-dim: '#4edea3'

&#x20; on-tertiary-fixed: '#002113'

&#x20; on-tertiary-fixed-variant: '#005236'

&#x20; background: '#131316'

&#x20; on-background: '#e4e1e6'

&#x20; surface-variant: '#353438'

typography:

&#x20; headline-lg:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 20px

&#x20;   fontWeight: '600'

&#x20;   lineHeight: 28px

&#x20;   letterSpacing: -0.015em

&#x20; headline-md:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 16px

&#x20;   fontWeight: '600'

&#x20;   lineHeight: 24px

&#x20;   letterSpacing: -0.01em

&#x20; headline-sm:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 14px

&#x20;   fontWeight: '600'

&#x20;   lineHeight: 20px

&#x20;   letterSpacing: -0.005em

&#x20; body-md:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 13px

&#x20;   fontWeight: '400'

&#x20;   lineHeight: 18px

&#x20;   letterSpacing: 0em

&#x20; body-sm:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 12px

&#x20;   fontWeight: '400'

&#x20;   lineHeight: 16px

&#x20;   letterSpacing: 0.005em

&#x20; label-mono-lg:

&#x20;   fontFamily: JetBrains Mono

&#x20;   fontSize: 12px

&#x20;   fontWeight: '500'

&#x20;   lineHeight: 16px

&#x20;   letterSpacing: -0.01em

&#x20; label-mono-sm:

&#x20;   fontFamily: JetBrains Mono

&#x20;   fontSize: 11px

&#x20;   fontWeight: '500'

&#x20;   lineHeight: 14px

&#x20;   letterSpacing: 0em

&#x20; badge-shortcut:

&#x20;   fontFamily: Inter

&#x20;   fontSize: 10px

&#x20;   fontWeight: '600'

&#x20;   lineHeight: 12px

&#x20;   letterSpacing: 0.04em

rounded:

&#x20; sm: 0.25rem

&#x20; DEFAULT: 0.5rem

&#x20; md: 0.75rem

&#x20; lg: 1rem

&#x20; xl: 1.5rem

&#x20; full: 9999px

spacing:

&#x20; gutter: 0.5rem

&#x20; margin: 1rem

&#x20; space-xs: 0.25rem

&#x20; space-sm: 0.5rem

&#x20; space-md: 0.75rem

&#x20; space-lg: 1rem

&#x20; space-xl: 1.5rem

\---



\## Brand \& Style



The design system embodies the hyper-focused precision of a high-performance desktop utility, synthesized with the dimensional depth of contemporary operating system surfaces (Windows 11 Mica material and macOS vibrancy). It treats the user's screen as an interactive stage, receding into an atmospheric dim while elevating capture workflows into tactile, floating instruments.



The aesthetic fuses \*\*Glassmorphism/Acrylic\*\* with \*\*Minimalist Precision Utility\*\*:

\- \*\*Tone:\*\* Professional, frictionless, surgical, and responsive.

\- \*\*Experience:\*\* Floating palettes feel weighted yet ethereal, balancing deep obsidian blacks with luminous frosted borders.

\- \*\*Interaction Metaphor:\*\* Optical hardware instruments—viewfinders, reticles, precision handles, and calibrated color pickers. Interactions feel instantaneous, snap-aligned, and visually non-destructive.



\## Colors



The palette operates in strict tiers to maintain contrast over variable desktop wallpapers and multi-window environments.



\### Overlay \& Foundation

\- \*\*Canvas Dim Backdrop:\*\* `rgba(10, 12, 16, 0.72)`—applied over unselected desktop areas to dim distracting content.

\- \*\*Surface Floating Acrylic:\*\* `rgba(24, 24, 27, 0.88)` to `rgba(30, 30, 36, 0.92)` combined with backdrop filtering.

\- \*\*Surface Inset/Elevated:\*\* `rgba(39, 39, 42, 0.65)` for segmented controls, toggle tracks, and input nests.



\### Accents \& Selection

\- \*\*Primary Indigo Glow (`#6366F1` / `#4F46E5`):\*\* Reserved for the crop bounding box, active tool pill states, primary download/upload triggers, and focus rings.

\- \*\*Precision Blue (`#0EA5E9`):\*\* Dimension callouts, pixel coordinates HUD, and snapping alignment guide-lines.

\- \*\*Status Accents:\*\* Emerald (`#10B981`) for capture confirmation/clipboard copy; Amber (`#F59E0B`) for screen recording timer; Crimson (`#EF4444`) for recording tally and cancel cues.



\### Translucent Borders

\- \*\*Glass Highlight Rim:\*\* `rgba(255, 255, 255, 0.12)` on top-lit borders.

\- \*\*Glass Ambient Rim:\*\* `rgba(255, 255, 255, 0.06)` on bottom/lateral borders.

\- \*\*Subtle Hairline Divider:\*\* `rgba(255, 255, 255, 0.08)`.



\### Typography Colors

\- \*\*Text High Contrast:\*\* `#FFFFFF` (Headings, primary tool icons, active shortcuts).

\- \*\*Text Body/Readable:\*\* `#E4E4E7` (Labels, tooltips, dimensions).

\- \*\*Text Muted/Secondary:\*\* `#A1A1AA` (Inactive icons, key combos, auxiliary hints).



\## Typography



The typography system relies on \*\*Inter\*\* for clean, neutral screen legibility at dense desktop scales, paired with \*\*JetBrains Mono\*\* for all telemetry, pixel dimension readouts, HEX/RGB values, and timecodes.



\- \*\*Scale Optimization:\*\* Built strictly around compact desktop viewports (10px–20px) to prevent HUD tools from occluding captured media.

\- \*\*Numbers \& Telemetry:\*\* All numeric readouts must use monospaced figures (`tabular-nums`) to prevent jitter during live window dragging, area resizing, and loupe navigation.

\- \*\*Key Caps \& Badges:\*\* Shortcuts use uppercase rendering with tight letter-spacing and heavy weight (`600`) within dedicated pill containers.



\## Layout \& Spacing



Because this system governs floating overlays, contextual HUDs, and fixed-ratio capture viewports, standard responsive webpage columns do not apply. Instead, layout relies on an anchor-and-dock system pinned to selection perimeters or screen corners.



\### Anchor Rules

\- \*\*Selection Bounding Box:\*\* Primary anchor point. Floating bars dynamically mount `12px` below the bottom edge of the crop selection. If the selection touches the bottom viewport edge, the bar flips to float inside or `12px` above the top edge.

\- \*\*HUD Reticle / Loupe:\*\* Offsets `16px` diagonally from the cursor pointer to avoid obscuring the target coordinate.

\- \*\*Corner Snapping:\*\* Secondary utility controls (history tray, pinned pins) dock to screen margins with `16px` padding from the display edge.



\### Component Spacing Rhythm

\- \*\*Micro Gaps (`space-xs` = 4px):\*\* Gaps between tool buttons within a segmented group; padding inside keycap chips.

\- \*\*Standard Tool Gap (`space-sm` = 8px):\*\* Spacing between distinct functional tool blocks and divider boundaries.

\- \*\*Panel Gaps (`space-md` = 12px):\*\* Popover-to-parent anchor spacing; margins within annotation property sheets.



\## Elevation \& Depth



Visual hierarchy is constructed via optical refraction, composite blur layers, and directional specular rims rather than stark drop shadows.



\### Glass Stack (Mica-Inspired Acrylic)

1\. \*\*Desktop Dimming Layer (Base):\*\* `background: rgba(10, 12, 16, 0.72)`. No blur applied to allow context visibility under dimming.

2\. \*\*Loupe \& Zoom Lens:\*\* `backdrop-filter: blur(0px)` with 8x-16x hardware scale transform, framed by an inner ring shadow `inset 0 0 0 1px rgba(255, 255, 255, 0.25)` and an outer rim shadow `0 8px 32px rgba(0, 0, 0, 0.6)`.

3\. \*\*Floating Toolbars \& Action Bars (Level 1 Elevation):\*\*

&#x20;  - Background: `rgba(24, 24, 27, 0.88)`

&#x20;  - Backdrop Filter: `blur(16px) saturate(180%)`

&#x20;  - Border: `1px solid rgba(255, 255, 255, 0.10)`

&#x20;  - Box Shadow: `0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 12px 24px -4px rgba(0, 0, 0, 0.5), inset 0 1px 0 0 rgba(255, 255, 255, 0.12)`

4\. \*\*Contextual Flyouts \& Color Palettes (Level 2 Elevation):\*\*

&#x20;  - Background: `rgba(30, 30, 36, 0.94)`

&#x20;  - Backdrop Filter: `blur(24px) saturate(200%)`

&#x20;  - Border: `1px solid rgba(255, 255, 255, 0.14)`

&#x20;  - Box Shadow: `0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 24px 48px -8px rgba(0, 0, 0, 0.7), inset 0 1px 0 0 rgba(255, 255, 255, 0.16)`

5\. \*\*Crop Selection Box:\*\*

&#x20;  - Border: `1.5px solid #6366F1`

&#x20;  - Drop Shadow: `0 0 0 1px rgba(0, 0, 0, 0.4), 0 0 16px rgba(99, 102, 241, 0.35)`



\## Shapes



The geometry uses a balanced curvature hierarchy designed to feel soft without compromising mechanical precision.



\- \*\*Main Toolbars \& Action Pods:\*\* `12px` to `16px` border-radius (`rounded-xl`), creating continuous pill-like floating pods.

\- \*\*Interactive Tool Icons \& Switches:\*\* `8px` (`rounded-md`), conforming tightly to square hit-targets (`32x32px` or `36x36px`).

\- \*\*Shortcut Badges \& Color Swatches:\*\* Fully circular or continuous pill shapes (`9999px`) to distinguish meta-elements from operational tools.

\- \*\*Crop Resizing Handles:\*\* Precision nodes at all 8 cardinal/intercardinal coordinates:

&#x20; - Width/Height: `8px x 8px` squares with a `2px` micro-radius.

&#x20; - White solid fill (`#FFFFFF`) framed by a `1.5px` border in `#6366F1` and a `1px` outer dark glow to ensure crisp visibility over pure white or pitch black screen backgrounds.



\## Components



\### Floating Primary Toolbar

\- \*\*Container:\*\* Horizontal acrylic capsule, `padding: 6px`, height `48px`, background `rgba(24, 24, 27, 0.88)` with `blur(16px)` and specular border highlight.

\- \*\*Divider:\*\* Vertical hairlines (`1px x 20px`), color `rgba(255, 255, 255, 0.08)`, spacing tool clusters (Select, Annotate, Redact, Export).

\- \*\*Tool Item:\*\* `36x36px` square button, radius `8px`. Inactive state is transparent with `#A1A1AA` icon. Hover state is `rgba(255, 255, 255, 0.06)` with `#FFFFFF` icon. Active state features a vibrant indigo tint (`rgba(99, 102, 241, 0.22)`), border `1px solid rgba(99, 102, 241, 0.5)`, icon color `#FFFFFF`, and an inner subtle blue glow.



\### Loupe / Magnifier HUD

\- \*\*Geometry:\*\* Circular HUD (`120px` diameter) or rounded squircle (`120x120px`, radius `24px`).

\- \*\*Reticle:\*\* Centered crosshair with a single clear 1-pixel target node.

\- \*\*Data Footprint:\*\* Pinned to bottom of the magnifier: monospaced badge containing the exact cursor coordinate `X: 1420 Y: 840` and the current hovered pixel HEX value (`#6366F1`), accompanied by a live 8px circular swatch.



\### Keyboard Shortcut Chips

\- \*\*Structure:\*\* Inset micro-capsule.

\- \*\*Styling:\*\* Background `rgba(255, 255, 255, 0.08)`, border `1px solid rgba(255, 255, 255, 0.12)`, radius `4px`, font `Inter` 10px uppercase, font weight `600`, text color `#A1A1AA`. Displayed within tooltips or directly adjacent to action labels (e.g., `Enter`, `Esc`, `⌘C`).



\### Annotation Popover (Stroke, Arrow, Blur, Highlighting)

\- \*\*Flyout Anchor:\*\* Vertically aligned to active tool, offset by `8px`.

\- \*\*Palette Grid:\*\* Pre-configured swatches (Crimson `#EF4444`, Amber `#F59E0B`, Emerald `#10B981`, Sky `#0EA5E9`, Violet `#8B5CF6`, White `#FFFFFF`, Dark `#18181B`). Swatches are `20px` circles. Active swatch features a white micro-check or concentric outer ring with a `2px` offset.

\- \*\*Slider Track:\*\* Inset `4px` height rounded track, filled with `#6366F1`, equipped with a `14px` white draggable circular thumb casting a soft elevation shadow.



\### Quick Action Bar (Post-Capture)

\- \*\*Positioning:\*\* Mounted directly below the active selection box or in the bottom-right workspace corner.

\- \*\*Action Buttons:\*\*

&#x20; - \*\*Copy (Primary):\*\* Solid background `#6366F1`, hover `#4F46E5`, white text and icon, radius `8px`.

&#x20; - \*\*Save / Upload:\*\* Secondary glass button, `rgba(255, 255, 255, 0.08)` fill, hover `rgba(255, 255, 255, 0.14)`.

&#x20; - \*\*Dismiss (Close):\*\* Subtle icon button with red hover tint (`rgba(239, 68, 68, 0.15)`).

