import init, { WasmRaster, WasmTraceConfig, trace } from '@vectors/core';
let wasmReady: Promise<unknown> | null = null;
const ensureWasm = () => {
  if (!wasmReady) wasmReady = init();
  return wasmReady;
};
import { log } from './log';

const isImage = (file: File): boolean =>
  file.type === 'image/png' ||
  file.type === 'image/jpeg' ||
  file.name.toLowerCase().endsWith('.png') ||
  file.name.toLowerCase().endsWith('.jpg') ||
  file.name.toLowerCase().endsWith('.jpeg');

export const filterPngFiles = (files: File[]): File[] => files.filter(isImage);

export const convertPngToSvgContent = async (file: File): Promise<string> => {
  log.info(`Converting ${file.name} (${file.size} bytes)`);
  const bytes = new Uint8Array(await file.arrayBuffer());
  await ensureWasm();

  let svgContent: string;
  try {
    const raster = WasmRaster.from_bytes(bytes);
    const config = new WasmTraceConfig();
    const result = trace(raster, config);
    svgContent = result.svg;
  } catch (error) {
    log.error('Wasm trace failed, falling back to embedded image', error);
    const dataUrl = await new Promise<string>((res, rej) => {
      const reader = new FileReader();
      reader.onload = () => res(reader.result as string);
      reader.onerror = rej;
      reader.readAsDataURL(file);
    });
    const img = new Image();
    await new Promise<void>((res, rej) => {
      img.onload = () => res();
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
  const suggestedName = fileName.replace(/\.(png|jpe?g)$/i, '.svg');
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
