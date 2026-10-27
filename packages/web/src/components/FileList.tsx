import type { FC } from 'react';

type FileListProps = {
  files: File[];
  onRemove: (index: number) => void;
  onClear: () => void;
  onConvert: () => void;
  converting: boolean;
};

export const FileList: FC<FileListProps> = ({
  files,
  onRemove,
  onClear,
  onConvert,
  converting,
}) => {
  return (
    <div className="w-full max-w-xl space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium">
          {files.length} file{files.length > 1 ? 's' : ''} selected
        </p>
        <button
          onClick={onClear}
          className="text-sm text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200">
          Clear all
        </button>
      </div>
      <ul className="divide-y divide-gray-200 rounded-lg border border-gray-200 dark:divide-gray-800 dark:border-gray-800">
        {files.map((file, index) => (
          <li
            key={`${file.name}-${index}`}
            className="flex items-center justify-between p-3">
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{file.name}</p>
              <p className="text-xs text-gray-500">
                {(file.size / 1024).toFixed(1)} KB
              </p>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRemove(index);
              }}
              className="ml-4 cursor-pointer text-sm text-red-600 hover:text-red-700">
              Remove
            </button>
          </li>
        ))}
      </ul>
      <button
        onClick={onConvert}
        disabled={converting || files.length === 0}
        className="w-full cursor-pointer rounded bg-blue-600 px-4 py-2 text-white disabled:cursor-not-allowed disabled:opacity-50">
        {converting
          ? 'Converting...'
          : `Convert ${files.length} file${files.length > 1 ? 's' : ''} to SVG`}
      </button>
    </div>
  );
};
