# Vectify Optimization

## Purpose

Vectify converts raster images such as PNG/JPG into SVG.

The goal of optimization is not simply to make the SVG visually similar to the original image.

The goal is to find a good balance between:

1. **Visual accuracy**
2. **Geometric simplicity**
3. **SVG size**
4. **Processing speed**

The core optimization question is:

> What is the simplest vector representation that reproduces the important visual information in the original raster image?

---

# 1. The Optimization Loop

Every optimization experiment should follow this loop:

```text
Original PNG
     │
     ▼
  Vectify
     │
     ▼
    SVG
     │
     ▼
 SVG Renderer
     │
     ▼
Reconstructed PNG
     │
     ▼
   Compare
     │
     ▼
 Metrics + Difference Image
     │
     ▼
 Identify Problem
     │
     ▼
Modify Algorithm
     │
     └───────────────► Repeat
```

The agent should evaluate the **actual output of the CLI**, rather than relying only on source-code reasoning.

---

# 2. Baseline Before Optimization

Before changing an algorithm, establish a baseline.

For every test image:

```text
input.png
    ↓
vectify
    ↓
output.svg
    ↓
render output.svg → reconstructed.png
    ↓
compare input.png and reconstructed.png
```

Record:

- reconstruction error
- perceptual similarity
- SVG file size
- number of paths
- number of Bézier curves
- processing time

Example:

```text
Image: circle.png

Pixel error:        4.21%
SSIM:               0.962
SVG size:           3.8 KB
Paths:              1
Bezier curves:      18
Processing time:    42 ms
```

The baseline provides a reference for every subsequent change.

---

# 3. Golden Test Images

Maintain a small collection of representative images.

Suggested categories:

```text
tests/images/
├── basic/
│   ├── circle.png
│   ├── square.png
│   ├── triangle.png
│   └── star.png
│
├── curves/
│   ├── circle.png
│   ├── ellipse.png
│   └── wave.png
│
├── logos/
│   ├── simple-logo.png
│   └── complex-logo.png
│
├── colors/
│   ├── two-color.png
│   ├── palette.png
│   └── gradient.png
│
├── holes/
│   ├── ring.png
│   └── letter.png
│
└── real-world/
    ├── drawing.png
    └── illustration.png
```

Start with simple synthetic images.

Do not begin optimization with photographs.

Photographs introduce problems that are fundamentally different from clean vector-like artwork.

---

# 4. The Reconstruction Pipeline

The optimization system should treat SVG as an intermediate representation.

PNG
│
▼
Preprocessing
│
▼
Color Quantization
│
▼
Segmentation
│
▼
Regions
│
▼
Contour Extraction
│
▼
Contour Simplification
│
▼
Bézier Fitting
│
▼
Vector Representation
│
▼
SVG
│
▼
resvg
│
▼
Reconstructed PNG

---

## 4.1 SVG Rendering

Use **resvg** as the primary SVG → PNG renderer for automated evaluation.

The evaluation pipeline is:

Original PNG
↓
Vectify
↓
SVG
↓
resvg
↓
Reconstructed PNG
↓
Image comparison
↓
Metrics

resvg is used because it is Rust-based, deterministic, and suitable for
CLI-based automated testing.

---

# 5. Image Comparison

The simplest comparison is pixel difference.

For each pixel:

```text
difference = |original - reconstructed|
```

This can produce:

- mean absolute error
- mean squared error
- maximum error
- percentage of significantly different pixels

However, pixel error alone is not sufficient.

A one-pixel boundary shift may produce a large pixel difference while being visually insignificant.

---

# 6. Perceptual Metrics

Use multiple metrics when possible.

Potential metrics include:

| Metric           | Purpose                       |
| ---------------- | ----------------------------- |
| MAE              | Average pixel difference      |
| MSE              | Penalizes larger errors       |
| PSNR             | Signal reconstruction quality |
| SSIM             | Structural similarity         |
| Edge similarity  | Boundary preservation         |
| Color difference | Color preservation            |

