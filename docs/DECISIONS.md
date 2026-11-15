# Decision log

Short records of why things are the way they are. Add new entries at the bottom; never rewrite history, supersede instead.

## 0001 - Rust for the core

**Decision:** The engine is written in Rust.
**Why:** One implementation can compile to WASM (browser and phone), a Python module (photo lab) and a command-line tool, which removes the need to keep two codebases in agreement. It is also fast enough for per-pixel and radiative-transfer work later, and the compiler's error messages are good for learning.
**Cost:** Heavier toolchain, and fewer contributors know Rust than JavaScript. Mitigated by keeping most contribution surface in packs and the UI.
**Escape hatch:** The core is small. If WASM friction becomes a problem it can be ported to TypeScript, checked against golden test vectors.

## 0002 - Static-first app, no backend

**Decision:** The public app is fully static; live data is fetched directly from the browser.
**Why:** Free hosting, no operations, easy to fork, works offline in the field.

## 0003 - Parameter packs instead of hardcoded estimates

**Decision:** Estimated numbers (stage boundaries, cloud heights) live in versioned JSON packs.
**Why:** Corrections ship as small data updates instead of code releases, regions can differ, and photo-derived calibration has somewhere to land.
**Rules:** Numbers only, never code. A `schema` field is mandatory and unknown versions are rejected.

## 0004 - Vue 3 + TypeScript for the front end

**Decision:** Vue 3 (Composition API) with TypeScript and Vite, built as a PWA.
**Why:** Familiar from coursework, small and fast on phones, good dev tooling for the planned debug panels.

## 0005 - Python lab with SQLite and notebooks

**Decision:** Photo analysis and calibration live in a Python package using SQLite and Jupyter notebooks.
**Why:** The dataset (hundreds of photos) is small and tabular, notebooks make the analysis visible and teachable, and SQLite is a single file you can query with plain SQL.

## 0006 - Licence: MIT for code

**Decision:** MIT for the repository for now.
**Why:** Simple and permissive for an open-source tool. Pack data may later get its own licence (for example CC0 or CC-BY) if community contributions begin; Apache-2.0 can be added as a dual licence then.

## 0007 - Tests use independent expected values

**Decision:** Tests check against known astronomical facts and hand calculations, not outputs copied from the implementation.
**Why:** A test that only repeats the code's answer cannot catch a wrong formula.
