import type { FC } from 'react';

export interface DropzoneProps {
  dragActive: boolean;
  onDrop: (e: React.DragEvent<HTMLLabelElement>) => void;
  onDragOver: (e: React.DragEvent<HTMLLabelElement>) => void;
  onDragLeave: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
}

export const Dropzone: FC<DropzoneProps> = ({
  dragActive,
  onDrop,
  onDragOver,
  onDragLeave,
  inputRef,
  onChange,
}) => (
  <label
    tabIndex={0}
    htmlFor="fileInput"
    onDrop={onDrop}
    onDragOver={onDragOver}
    onDragLeave={onDragLeave}
    className={`flex cursor-pointer flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed px-6 py-14 text-center transition focus:ring-4 focus:ring-indigo-300/50 focus:outline-none ${
      dragActive
        ? 'border-indigo-500 bg-indigo-50'
        : 'border-slate-300 bg-white'
    }`}>
    <svg
      className="h-12 w-12 text-indigo-500"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={1.5}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 16V4m0 0L8 8m4-4 4 4M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"
      />
    </svg>
    <div>
      <p className="text-lg font-semibold">
        <span className="text-indigo-600">Click to upload</span> or drag and
        drop
      </p>
      <p className="mt-1 text-sm text-slate-500">PNG files only, up to 10 MB</p>
    </div>
    <input
      id="fileInput"
      ref={inputRef}
      type="file"
      accept="image/png,.png"
      className="sr-only"
      onChange={onChange}
    />
  </label>
);
