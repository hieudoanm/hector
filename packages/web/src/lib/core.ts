import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { log } from './log';

const isTauri = (): boolean => {
  if (typeof window === 'undefined') return false;
  const w = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return !!(w.__TAURI_INTERNALS__ || w.__TAURI__);
};

export const convertPngToSvgContent = async (file: File): Promise<string> => {
  log.info(`Converting ${file.name} (${file.size} bytes)`);
  const dataUrl = await new Promise<string>((res, rej) => {
    const reader = new FileReader();
    reader.onload = () => res(reader.result as string);
    reader.onerror = rej;
    reader.readAsDataURL(file);
  });

  let svgContent: string;
  if (isTauri()) {
    log.debug('Using Tauri backend for conversion');
    svgContent = await invoke<string>('convert_png_to_svg', {
      pngDataUrl: dataUrl,
    });
  } else {
    log.debug('Using browser fallback for conversion');
    const img = new Image();
    await new Promise((res, rej) => {
      img.onload = res;
      img.onerror = rej;
      img.src = dataUrl;
    });

    const width = img.width || 512;
    const height = img.height || 512;
    svgContent = `<?xml version="1.0" encoding="UTF-8"?>\n<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">\n  <image width="${width}" height="${height}" xlink:href="${dataUrl}" />\n</svg>\n`;
  }

  log.info(`Successfully converted ${file.name}`);
  return svgContent;
};

export const downloadSvg = async (
  svgContent: string,
  fileName: string
): Promise<void> => {
  const suggestedName = fileName.replace(/\.png$/i, '.svg');
  if (isTauri()) {
    try {
      const path = await save({
        defaultPath: suggestedName,
        filters: [
          {
            name: 'SVG',
            extensions: ['svg'],
          },
        ],
      });
      if (path) {
        await invoke('save_svg', { path, svgContent });
        log.info(`Saved to ${path}`);
      }
      return;
    } catch (error) {
      log.error('Failed to save via dialog:', error);
    }
  }

  const blob = new Blob([svgContent], { type: 'image/svg+xml' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = suggestedName;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  log.debug(`Downloaded ${a.download}`);
};

export const filterPngFiles = (files: File[]): File[] => {
  return files.filter(
    (file) =>
      file.type === 'image/png' || file.name.toLowerCase().endsWith('.png')
  );
};