Do not rely on a single metric.

A good optimization result should generally improve visual quality without dramatically increasing geometric complexity.

---

# 7. Difference Images

Generate a visual difference image.

Conceptually:

```text
Original             Reconstructed          Difference

████████             ████████              ░░░░░░░░
████████             ███████░              ░░░░░░▓░
████████             ███████░              ░░░░░░▓░
████████             ███████░              ░░░░░░▓░
```

The difference image is useful for identifying:

- incorrect boundaries
- missing regions
- incorrect colors
- holes that were lost
- excessive smoothing
- excessive simplification
- Bézier fitting errors

An agent should inspect the difference image whenever possible.

---

# 8. Complexity Must Be Part of the Objective

Pure reconstruction accuracy is not the goal.

For example:

```text
Original
   ↓
1000 Bézier curves
   ↓
Very accurate reconstruction
```

This may be worse than:

```text
Original
   ↓
50 Bézier curves
   ↓
Slightly less accurate reconstruction
```

The second representation may be much more useful as a vector image.

Therefore optimization should consider complexity.

Useful complexity measurements include:

- number of paths
- number of Bézier curves
- number of points
- SVG file size
- number of colors
- number of control points

---

# 9. Optimization Objective

A conceptual objective function is:

```text
Loss =
    reconstruction_error
    + λ × geometric_complexity
    + μ × file_size
```

Where:

```text
reconstruction_error
```

measures visual difference.

```text
geometric_complexity
```

measures how complicated the vector representation is.

```text
file_size
```

measures the resulting SVG size.

`λ` and `μ` control the importance of simplicity and file size.

The exact objective should evolve as Vectify develops.

---

# 10. Bézier Curve Optimization

Bézier fitting is one of the most important optimization targets.

Given a contour:

```text
P0 P1 P2 P3 P4 ... Pn
```

try to represent it using a small number of cubic Bézier curves.

The basic strategy is:

```text
Contour
   │
   ▼
Try one Bézier curve
   │
   ▼
Calculate fitting error
   │
   ├── Error acceptable ──► Keep curve
   │
   └── Error too large
             │
             ▼
        Find worst point
             │
             ▼
           Split
             │
       ┌─────┴─────┐
       ▼           ▼
   Fit curve    Fit curve
       │           │
       └─────┬─────┘
             ▼
           Repeat
```

The main parameters are:

- fitting tolerance
- maximum curve count
- splitting strategy
- parameterization method
- control-point estimation

---

# 11. Tolerance Experiments

Tolerance is one of the most important Vectify parameters.

Run experiments such as:

```text
Tolerance    Curves    Error
--------------------------------
0.5          420       1.2%
1.0          230       1.8%
2.0          130       2.4%
3.0           82       3.1%
5.0           51       4.7%
```

The objective is not automatically to select the lowest error.

Look for a useful tradeoff between:

```text
accuracy
    ↕
complexity
```

---

# 12. Parameter Sweeps

When a parameter is important, test multiple values automatically.

For example:

```text
tolerance = 0.5
tolerance = 1.0
tolerance = 1.5
tolerance = 2.0
tolerance = 3.0
tolerance = 5.0
```

For each experiment:

1. Run Vectify.
2. Generate SVG.
3. Render SVG to PNG.
4. Compare against original.
5. Record metrics.
6. Save the result.
7. Compare against baseline.

This creates an empirical understanding of the algorithm.

---

# 13. Stage-by-Stage Optimization

Do not optimize the entire pipeline at once.

Optimize individual stages.

## Preprocessing

Investigate:

- image resizing
- noise removal
- smoothing
- alpha handling
- color space

Questions:

> Does preprocessing remove useful information?

> Does preprocessing make contours easier to detect?

---

## Color Quantization

Investigate:

- number of colors
- clustering method
- color distance
- handling of small color regions

Example:

```text
Original: 16 million colors

↓ quantization

8 colors
↓
16 colors
↓
32 colors
↓
64 colors
```

