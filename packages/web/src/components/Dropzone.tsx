import type { FC } from 'react';

type DropzoneProps = {
  dragActive: boolean;
  onDrop: (e: React.DragEvent<HTMLDivElement>) => void;
  onDragOver: (e: React.DragEvent<HTMLDivElement>) => void;
  onDragLeave: (e: React.DragEvent<HTMLDivElement>) => void;
  onClick: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
};

export const Dropzone: FC<DropzoneProps> = ({
  dragActive,
  onDrop,
  onDragOver,
  onDragLeave,
  onClick,
  inputRef,
  onChange,
}) => {
  return (
    <div
      onDrop={onDrop}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onClick={onClick}
      className={`w-full max-w-xl cursor-pointer rounded-lg border-2 border-dashed p-10 text-center transition-colors ${
        dragActive
          ? 'border-blue-500 bg-blue-50 dark:bg-blue-950/20'
          : 'border-gray-300 hover:border-gray-400 dark:border-gray-700 dark:hover:border-gray-600'
      }`}>
      <input
        ref={inputRef}
        type="file"
        accept=".png,image/png"
        multiple
        className="hidden"
        onChange={onChange}
      />
      <div className="flex flex-col items-center gap-2">
        <svg
          className="h-10 w-10 text-gray-400"
          stroke="currentColor"
          fill="none"
          viewBox="0 0 48 48"
          aria-hidden="true">
          <path
            d="M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02"
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        <p className="text-sm">
          <span className="font-semibold text-blue-600">Click to upload</span>{' '}
          or drag and drop
        </p>
        <p className="text-xs text-gray-500">PNG files only</p>
      </div>
    </div>
  );
};
