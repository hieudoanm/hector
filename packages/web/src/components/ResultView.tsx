import type { FC } from 'react';

export interface ResultViewProps {
  dataUrl: string;
  blobUrl: string;
  originalSize: number;
  svgSize: number;
  svg: string;
  onCopy: () => void;
  onReset: () => void;
}

const fmt = (b: number) =>
  b < 1024
    ? `${b} B`
    : b < 1048576
      ? `${(b / 1024).toFixed(1)} KB`
      : `${(b / 1048576).toFixed(2)} MB`;

export const ResultView: FC<ResultViewProps> = ({
  dataUrl,
  blobUrl,
  originalSize,
  svgSize,
  svg,
  onCopy,
  onReset,
}) => (
  <section className="mt-6 space-y-5">
    <div className="grid gap-4 sm:grid-cols-2">
      <figure className="rounded-2xl border border-slate-200 bg-white p-4">
        <figcaption className="mb-2 flex items-center justify-between text-sm font-medium">
          Original PNG
          <span className="text-xs font-normal text-slate-500">
            {fmt(originalSize)}
          </span>
        </figcaption>
        <div className="checker flex aspect-square items-center justify-center overflow-hidden rounded-lg">
          <img
            src={dataUrl}
            alt="Original PNG"
            className="max-h-full max-w-full object-contain"
          />
        </div>
      </figure>
      <figure className="rounded-2xl border border-slate-200 bg-white p-4">
        <figcaption className="mb-2 flex items-center justify-between text-sm font-medium">
          Result SVG
          <span className="text-xs font-normal text-slate-500">
            {fmt(svgSize)}
          </span>
        </figcaption>
        <div className="checker flex aspect-square items-center justify-center overflow-hidden rounded-lg">
          <img
            src={blobUrl}
            alt="Converted SVG"
            className="max-h-full max-w-full object-contain"
          />
        </div>
      </figure>
    </div>
    <div className="rounded-2xl border border-slate-200 bg-white p-4">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-sm font-medium">SVG code</h2>
        <button
          onClick={onCopy}
          className="rounded-lg bg-slate-100 px-3 py-1.5 text-sm font-medium hover:bg-slate-200">
          Copy
        </button>
      </div>
      <textarea
        readOnly
        rows={6}
        value={svg}
        className="w-full resize-y rounded-lg border border-slate-200 bg-slate-50 p-3 font-mono text-xs text-slate-700"
      />
    </div>
    <button
      onClick={onReset}
      className="w-full rounded-xl border border-slate-300 px-5 py-3 font-semibold hover:bg-slate-100">
      Convert another file
    </button>
  </section>
);