Compare the resulting vector complexity and reconstruction quality.

---

## Segmentation

Investigate:

- connected components
- region merging
- small-region removal
- adjacency
- color similarity

Potential failure:

```text
One object
   ↓
Too many regions
   ↓
Too many paths
```

---

## Contour Extraction

Investigate:

- contour accuracy
- boundary connectivity
- diagonal handling
- holes
- nested contours

The contour is extremely important because later stages cannot recover information that was lost here.

---

## Contour Simplification

Investigate:

- Douglas–Peucker tolerance
- alternative simplification strategies
- preservation of corners
- preservation of curvature

Potential failure:

```text
Too much simplification
        ↓
important shape information disappears
```

---

## Bézier Fitting

Investigate:

- fitting tolerance
- curve splitting
- parameterization
- control-point estimation
- corner detection
- closed-curve handling

This is likely to be one of the highest-value optimization areas.

---

# 14. Debugging by Rendering Every Stage

When an output is wrong, save intermediate representations.

For example:

```text
debug/
├── 01-original.png
├── 02-quantized.png
├── 03-segments.png
├── 04-contours.png
├── 05-simplified.png
├── 06-bezier.png
├── 07-final.svg
├── 08-reconstructed.png
└── 09-difference.png
```

This allows an agent to determine:

```text
Where did the error first appear?
```

For example:

```text
Original        ✓
Quantization   ✓
Segmentation   ✓
Contours       ✗
Simplification ✓
Bézier         ✓
```

The optimization target is then the contour algorithm rather than the Bézier algorithm.

---

# 15. Agent Optimization Workflow

Coding agents working on Vectify should follow this procedure.

### Step 1 — Establish baseline

Run the current implementation.

### Step 2 — Measure

Collect:

- visual metrics
- SVG size
- curve count
- path count
- runtime

### Step 3 — Inspect

Look at:

- original image
- reconstructed image
- difference image
- intermediate debug images

### Step 4 — Identify one problem

Do not change multiple unrelated algorithms simultaneously.

Example:

> Curved boundaries contain too many Bézier segments.

### Step 5 — Form a hypothesis

Example:

> The Bézier fitting tolerance is too conservative.

### Step 6 — Make one focused change

Change only the relevant algorithm or parameter.

### Step 7 — Run the complete pipeline

```text
PNG
→ Vectify
→ SVG
→ resvg
→ PNG
→ comparison
```

### Step 8 — Compare against baseline

Determine whether the change:

- improved reconstruction
- reduced complexity
- increased complexity
- changed runtime
- introduced regressions

### Step 9 — Keep or revert

Only keep changes supported by measurements.

### Step 10 — Repeat

Build improvements incrementally.

---

# 16. Never Optimize Against a Single Image

An algorithm can overfit a particular test image.

For example:

```text
circle.png
    ↓
Algorithm change
    ↓
Excellent
```

but:

```text
star.png
    ↓
Algorithm change
    ↓
Terrible
```

Therefore every meaningful optimization should be tested against a test suite.

Conceptually:

```text
                 ┌── circle
                 ├── square
Algorithm ───────┼── star
                 ├── logo
                 ├── holes
                 └── curves
```

An improvement should ideally improve the overall test suite without introducing significant regressions.

---

# 17. Regression Testing

Every optimization should preserve previously working behavior.

Maintain baseline results for important images.

Example:

```text
Image       Baseline    Current    Status
------------------------------------------------
circle      2.1%        1.8%       improved
square      1.4%        1.5%       acceptable
star        3.2%        2.4%       improved
ring        2.8%        5.1%       regression
```

A regression should trigger investigation.

Do not optimize one category while silently breaking another.

---

# 18. Optimization Experiments

Keep experiments reproducible.

Each experiment should record:

```text
Experiment
-----------
Date:
Git commit:
Input:
Algorithm change:
Parameters:
Pixel metric:
Perceptual metric:
Curve count:
Path count:
SVG size:
Runtime:
Result:
Notes:
```

