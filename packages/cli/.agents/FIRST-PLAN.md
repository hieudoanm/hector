# Vectify — Raster → Vector

> **Status:** the whole pipeline described below is implemented and tested, plus
> an HTTP server (`vectify serve`) and a self-hosting `Dockerfile`. For how to
> run it, start at [docs/DOWNLOADS.md](docs/DOWNLOADS.md); for how it works,
> read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The rest of this file is
> the original design brief, kept as the specification.

```bash
cargo run --release -- logo.png logo.svg
cargo run --release -- logo.png logo.svg --colors 2 --detail bold
cargo run --release -- logo.png logo.svg --debug-dir ./trace

cargo run --release -- serve --bind 0.0.0.0:8080
docker build -t vectify . && docker run --rm -p 8080:8080 vectify
```

## 1. Project Goal

Build a Rust application that converts raster images such as:

- PNG
- JPG/JPEG

into SVG vector images.

The core approach is:

**Pixels → Regions → Contours → Simplified Contours → Bézier Curves → SVG**

The project should prioritize **understanding and implementing the algorithms**, rather than simply wrapping an existing image-tracing library.

---

# 2. Core Concept

A raster image consists of pixels:

```text
Image
 ↓
pixels
 ↓
each pixel has a color
```

Vectify needs to discover the larger structures represented by those pixels:

```text
pixels
 ↓
colors / regions
 ↓
boundaries
 ↓
curves
 ↓
vector paths
```

The fundamental transformation is:

> Convert a collection of discrete pixel samples into a compact mathematical representation of shapes.

---

# 3. Main Pipeline

```text
             PNG / JPG
                 │
                 ▼
          ┌─────────────┐
          │ Load Image  │
          └──────┬──────┘
                 │
                 ▼
              Pixels
                 │
                 ▼
          ┌─────────────┐
          │ Preprocess  │
          └──────┬──────┘
                 │
                 ▼
       Colors / simplified image
                 │
                 ▼
          ┌─────────────┐
          │ Segment     │
          │ Regions     │
          └──────┬──────┘
                 │
                 ▼
              Regions
                 │
                 ▼
          ┌─────────────┐
          │ Find        │
          │ Contours    │
          └──────┬──────┘
                 │
                 ▼
             Contours
                 │
                 ▼
          ┌─────────────┐
          │ Simplify    │
          │ Contours    │
          └──────┬──────┘
                 │
                 ▼
        Simplified points
                 │
                 ▼
          ┌─────────────┐
          │ Bézier      │
          │ Fitting     │
          └──────┬──────┘
                 │
                 ▼
           Bézier paths
                 │
                 ▼
          ┌─────────────┐
          │ SVG Output  │
          └──────┬──────┘
                 │
                 ▼
                SVG
```

---

# 4. Phase 1 — Raster Image

First establish the concept of a raster image.

Each pixel contains color information:

```text
Pixel
├── Red
├── Green
├── Blue
└── Alpha
```

The image is essentially a 2D grid of these pixels.

### Goal

Successfully load an image and access:

- width
- height
- pixel coordinates
- pixel colors

Don't perform vectorization yet.

---

# 5. Phase 2 — Start With Black & White

Do **not** immediately attempt arbitrary photographs.

Start with:

```text
black
white
```

Convert the image into a binary representation:

```text
foreground
background
```

For example:

```text
........
...###..
..#####.
.#######.
..#####.
...###..
........
```

This dramatically simplifies the problem.

### Goal

Given a black-and-white image, identify which pixels represent the foreground.

---

# 6. Phase 3 — Find Regions

Next determine which foreground pixels belong to the same object.

For example:

```text
..............
..####........
.######.......
..####........
..............
........###...
.......#####..
........###...
```

There are two separate regions.

The algorithm should discover:

```text
Region A
Region B
```

This is a **connected-component** problem.

### Goal

Turn:

```text
pixels
```

into:

```text
regions
```

---

# 7. Phase 4 — Find Contours

A region contains many pixels, but we don't need to keep all of them.

We want its boundary.

For example:

```text
    ####
  ########
 ##########
 ##########
  ########
    ####
```

becomes conceptually:

```text
       ┌────┐
    ┌──┘    └──┐
   │            │
    └──┐    ┌──┘
       └────┘
```

The boundary is called a **contour**.

Possible algorithms include:

- boundary tracing
- Marching Squares

### Goal

Convert:

```text
Region
```

into:

