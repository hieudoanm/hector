import type { FC } from 'react';
import { useState } from 'react';
import { downloadSvg } from '@/lib/core';

type ConvertedItem = {
  name: string;
  svg: string;
};

type ConvertedListProps = {
  items: ConvertedItem[];
  onClear: () => void;
};

export const ConvertedList: FC<ConvertedListProps> = ({ items, onClear }) => {
  const [previews, setPreviews] = useState<string[]>([]);

  const generatePreviews = () => {
    const urls = items.map(
      (item) =>
        `data:image/svg+xml;charset=utf-8,${encodeURIComponent(item.svg)}`
    );
    setPreviews(urls);
  };

  return (
    <div className="w-full max-w-xl space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium">Converted SVGs</p>
        <div className="flex gap-2">
          <button
            onClick={generatePreviews}
            className="cursor-pointer text-sm text-blue-600 hover:text-blue-700">
            Preview
          </button>
          <button
            onClick={onClear}
            className="cursor-pointer text-sm text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200">
            Clear all
          </button>
        </div>
      </div>
      {previews.length > 0 && (
        <div className="grid grid-cols-2 gap-2">
          {previews.map((url, index) => (
            <div
              key={index}
              className="flex items-center justify-center rounded border border-gray-200 p-2 dark:border-gray-800">
              <img src={url} alt={items[index]?.name} className="max-h-32" />
            </div>
          ))}
        </div>
      )}
      <ul className="divide-y divide-gray-200 rounded-lg border border-gray-200 dark:divide-gray-800 dark:border-gray-800">
        {items.map((item, index) => (
          <li
            key={`${item.name}-${index}`}
            className="flex items-center justify-between p-3">
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{item.name}</p>
            </div>
            <button
              onClick={async () => {
                await downloadSvg(item.svg, item.name);
              }}
              className="ml-4 cursor-pointer rounded bg-green-600 px-3 py-1 text-sm text-white hover:bg-green-700">
              Save
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
};