This creates an experimental history for Vectify.

---

# 19. Determinism

Vectify should preferably produce deterministic results.

For the same:

```text
input
+
configuration
+
version
```

the result should be the same.

Avoid uncontrolled randomness.

If an algorithm eventually requires randomness, use a configurable seed.

This makes optimization experiments reproducible.

---

# 20. Performance Optimization

Accuracy should be optimized before low-level performance.

First establish:

```text
correctness
    ↓
quality
    ↓
complexity
    ↓
performance
```

Once the algorithm is stable, profile:

- image loading
- color quantization
- segmentation
- contour extraction
- contour simplification
- Bézier fitting
- SVG serialization

Only optimize code paths supported by profiling data.

---

# 21. Future Automated Optimization

Once the evaluation pipeline works, Vectify can automatically search parameter space.

For example:

```text
                 ┌── tolerance
                 ├── color count
                 ├── simplification
Parameters ──────┼── segmentation threshold
                 └── Bézier fitting threshold
                         │
                         ▼
                      Vectify
                         │
                         ▼
                    Render SVG
                         │
                         ▼
                       Loss
                         │
                         ▼
                  Parameter search
                         │
                         └──────► repeat
```

Possible approaches include:

- grid search
- random search
- Bayesian optimization
- evolutionary optimization
- gradient-based optimization where applicable

Do not introduce these systems until the basic evaluation pipeline is reliable.

---

# 22. Future Differentiable Optimization

A longer-term research direction is:

```text
PNG
 ↓
Vector parameters
 ↓
Differentiable renderer
 ↓
Rendered image
 ↓
Loss
 ↓
Gradient
 ↓
Update vector parameters
```

This could eventually allow Vectify to optimize Bézier control points directly against the original raster image.

This is substantially more advanced and should be treated as a future research direction rather than part of V1.

---

# 23. The Important Distinction

Vectify should distinguish between:

### Algorithm correctness

Does the algorithm correctly perform its intended operation?

Example:

> Does contour extraction correctly follow the boundary?

### Reconstruction quality

Does the final SVG resemble the original image?

Example:

> Does the rendered SVG preserve the shape?

### Representation efficiency

Does it represent the image with a small amount of geometry?

Example:

> Can 20 Bézier curves replace 100?

### Performance

Can the algorithm do this quickly?

Example:

> Can a 4000×4000 image be processed efficiently?

These are different optimization dimensions.

---

# 24. Optimization Priority

Use this general order:

```text
1. Correctness
       ↓
2. End-to-end reconstruction
       ↓
3. Major visual errors
       ↓
4. Bézier quality
       ↓
5. Complexity reduction
       ↓
6. Color accuracy
       ↓
7. Edge cases
       ↓
8. Performance
       ↓
9. Advanced optimization
```

Do not prematurely optimize performance while the geometry is incorrect.

---

# 25. V1 Optimization Goal

V1 should be able to take simple vector-like raster images:

```text
PNG
 ↓
Vectify
 ↓
SVG
 ↓
PNG
```

and produce a reconstruction that:

- preserves the major shapes
- preserves major colors
- preserves holes
- has reasonably accurate boundaries
- uses a reasonable number of Bézier curves
- produces valid SVG
- behaves deterministically

Perfect reconstruction is not required.

The objective is to establish a strong measurable baseline.

---

# 26. Guiding Principle

The optimization system should always ask:

> **Did the change actually make Vectify better?**

Not:

> Does the code look better?

Not:

> Does the algorithm sound better?

Not:

> Does the SVG source look better?

The evidence should come from the complete pipeline:

```text
PNG
 ↓
Vectify
 ↓
SVG
 ↓
Rasterize
 ↓
PNG
 ↓
Compare
 ↓
Measure
```

The rendered result is the ground truth for evaluating the current implementation.

---

# 27. Long-Term Vision

Vectify can eventually become more than a raster-to-SVG converter.

The architecture can evolve into an optimization system:

