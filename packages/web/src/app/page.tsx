'use client';

import { useRef, useState } from 'react';
import type { FC } from 'react';
import { convertPngToSvgContent } from '@/lib/core';
import { Dropzone } from '@/components/Dropzone';
import { FilePreview } from '@/components/FilePreview';
import { ResultView } from '@/components/ResultView';

const Home: FC = () => {
  const [file, setFile] = useState<File | null>(null);
  const [dataUrl, setDataUrl] = useState('');
  const [dims, setDims] = useState({ w: 0, h: 0 });
  const [error, setError] = useState('');
  const [progress, setProgress] = useState(0);
  const [converting, setConverting] = useState(false);
  const [showProgress, setShowProgress] = useState(false);
  const [svg, setSvg] = useState('');
  const [blobUrl, setBlobUrl] = useState('');
  const [dragActive, setDragActive] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFile = (f: File | undefined) => {
    setError('');
    if (!f) return;
    if (f.type !== 'image/png' && !/\.png$/i.test(f.name)) {
      setError('Please choose a PNG file.');
      return;
    }
    if (f.size > 10 * 1024 * 1024) {
      setError('That file is larger than 10 MB.');
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const img = new Image();
      img.onload = () => {
        setFile(f);
        setDataUrl(reader.result as string);
        setDims({ w: img.naturalWidth, h: img.naturalHeight });
      };
      img.onerror = () => setError("That file couldn't be read as an image.");
      img.src = reader.result as string;
    };
    reader.onerror = () => setError("Couldn't read the file.");
    reader.readAsDataURL(f);
  };

  const reset = () => {
    setFile(null);
    setDataUrl('');
    setDims({ w: 0, h: 0 });
    setSvg('');
    setBlobUrl('');
    setShowProgress(false);
    setProgress(0);
    if (inputRef.current) inputRef.current.value = '';
    setError('');
  };

  const convert = async () => {
    if (!file) return;
    setConverting(true);
    setShowProgress(true);
    setProgress(0);
    try {
      const result = await convertPngToSvgContent(file);
      setSvg(result);
      const url = URL.createObjectURL(
        new Blob([result], { type: 'image/svg+xml' })
      );
      setBlobUrl(url);
    } catch {
      setError('Conversion failed.');
    } finally {
      setConverting(false);
      setShowProgress(false);
    }
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(svg);
    } catch {
      /* ignore */
    }
  };

  return (
    <main className="mx-auto max-w-3xl px-4 py-10 sm:py-16">
      <header className="mb-8 text-center">
        <div className="inline-flex items-center gap-2 rounded-full bg-indigo-100 px-3 py-1 text-xs font-medium text-indigo-700">
          PNG → SVG
        </div>
        <h1 className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          Vectors
        </h1>
        <p className="mt-2 text-slate-600">
          Drop a PNG file below and turn it into a scalable vector graphic.
        </p>
      </header>

      {error && (
        <p
          role="alert"
          className="mb-3 rounded-lg bg-red-50 px-4 py-2 text-sm text-red-700">
          {error}
        </p>
      )}

      {!file && (
        <Dropzone
          dragActive={dragActive}
          onDrop={(e) => {
            e.preventDefault();
            setDragActive(false);
            handleFile(e.dataTransfer.files?.[0]);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragActive(true);
          }}
          onDragLeave={() => setDragActive(false)}
          inputRef={inputRef}
          onChange={(e) => handleFile(e.target.files?.[0])}
        />
      )}

      {file && (
        <FilePreview
          name={file.name}
          dataUrl={dataUrl}
          dims={dims}
          size={file.size}
          showProgress={showProgress}
          progress={progress}
          converting={converting}
          hasSvg={!!svg}
          onRemove={reset}
          onConvert={convert}
        />
      )}

      {svg && (
        <ResultView
          dataUrl={dataUrl}
          blobUrl={blobUrl}
          originalSize={file!.size}
          svgSize={new Blob([svg]).size}
          svg={svg}
          onCopy={copy}
          onReset={reset}
        />
      )}

      <p className="mt-10 text-center text-xs text-slate-400">
        The conversion runs entirely in your browser; no files are uploaded.
      </p>
    </main>
  );
};

export default Home;
