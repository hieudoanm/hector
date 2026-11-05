/* tslint:disable */
/* eslint-disable */

/**
 * An RGBA8 image buffer, row-major, 4 bytes per pixel.
 *
 * Construct from a `Uint8ClampedArray` (or any `Uint8Array`) of length
 * `width * height * 4`, or decode PNG/JPEG bytes with `WasmRaster.from_bytes`.
 */
export class WasmRaster {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Raw RGBA8 bytes, row-major (a `Uint8Array` from JS).
     */
    bytes(): Uint8Array;
    /**
     * Decode PNG or JPEG bytes into a raster.
     */
    static from_bytes(bytes: Uint8Array | number[]): WasmRaster;
    constructor(width: number, height: number, pixels: Uint8Array | number[]);
    readonly height: number;
    readonly width: number;
}

/**
 * RGBA color exposed to JavaScript.
 */
export class WasmRgba {
    free(): void;
    [Symbol.dispose](): void;
    constructor(r: number, g: number, b: number, a: number);
    /**
     * Opaque RGB color (alpha defaults to 255).
     */
    static rgb(r: number, g: number, b: number): WasmRgba;
    /**
     * `#rrggbb`, or `#rrggbbaa` when translucent.
     */
    to_hex(): string;
    a: number;
    b: number;
    g: number;
    r: number;
}

/**
 * SVG serialization options exposed to JavaScript.
 */
export class WasmSvgOptions {
    free(): void;
    [Symbol.dispose](): void;
    constructor();
    /**
     * Emit `width`/`height` attributes alongside `viewBox`.
     */
    explicit_size: boolean;
    /**
     * Decimal places kept on every coordinate.
     */
    precision: number;
    /**
     * Put each path on its own line.
     */
    pretty: boolean;
}

/**
 * Tracing configuration exposed to JavaScript. Defaults match
 * [`TraceConfig::default`].
 */
export class WasmTraceConfig {
    free(): void;
    [Symbol.dispose](): void;
    constructor();
    /**
     * Backdrop composited under transparent pixels.
     */
    backdrop: WasmRgba;
    /**
     * Palette index treated as background and never traced.
     */
    background_index: number;
    /**
     * Maximum Bézier deviation in pixels; higher means fewer curves.
     */
    bezier_tolerance: number;
    /**
     * Colors in the palette. `1` or `2` means pure black and white.
     */
    colors: number;
    /**
     * Run a median filter over the raster before labelling (needed for JPEG).
     */
    denoise: boolean;
    /**
     * Regions smaller than this are dropped as anti-aliasing noise.
     */
    min_area: number;
    /**
     * Shortest contour worth emitting.
     */
    min_contour_points: number;
    /**
     * Douglas–Peucker tolerance in pixels; higher means fewer points.
     */
    simplify_tolerance: number;
    /**
     * Luminance cut between background and foreground in binary mode.
     */
    threshold: number;
}

/**
 * The result of a trace: an SVG document plus the run's statistics.
 */
export class WasmTraceResult {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly stats: WasmTraceStats;
    readonly svg: string;
}

/**
 * Collected counts from a trace, exposed to JavaScript.
 */
export class WasmTraceStats {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly colors: number;
    readonly contours: number;
    readonly curves: number;
    readonly holes: number;
    readonly points_after_simplify: number;
    readonly points_before_simplify: number;
    readonly regions: number;
}

/**
 * Trace a raster and return `{ svg, stats }`.
 *
 * Rejects (throws) on invalid configuration or an undecodable image.
 */
export function trace(raster: WasmRaster, config: WasmTraceConfig): WasmTraceResult;

/**
 * Trace a raster with custom SVG options and return `{ svg, stats }`.
 */
export function trace_with_options(raster: WasmRaster, config: WasmTraceConfig, options: WasmSvgOptions): WasmTraceResult;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmraster_free: (a: number, b: number) => void;
    readonly __wbg_wasmrgba_free: (a: number, b: number) => void;
    readonly __wbg_wasmsvgoptions_free: (a: number, b: number) => void;
    readonly __wbg_wasmtraceconfig_free: (a: number, b: number) => void;
    readonly __wbg_wasmtraceresult_free: (a: number, b: number) => void;
    readonly __wbg_wasmtracestats_free: (a: number, b: number) => void;
    readonly trace: (a: number, b: number) => [number, number, number];
    readonly trace_with_options: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmraster_bytes: (a: number) => [number, number];
    readonly wasmraster_from_bytes: (a: number, b: number) => [number, number, number];
    readonly wasmraster_height: (a: number) => number;
    readonly wasmraster_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmraster_width: (a: number) => number;
    readonly wasmrgba_a: (a: number) => number;
    readonly wasmrgba_b: (a: number) => number;
    readonly wasmrgba_g: (a: number) => number;
    readonly wasmrgba_new: (a: number, b: number, c: number, d: number) => number;
    readonly wasmrgba_r: (a: number) => number;
    readonly wasmrgba_rgb: (a: number, b: number, c: number) => number;
    readonly wasmrgba_set_a: (a: number, b: number) => void;
    readonly wasmrgba_set_b: (a: number, b: number) => void;
    readonly wasmrgba_set_g: (a: number, b: number) => void;
    readonly wasmrgba_set_r: (a: number, b: number) => void;
    readonly wasmrgba_to_hex: (a: number) => [number, number];
    readonly wasmsvgoptions_explicit_size: (a: number) => number;
    readonly wasmsvgoptions_new: () => number;
    readonly wasmsvgoptions_precision: (a: number) => number;
    readonly wasmsvgoptions_pretty: (a: number) => number;
    readonly wasmsvgoptions_set_explicit_size: (a: number, b: number) => void;
    readonly wasmsvgoptions_set_precision: (a: number, b: number) => void;
    readonly wasmsvgoptions_set_pretty: (a: number, b: number) => void;
    readonly wasmtraceconfig_backdrop: (a: number) => number;
    readonly wasmtraceconfig_background_index: (a: number) => number;
    readonly wasmtraceconfig_bezier_tolerance: (a: number) => number;
    readonly wasmtraceconfig_colors: (a: number) => number;
    readonly wasmtraceconfig_denoise: (a: number) => number;
    readonly wasmtraceconfig_min_area: (a: number) => number;
    readonly wasmtraceconfig_min_contour_points: (a: number) => number;
    readonly wasmtraceconfig_new: () => number;
    readonly wasmtraceconfig_set_backdrop: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_background_index: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_bezier_tolerance: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_colors: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_denoise: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_min_area: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_min_contour_points: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_simplify_tolerance: (a: number, b: number) => void;
    readonly wasmtraceconfig_set_threshold: (a: number, b: number) => void;
    readonly wasmtraceconfig_simplify_tolerance: (a: number) => number;
    readonly wasmtraceconfig_threshold: (a: number) => number;
    readonly wasmtraceresult_stats: (a: number) => number;
    readonly wasmtraceresult_svg: (a: number) => [number, number];
    readonly wasmtracestats_colors: (a: number) => number;
    readonly wasmtracestats_contours: (a: number) => number;
    readonly wasmtracestats_curves: (a: number) => number;
    readonly wasmtracestats_holes: (a: number) => number;
    readonly wasmtracestats_points_after_simplify: (a: number) => number;
    readonly wasmtracestats_points_before_simplify: (a: number) => number;
    readonly wasmtracestats_regions: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