```text
ordered contour points
```

---

# 8. Phase 5 — Simplify the Contour

A raster boundary is noisy.

You might get:

```text
P1 P2 P3 P4 P5 P6 P7 ... P500
```

Many of those points are unnecessary.

Use a simplification algorithm such as **Douglas–Peucker**.

Conceptually:

```text
500 points
    ↓
100 points
    ↓
20 points
```

while maintaining approximately the same shape.

The user should be able to control the tolerance.

### Low tolerance

```text
more points
more detail
higher accuracy
```

### High tolerance

```text
fewer points
simpler geometry
more approximation
```

---

# 9. Phase 6 — Bézier Curves

This is the central geometric component.

Instead of representing a contour as hundreds of points, represent it using cubic Bézier curves.

A cubic Bézier contains:

```text
start point
control point
control point
end point
```

A complex contour could therefore become:

```text
500 pixels
   ↓
50 contour points
   ↓
8 Bézier curves
```

The important question is:

> How many Bézier curves are necessary to represent the contour accurately enough?

---

# 10. Bézier Fitting

The fitting algorithm should:

1. Take a section of the contour.
2. Try to represent it with one Bézier curve.
3. Measure the error.
4. If the error is acceptable, keep the curve.
5. If the error is too large, split the contour.
6. Fit each section separately.
7. Continue until the error is acceptable.

Conceptually:

```text
Contour
   │
   ▼
Try one curve
   │
   ├── error acceptable → keep
   │
   └── error too large
             │
             ▼
           split
          /     \
         /       \
      curve     curve
```

This is the most important algorithm in Vectify.

---

# 11. Accuracy vs Complexity

Vectify should expose a tolerance parameter.

Think of it as:

```text
                 Accuracy
                    ▲
                    │
                    │
             many curves
                    │
                    │
                    │
                    │
                    └──────────────► Simplicity
                              fewer curves
```

More curves:

- more accurate
- larger SVG
- more complex geometry

Fewer curves:

- smaller SVG
- simpler geometry
- potentially less accurate

This creates a useful optimization problem.

---

# 12. Phase 7 — Vector Representation

Once Bézier fitting works, create an internal vector representation.

Conceptually:

```text
Vector Image
│
├── Path
│   ├── Bézier
│   ├── Bézier
│   └── Bézier
│
├── Path
│   ├── Bézier
│   └── Bézier
│
└── Path
    └── Bézier
```

Each path should have properties such as:

- Bézier curves
- fill color
- optional stroke
- stroke width

This representation should **not depend on SVG**.

---

# 13. Phase 8 — SVG Generation

Only after the vector representation exists should SVG be generated.

Conceptually:

```text
Vector Path
     ↓
SVG path commands
     ↓
<path ... />
```

For example, the internal representation:

```text
Move
Cubic Bézier
Cubic Bézier
Cubic Bézier
Close
```

becomes an SVG `<path>`.

At this point, SVG generation should be relatively straightforward.

---

# 14. Phase 9 — Multiple Colors

Once black-and-white conversion works, introduce color.

The problem becomes:

```text
millions of possible colors
        ↓
simplified color palette
        ↓
regions
        ↓
contours
        ↓
Bézier curves
```

For example:

```text
Original
16,777,216 possible RGB colors

        ↓

Simplified
16 representative colors
```

This is called **color quantization**.

Possible approaches:

- K-means
- Median Cut
- Octree
- other clustering methods

Start with one approach and make it replaceable later.

---

# 15. Phase 10 — Holes

Eventually support shapes like:

```text
████████
██    ██
██    ██
████████
```

The middle is a hole.

The vector representation needs to understand:

```text
outer contour
     +
inner contour
```

This introduces:

- nested contours
- winding direction
- SVG fill rules

This is important for real-world logos and icons.

---

# 16. Phase 11 — Debugging Visualization

Build a way to inspect every stage.

Ideally Vectify can show:

```text
Original
    ↓
Quantized
    ↓
Regions
    ↓
Raw contours
    ↓
Simplified contours
    ↓
Bézier curves
    ↓
Final SVG
```

This will be extremely useful for development.

Instead of asking:

> "Why does the SVG look wrong?"

you can identify:

> "The contour extraction was wrong."

or:

> "The contour was correct, but Bézier fitting introduced too much error."

---

# 17. Phase 12 — CLI

The basic CLI should eventually look like:

```text
vectify input.png output.svg
```

Then add options such as:

