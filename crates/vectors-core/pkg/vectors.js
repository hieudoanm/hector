/* @ts-self-types="./vectors.d.ts" */

/**
 * An RGBA8 image buffer, row-major, 4 bytes per pixel.
 *
 * Construct from a `Uint8ClampedArray` (or any `Uint8Array`) of length
 * `width * height * 4`, or decode PNG/JPEG bytes with `WasmRaster.from_bytes`.
 */
export class WasmRaster {
    static __wrap(ptr) {
        const obj = Object.create(WasmRaster.prototype);
        obj.__wbg_ptr = ptr;
        WasmRasterFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRasterFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmraster_free(ptr, 0);
    }
    /**
     * Raw RGBA8 bytes, row-major (a `Uint8Array` from JS).
     * @returns {Uint8Array}
     */
    bytes() {
        const ret = wasm.wasmraster_bytes(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Decode PNG or JPEG bytes into a raster.
     * @param {Uint8Array | number[]} bytes
     * @returns {WasmRaster}
     */
    static from_bytes(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmraster_from_bytes(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmRaster.__wrap(ret[0]);
    }
    /**
     * @returns {number}
     */
    get height() {
        const ret = wasm.wasmraster_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} width
     * @param {number} height
     * @param {Uint8Array | number[]} pixels
     */
    constructor(width, height, pixels) {
        const val2 = +width;
        const val3 = +height;
        const ptr0 = passArray8ToWasm0(pixels, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmraster_new(val2, val3, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmRasterFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    get width() {
        const ret = wasm.wasmraster_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) WasmRaster.prototype[Symbol.dispose] = WasmRaster.prototype.free;

/**
 * RGBA color exposed to JavaScript.
 */
export class WasmRgba {
    static __wrap(ptr) {
        const obj = Object.create(WasmRgba.prototype);
        obj.__wbg_ptr = ptr;
        WasmRgbaFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRgbaFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmrgba_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get a() {
        const ret = wasm.wasmrgba_a(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get b() {
        const ret = wasm.wasmrgba_b(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get g() {
        const ret = wasm.wasmrgba_g(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @param {number} a
     */
    constructor(r, g, b, a) {
        const ret = wasm.wasmrgba_new(r, g, b, a);
        this.__wbg_ptr = ret;
        WasmRgbaFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    get r() {
        const ret = wasm.wasmrgba_r(this.__wbg_ptr);
        return ret;
    }
    /**
     * Opaque RGB color (alpha defaults to 255).
     * @param {number} r
     * @param {number} g
     * @param {number} b
     * @returns {WasmRgba}
     */
    static rgb(r, g, b) {
        const ret = wasm.wasmrgba_rgb(r, g, b);
        return WasmRgba.__wrap(ret);
    }
    /**
     * @param {number} v
     */
    set a(v) {
        wasm.wasmrgba_set_a(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set b(v) {
        wasm.wasmrgba_set_b(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set g(v) {
        wasm.wasmrgba_set_g(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set r(v) {
        wasm.wasmrgba_set_r(this.__wbg_ptr, v);
    }
    /**
     * `#rrggbb`, or `#rrggbbaa` when translucent.
     * @returns {string}
     */
    to_hex() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmrgba_to_hex(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) WasmRgba.prototype[Symbol.dispose] = WasmRgba.prototype.free;

/**
 * SVG serialization options exposed to JavaScript.
 */
export class WasmSvgOptions {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSvgOptionsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsvgoptions_free(ptr, 0);
    }
    /**
     * Emit `width`/`height` attributes alongside `viewBox`.
     * @returns {boolean}
     */
    get explicit_size() {
        const ret = wasm.wasmsvgoptions_explicit_size(this.__wbg_ptr);
        return ret !== 0;
    }
    constructor() {
        const ret = wasm.wasmsvgoptions_new();
        this.__wbg_ptr = ret;
        WasmSvgOptionsFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Decimal places kept on every coordinate.
     * @returns {number}
     */
    get precision() {
        const ret = wasm.wasmsvgoptions_precision(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Put each path on its own line.
     * @returns {boolean}
     */
    get pretty() {
        const ret = wasm.wasmsvgoptions_pretty(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {boolean} v
     */
    set explicit_size(v) {
        wasm.wasmsvgoptions_set_explicit_size(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set precision(v) {
        wasm.wasmsvgoptions_set_precision(this.__wbg_ptr, v);
    }
    /**
     * @param {boolean} v
     */
    set pretty(v) {
        wasm.wasmsvgoptions_set_pretty(this.__wbg_ptr, v);
    }
}
if (Symbol.dispose) WasmSvgOptions.prototype[Symbol.dispose] = WasmSvgOptions.prototype.free;

/**
 * Tracing configuration exposed to JavaScript. Defaults match
 * [`TraceConfig::default`].
 */
export class WasmTraceConfig {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmTraceConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmtraceconfig_free(ptr, 0);
    }
    /**
     * Backdrop composited under transparent pixels.
     * @returns {WasmRgba}
     */
    get backdrop() {
        const ret = wasm.wasmtraceconfig_backdrop(this.__wbg_ptr);
        return WasmRgba.__wrap(ret);
    }
    /**
     * Palette index treated as background and never traced.
     * @returns {number}
     */
    get background_index() {
        const ret = wasm.wasmtraceconfig_background_index(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Maximum Bézier deviation in pixels; higher means fewer curves.
     * @returns {number}
     */
    get bezier_tolerance() {
        const ret = wasm.wasmtraceconfig_bezier_tolerance(this.__wbg_ptr);
        return ret;
    }
    /**
     * Colors in the palette. `1` or `2` means pure black and white.
     * @returns {number}
     */
    get colors() {
        const ret = wasm.wasmtraceconfig_colors(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Run a median filter over the raster before labelling (needed for JPEG).
     * @returns {boolean}
     */
    get denoise() {
        const ret = wasm.wasmtraceconfig_denoise(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Regions smaller than this are dropped as anti-aliasing noise.
     * @returns {number}
     */
    get min_area() {
        const ret = wasm.wasmtraceconfig_min_area(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Shortest contour worth emitting.
     * @returns {number}
     */
    get min_contour_points() {
        const ret = wasm.wasmtraceconfig_min_contour_points(this.__wbg_ptr);
        return ret >>> 0;
    }
    constructor() {
        const ret = wasm.wasmtraceconfig_new();
        this.__wbg_ptr = ret;
        WasmTraceConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {WasmRgba} v
     */
    set backdrop(v) {
        _assertClass(v, WasmRgba);
        var ptr0 = v.__destroy_into_raw();
        wasm.wasmtraceconfig_set_backdrop(this.__wbg_ptr, ptr0);
    }
    /**
     * @param {number} v
     */
    set background_index(v) {
        wasm.wasmtraceconfig_set_background_index(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set bezier_tolerance(v) {
        wasm.wasmtraceconfig_set_bezier_tolerance(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set colors(v) {
        wasm.wasmtraceconfig_set_colors(this.__wbg_ptr, v);
    }
    /**
     * @param {boolean} v
     */
    set denoise(v) {
        wasm.wasmtraceconfig_set_denoise(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set min_area(v) {
        wasm.wasmtraceconfig_set_min_area(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set min_contour_points(v) {
        wasm.wasmtraceconfig_set_min_contour_points(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set simplify_tolerance(v) {
        wasm.wasmtraceconfig_set_simplify_tolerance(this.__wbg_ptr, v);
    }
    /**
     * @param {number} v
     */
    set threshold(v) {
        wasm.wasmtraceconfig_set_threshold(this.__wbg_ptr, v);
    }
    /**
     * Douglas–Peucker tolerance in pixels; higher means fewer points.
     * @returns {number}
     */
    get simplify_tolerance() {
        const ret = wasm.wasmtraceconfig_simplify_tolerance(this.__wbg_ptr);
        return ret;
    }
    /**
     * Luminance cut between background and foreground in binary mode.
     * @returns {number}
     */
    get threshold() {
        const ret = wasm.wasmtraceconfig_threshold(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmTraceConfig.prototype[Symbol.dispose] = WasmTraceConfig.prototype.free;

/**
 * The result of a trace: an SVG document plus the run's statistics.
 */
export class WasmTraceResult {
    static __wrap(ptr) {
        const obj = Object.create(WasmTraceResult.prototype);
        obj.__wbg_ptr = ptr;
        WasmTraceResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmTraceResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmtraceresult_free(ptr, 0);
    }
    /**
     * @returns {WasmTraceStats}
     */
    get stats() {
        const ret = wasm.wasmtraceresult_stats(this.__wbg_ptr);
        return WasmTraceStats.__wrap(ret);
    }
    /**
     * @returns {string}
     */
    get svg() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmtraceresult_svg(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) WasmTraceResult.prototype[Symbol.dispose] = WasmTraceResult.prototype.free;

/**
 * Collected counts from a trace, exposed to JavaScript.
 */
export class WasmTraceStats {
    static __wrap(ptr) {
        const obj = Object.create(WasmTraceStats.prototype);
        obj.__wbg_ptr = ptr;
        WasmTraceStatsFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmTraceStatsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmtracestats_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get colors() {
        const ret = wasm.wasmtracestats_colors(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get contours() {
        const ret = wasm.wasmtracestats_contours(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get curves() {
        const ret = wasm.wasmtracestats_curves(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get holes() {
        const ret = wasm.wasmtracestats_holes(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get points_after_simplify() {
        const ret = wasm.wasmtracestats_points_after_simplify(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get points_before_simplify() {
        const ret = wasm.wasmtracestats_points_before_simplify(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get regions() {
        const ret = wasm.wasmtracestats_regions(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) WasmTraceStats.prototype[Symbol.dispose] = WasmTraceStats.prototype.free;

/**
 * Trace a raster and return `{ svg, stats }`.
 *
 * Rejects (throws) on invalid configuration or an undecodable image.
 * @param {WasmRaster} raster
 * @param {WasmTraceConfig} config
 * @returns {WasmTraceResult}
 */
export function trace(raster, config) {
    _assertClass(raster, WasmRaster);
    _assertClass(config, WasmTraceConfig);
    const ret = wasm.trace(raster.__wbg_ptr, config.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return WasmTraceResult.__wrap(ret[0]);
}

/**
 * Trace a raster with custom SVG options and return `{ svg, stats }`.
 * @param {WasmRaster} raster
 * @param {WasmTraceConfig} config
 * @param {WasmSvgOptions} options
 * @returns {WasmTraceResult}
 */
export function trace_with_options(raster, config, options) {
    _assertClass(raster, WasmRaster);
    _assertClass(config, WasmTraceConfig);
    _assertClass(options, WasmSvgOptions);
    const ret = wasm.trace_with_options(raster.__wbg_ptr, config.__wbg_ptr, options.__wbg_ptr);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return WasmTraceResult.__wrap(ret[0]);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_4c88d8bf264fd984: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./vectors_bg.js": import0,
    };
}

const WasmRasterFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmraster_free(ptr, 1));
const WasmRgbaFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmrgba_free(ptr, 1));
const WasmSvgOptionsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsvgoptions_free(ptr, 1));
const WasmTraceConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmtraceconfig_free(ptr, 1));
const WasmTraceResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmtraceresult_free(ptr, 1));
const WasmTraceStatsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmtracestats_free(ptr, 1));

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('vectors_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
