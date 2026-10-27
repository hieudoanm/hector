'use client';

import { useRef, useState } from 'react';
import type { FC } from 'react';
import {
  convertPngToSvgContent,
  downloadSvg,
  filterPngFiles,
} from '@/lib/core';
import { log } from '@/lib/log';
import { Dropzone } from '@/components/Dropzone';
import { FileList } from '@/components/FileList';
import { ConvertedList } from '@/components/ConvertedList';

const Home: FC = () => {
  const [files, setFiles] = useState<File[]>([]);
  const [converted, setConverted] = useState<{ name: string; svg: string }[]>(
    []
  );
  const [converting, setConverting] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFiles = (selected: FileList | null) => {
    if (!selected) return;
    const pngFiles = filterPngFiles(Array.from(selected));
    if (pngFiles.length) {
      setFiles((prev) => [...prev, ...pngFiles]);
      log.info(`Added ${pngFiles.length} file(s)`);
    } else {
      log.warn('No PNG files selected');
    }
  };

  const onDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    handleFiles(e.dataTransfer.files);
  };

  const onDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(true);
  };

  const onDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
  };

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
    log.debug(`Removed file at index ${index}`);
  };

  const clearFiles = () => {
    setFiles([]);
    if (inputRef.current) inputRef.current.value = '';
    log.info('Cleared all files');
  };

  const convert = async () => {
    setConverting(true);
    log.info(`Starting conversion of ${files.length} file(s)`);
    try {
      const results: { name: string; svg: string }[] = [];
      for (const file of files) {
        const svgContent = await convertPngToSvgContent(file);
        const suggestedName = file.name.replace(/\.png$/i, '.svg');
        results.push({ name: suggestedName, svg: svgContent });
        await downloadSvg(svgContent, suggestedName);
      }
      setConverted((prev) => [...prev, ...results]);
      log.info('Conversion completed');
    } catch (error) {
      log.error('Conversion failed:', error);
    } finally {
      setConverting(false);
    }
  };

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 p-8">
      <div className="text-center">
        <h1 className="text-3xl font-bold">PNG to SVG</h1>
        <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
          Drop PNG files or click to select
        </p>
      </div>

      <Dropzone
        dragActive={dragActive}
        onDrop={onDrop}
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onClick={() => inputRef.current?.click()}
        inputRef={inputRef}
        onChange={(e) => {
          handleFiles(e.target.files);
          if (inputRef.current) inputRef.current.value = '';
        }}
      />

      {files.length > 0 && (
        <FileList
          files={files}
          onRemove={removeFile}
          onClear={clearFiles}
          onConvert={convert}
          converting={converting}
        />
      )}

      {converted.length > 0 && (
        <ConvertedList items={converted} onClear={() => setConverted([])} />
      )}
    </main>
  );
};

export default Home;
