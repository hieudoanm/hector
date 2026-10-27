import type { FC } from 'react';

export interface FilePreviewProps {
  name: string;
  dataUrl: string;
  dims: { w: number; h: number };
  size: number;
  showProgress: boolean;
  progress: number;
  converting: boolean;
  hasSvg: boolean;
  onRemove: () => void;
  onConvert: () => void;
}

const fmt = (b: number) =>
  b < 1024
    ? `${b} B`
    : b < 1048576
      ? `${(b / 1024).toFixed(1)} KB`
      : `${(b / 1048576).toFixed(2)} MB`;

export const FilePreview: FC<FilePreviewProps> = ({
  name,
  dataUrl,
  dims,
  size,
  showProgress,
  progress,
  converting,
  hasSvg,
  onRemove,
  onConvert,
}) => (
  <section className="rounded-2xl border border-slate-200 bg-white p-5">
    <div className="flex items-center gap-4">
      <div className="checker h-20 w-20 shrink-0 overflow-hidden rounded-lg border border-slate-200">
        <img
          src={dataUrl}
          alt="Selected PNG preview"
          className="h-full w-full object-contain"
        />
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate font-semibold">{name}</p>
        <p className="text-sm text-slate-500">
          {dims.w} × {dims.h}px · {fmt(size)}
        </p>
      </div>
      <button
        onClick={onRemove}
        className="rounded-lg px-3 py-2 text-sm text-slate-500 hover:bg-slate-100"
        aria-label="Remove file">
        Remove
      </button>
    </div>
    {showProgress && (
      <div className="mt-5">
        <div className="mb-1 flex justify-between text-xs text-slate-500">
          <span>Converting…</span>
          <span>{progress}%</span>
        </div>
        <div className="h-2 w-full overflow-hidden rounded-full bg-slate-200">
          <div
            className="h-full rounded-full bg-indigo-500 transition-all duration-150"
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>
    )}
    <button
      onClick={onConvert}
      disabled={converting}
      className="mt-5 w-full rounded-xl bg-indigo-600 px-5 py-3 font-semibold text-white shadow hover:bg-indigo-500 focus:ring-4 focus:ring-indigo-300/50 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60">
      {converting ? 'Converting…' : hasSvg ? 'Convert again' : 'Convert to SVG'}
    </button>
  </section>
);
