# Native application behavior

NPZSee is a general-purpose, read-only NumPy array viewer. It indexes NPZ entries
before loading any array and renders Table by default. Array selection, file
loading and analysis use generation IDs so stale work cannot replace newer data.

Table navigation descends through leading dimensions until a matrix or vector
is reached. Rows are virtualized; 10,000-row and 16-column pages leave all values
reachable. Clicking a value opens a resizable detail pane inside Table. Content
is not truncated for the full-copy operation. JSON object/array nodes expand,
large JSON arrays paginate, and individual JSON values open the same pane.
Selecting a new array or navigating a slice clears the previous cell selection.

Plot uses min/max envelopes sized to the viewport and revisits visible original
samples when zooming index plots. Histogram has a filled area, sampled previews
with explicit labels and opt-in exact scans. Log-count fill reaches the lower
visible edge. SVG export matches the filled histogram.

Six UI languages are bundled. Toolbar changes apply immediately and are saved
in the per-user NPZSee configuration directory. A command-line --language override
does not alter the saved language. Bidi runs keep Arabic shaping separate from
Latin/numeric runs, using egui's HarfRust shaper; source text remains unchanged.
Low-level diagnostic details may be in English. The overall layout and numeric
array index order remain consistent across languages.

See README for supported formats and cross-platform.md for desktop requirements.