```text
--colors
--tolerance
--threshold
--simplification
--debug
```

The CLI should be thin.

The actual algorithms should live in the library.

Two subcommands ship on top of that thin CLI: `info` reports an image's palette
without tracing it, and `serve` exposes the same library over HTTP
(`GET /health`, `GET /info`, `POST /vectify`) so other processes can reach the
tracer without shelling out.

---

# 18. Architecture

Keep the major concepts separate:

```text
Raster
  ↓
Preprocessing
  ↓
Segmentation
  ↓
Contours
  ↓
Simplification
  ↓
Geometry
  ↓
Vector Model
  ↓
Output
```

In particular:

**Raster ≠ Region ≠ Contour ≠ Bézier ≠ SVG**

Each represents a different abstraction level.

---

# 19. Rust Design

Rust is a good fit because the project involves:

- large arrays of pixels
- numerical computation
- geometry
- performance-sensitive algorithms
- deterministic processing
- possible parallelism later
- potential WASM support

Use Rust for the core engine.

Potential future interfaces:

```text
Rust Core
   │
   ├── CLI
   ├── WASM
   ├── Tauri
   └── Python bindings
```

---

# 20. Future ML Direction

**Do not start with ML.**

First create a strong classical baseline.

Then ML can replace individual components.

For example:

```text
Classical:

Image
 ↓
Color segmentation
 ↓
Contour
 ↓
Bézier fitting
 ↓
SVG
```

Later:

```text
Image
 ↓
Neural segmentation
 ↓
Contour
 ↓
Bézier fitting
 ↓
SVG
```

Eventually:

```text
Image
 ↓
Neural model
 ↓
Bézier representation
 ↓
SVG
```

The classical implementation becomes the baseline against which the ML approach can be evaluated.

---

# 21. Future Optimization Research

There is another interesting direction.

Instead of only fitting Bézier curves to contours, define:

```text
Original raster
       ↓
candidate vector
       ↓
render candidate
       ↓
compare with original
       ↓
loss
```

The objective could combine:

```text
image reconstruction error
+
number of curves
+
geometric complexity
```

This turns Vectify into an optimization problem:

> Find the simplest vector representation that reconstructs the raster image sufficiently well.

This could eventually connect the project to:

- computational geometry
- image compression
- optimization
- differentiable rendering
- neural vector graphics

---

# 22. Development Milestones

### Milestone 1 — Pixel Reader

```text
PNG → pixels
```

### Milestone 2 — Binary Image

```text
PNG → black/white
```

### Milestone 3 — Regions

```text
black/white → connected regions
```

### Milestone 4 — Contours

```text
regions → boundaries
```

### Milestone 5 — Simplification

```text
boundaries → simplified points
```

### Milestone 6 — Bézier

```text
points → Bézier curves
```

### Milestone 7 — SVG

```text
Bézier curves → SVG
```

### Milestone 8 — End-to-End

```text
PNG → SVG
```

### Milestone 9 — Color

```text
PNG → color regions → SVG
```

### Milestone 10 — Robustness

Support:

- holes
- complex shapes
- different image sizes
- transparency

### Milestone 11 — Performance

Profile and optimize the implementation.

### Milestone 12 — Advanced Research

Explore:

- optimization
- neural segmentation
- neural Bézier prediction
- differentiable rendering

---

# 23. Definition of a Successful V1

V1 does **not** need to perfectly vectorize photographs.

A successful V1 should take a simple image such as:

```text
simple logo
simple icon
simple drawing
```

and produce:

```text
PNG
 ↓
regions
 ↓
contours
 ↓
Bézier curves
 ↓
SVG
```

with the generated SVG looking recognizably like the original.

The important achievement is having the **entire pipeline working and understandable**.

---

# 24. Guiding Principle for Future Agents

When implementing Vectify:

> **Do not jump directly from pixels to SVG.**

Always think in layers:

```text
"What are the pixels?"
        ↓
"What regions do they form?"
        ↓
"What are the boundaries?"
        ↓
"Which boundary points matter?"
        ↓
"Which Bézier curves approximate those boundaries?"
        ↓
"How do those curves become SVG?"
```

Build each stage independently, test it, visualize it, and only then connect it to the next stage.

The long-term goal is not merely a PNG-to-SVG converter.

It is a **Rust implementation of raster-to-vector reconstruction**, starting with classical computer vision and computational geometry and eventually providing a foundation for optimization and machine-learning experiments.