```text
                 Original Raster
                       │
                       ▼
              ┌─────────────────┐
              │    Vectify      │
              │                 │
              │ segmentation    │
              │ contours        │
              │ simplification  │
              │ Bézier fitting  │
              └────────┬────────┘
                       │
                       ▼
                      SVG
                       │
                       ▼
                  Rasterizer
                       │
                       ▼
                Reconstructed
                    Raster
                       │
              ┌────────┴────────┐
              ▼                 ▼
        Visual Metrics      Complexity
              │                 │
              └────────┬────────┘
                       ▼
                     Loss
                       │
                       ▼
                  Optimization
                       │
                       └──────────► Vectify
```

The ultimate goal is:

> **Find a compact geometric representation that preserves the important information contained in the original raster image.**

That principle should guide future algorithmic and architectural decisions.

---

# 28. Competent Optimization Protocol

The built-in `sweep` command varies one parameter while holding all other
settings fixed. Its result is conditional on that configuration; it is not a
global optimum. Use sweeps to find promising ranges, then measure combinations
explicitly.

## 28.1 Search in Two Passes

1. **Record an exact baseline.** Save the input, command/configuration, metrics,
   geometry counts, SVG bytes, and rendered artifacts.
2. **Sweep stage families.** First choose the active color mode: test
   `threshold` for binary tracing (`colors` 1 or 2), or `colors` for palette
   tracing (`colors` greater than 2). Then test `min-area` in either mode, and
   explore geometric controls such as `simplify-tolerance` and
   `bezier-tolerance` against promising configurations.
3. **Refine around useful values.** Run a coarse sweep, then test smaller steps
   around the best and the Pareto-efficient results. Do not assume the response
   is smooth: integer region cutoffs and palette splits can change the output
   abruptly.
4. **Measure combinations directly.** For every candidate worth keeping, run
   `eval` with the complete explicit configuration. A parameter sweep does not
   test interactions with parameters it is holding fixed.
5. **Inspect the candidate render and difference image.** Metrics locate a
   change; artifacts explain whether it is a color, missing-region, or boundary
   error.
6. **Keep a Pareto set.** Compare MAE, SSIM, edge similarity, primitives, and
   bytes together. A candidate is dominated when another is at least as good on
   every metric and better on one. Choose among the remaining trade-offs for
   the use case; the combined loss is a ranking aid, not a substitute for
   inspecting the component metrics.
7. **Guard shared algorithm changes with the golden suite.** A per-image SVG
   may use explicit settings tuned for that image. Do not change crate-wide
   defaults based on one logo; check the golden suite and record any deliberate
   baseline changes.

## 28.2 Treat Region Filtering as a Quality Parameter

`min-area` often changes reconstruction more than curve tolerance. Lowering it
can preserve anti-aliased edge fragments and small details, while multiplying
the number of regions and paths. Sweep values around the current setting,
including `0` or `1` when full detail is requested, and measure the cost. Never
infer the best value from the name of a detail preset alone.

Likewise, test color counts around a promising value rather than assuming that
more colors improve the result. Median-cut splits and nearest-swatch assignment
are discrete; an intermediate count can outperform both a smaller and a larger
palette.

## 28.3 Record Reproducible Commands

For a shipped example, record the source image, exact tracing flags, renderer,
and report. Keep its settings local to that artifact. A report that says only
“tolerance 0.8” is incomplete if color count, detail, and minimum area also
affect the output.

### Example: VietinBank SVG Variants

These measurements are for the 500×129
`examples/images/vietinbank.png`, rendered by the eval harness with resvg. They
illustrate three different objectives, not a universal default. The monochrome
variant cannot reproduce the source palette, so compare its structural scores
and complexity as well as its higher pixel error.

