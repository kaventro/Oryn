// src/modules/binaryPreview.ts
// Shared heuristics for refusing to render binary files as text in previews.
// Used by the F3 viewer (commandsController) and the split-pane Quick View.

export const NON_TEXT_PREVIEW_EXTS: Set<string> = new Set([
  // archives & compression
  'zip', 'rar', '7z', 'gz', 'tgz', 'bz2', 'tbz2', 'xz', 'zst', 'lz', 'lzma',
  'tar', 'cab', 'arj', 'z', 'jar', 'war', 'apk', 'aab', 'nupkg', 'deb', 'rpm',
  // disk & installer images
  'iso', 'dmg', 'img', 'vhd', 'vhdx', 'vmdk', 'ova', 'pkg', 'msi', 'msix', 'appimage',
  // executables & compiled artifacts
  'exe', 'dll', 'com', 'so', 'dylib', 'bin', 'obj', 'class', 'pyc', 'pyd',
  'wasm', 'node', 'elf', 'ko',
  // media not handled as an image elsewhere
  'mp3', 'wav', 'flac', 'aac', 'ogg', 'oga', 'm4a', 'wma',
  'mp4', 'm4v', 'mkv', 'mov', 'avi', 'webm', 'wmv', 'flv',
  'psd', 'ai', 'xcf', 'heic', 'heif',
  // fonts
  'ttf', 'otf', 'woff', 'woff2', 'eot',
  // databases & opaque binary data
  'db', 'sqlite', 'sqlite3', 'mdb', 'accdb', 'pack', 'idx',
  // documents we do not render
  'pdf',
]);

/**
 * True when a string that came back from Rust's from_utf8_lossy read is really
 * binary. Invalid bytes decode to U+FFFD and NUL is kept verbatim: real text has
 * neither, a binary read this way is full of both. Sampling the head keeps a
 * 500 KB read cheap.
 */
export function looksBinaryText(text?: string | null): boolean {
  if (!text) return false;
  const sample = text.length > 8192 ? text.slice(0, 8192) : text;
  if (sample.includes('\u0000')) return true; // NUL never appears in real text
  let replacements = 0;
  for (let i = 0; i < sample.length; i += 1) {
    if (sample.charCodeAt(i) === 0xfffd) replacements += 1;
  }
  return replacements * 100 >= sample.length * 2; // ≥2% replacement characters
}