| Output                | Explicit settings                                                                        |    MAE |   SSIM |   Edge | Primitives | SVG bytes |
| --------------------- | ---------------------------------------------------------------------------------------- | -----: | -----: | -----: | ---------: | --------: |
| `vietinbank.svg`      | `--colors 6 --detail full --min-area 4 --simplify-tolerance 0.97 --bezier-tolerance 0.1` | 0.650% | 0.9824 | 0.9891 |        605 |    11,313 |
| `vietinbank-bold.svg` | `--colors 5 --detail bold --simplify-tolerance 1.4`                                      | 1.085% | 0.9570 | 0.9796 |        161 |     2,390 |
| `vietinbank-bw.svg`   | `--colors 2 --threshold 111 --detail full`                                               | 9.333% | 0.8085 | 0.9645 |        212 |     2,845 |

Compared with the previous measured examples, the full-color variant reduced
MAE from 0.737% to 0.650%, primitives from 827 to 605, and SVG size from 13,938
to 11,313 bytes. The bold variant reduced MAE from 1.096% to 1.085% with one
additional primitive. For the monochrome variant, threshold 111 gave the best
pixel and structural scores in the narrow 106–112 sweep; threshold 108 used
four fewer primitives with nearly the same MAE. That is a real trade-off, so
retain the chosen threshold with its reason instead of presenting it as an
absolute optimum.

The full-color result has a nearby Pareto alternative: lowering `min-area` to
3 scores 0.639% MAE and 0.9827 SSIM, with 725 primitives and a 13,521-byte SVG.
The recorded `min-area 4` result gives up 0.011 percentage points of MAE and
0.0003 SSIM while saving 120 primitives and 2,208 bytes. Keep the more compact
point for the example; choose the lower-area point when the extra fidelity is
worth the size.

The sweeps also exposed interactions that single-parameter tuning missed:
with 6 colors, lowering the minimum area and tightening Bézier tolerance
improved the full-color result. Below a 0.2 px Bézier tolerance, the score
stopped changing for this candidate, so a still tighter fit was unnecessary.

Reproduce a row by passing its flags to both the converter and evaluator. For
example:

```bash
cargo run --release -- \
  --colors 6 --detail full --min-area 4 \
  --simplify-tolerance 0.97 --bezier-tolerance 0.1 \
  ../../examples/images/vietinbank.png ../../examples/svg/vietinbank.svg

cargo run --release --features eval -- \
  --colors 6 --detail full --min-area 4 \
  --simplify-tolerance 0.97 --bezier-tolerance 0.1 \
  eval ../../examples/images/vietinbank.png
```

### Example: HSBC with Transparent Artwork

`examples/optimization/original/hsbc.svg` contains both transparent canvas and
an opaque white shape inside the mark. Rendering it onto white before tracing
merges those two meanings. Render with alpha preserved, then choose a near-white
backdrop key that is distinct from the opaque white fill. Inspect the palette
with `vectify info` and set `background-index` to the key's palette index so
only the canvas is omitted from the output SVG.

At 4× scale the PNG is 1569×340. With a `#f7f7f7` key, the 7-color palette puts
the key at index 1 and retains white, red, and black artwork. The measured
candidate is 0.428% MAE, 0.9779 SSIM, 0.9916 edge similarity, 195 primitives,
and 3,009 SVG bytes. The metrics compare against the source composited onto the
same `#f7f7f7` key; edge colors therefore describe that backdrop.

```bash
rsvg-convert --zoom 4 \
  --output ../../examples/optimization/png/hsbc.png \
  ../../examples/optimization/original/hsbc.svg

cargo run --release -- \
  --colors 7 --detail full --background-index 1 --backdrop '#f7f7f7' \
  --min-area 384 --simplify-tolerance 1.2 --bezier-tolerance 0.75 \
  ../../examples/optimization/png/hsbc.png \
  ../../examples/optimization/svg/hsbc.svg

cargo run --release --features eval -- \
  --colors 7 --detail full --background-index 1 --backdrop '#f7f7f7' \
  --min-area 384 --simplify-tolerance 1.2 --bezier-tolerance 0.75 \
  eval ../../examples/optimization/png/hsbc.png
```
