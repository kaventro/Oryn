// src/modules/icons/iconRegistry.ts
import { ICON_SVGS, type IconKey } from './iconDefinitions.ts';

export type { IconKey };

export interface IconItemDescriptor {
  base?: string;
  isDir?: boolean;
}

export class IconRegistry {
  private exactNameMap: Map<string, IconKey>;
  private prefixNameMap: Array<{ prefix: string; iconKey: IconKey }>;
  private extensionMap: Map<string, IconKey>;

  constructor() {
    this.exactNameMap = new Map();
    this.prefixNameMap = [];
    this.extensionMap = new Map();

    this.seedDefaults();
  }

  /**
   * Register icon for an exact file name (e.g. 'Dockerfile', 'package.json')
   */
  public registerExactName(name: string, iconKey: IconKey): this {
    this.exactNameMap.set(name.toLowerCase(), iconKey);
    return this;
  }

  /**
   * Register icon for a prefix pattern (e.g. 'dockerfile.', 'license.')
   */
  public registerNamePrefix(prefix: string, iconKey: IconKey): this {
    this.prefixNameMap.push({ prefix: prefix.toLowerCase(), iconKey });
    return this;
  }

  /**
   * Register icon for one or multiple extensions (e.g. ['ts', 'mts', 'cts'], 'ts')
   */
  public registerExtensions(extensions: string | string[], iconKey: IconKey): this {
    const list = Array.isArray(extensions) ? extensions : [extensions];
    for (const ext of list) {
      const cleanExt = ext.startsWith('.') ? ext.slice(1) : ext;
      this.extensionMap.set(cleanExt.toLowerCase(), iconKey);
    }
    return this;
  }

  /**
   * Resolves the IconKey for a given file item descriptor
   */
  public resolveIconKey(item?: IconItemDescriptor | null): IconKey {
    if (!item) return 'doc';
    if (item.base === '..') return 'parent';
    if (item.isDir) return 'folder';

    const lowerBase = (item.base || '').toLowerCase();
    if (!lowerBase) return 'doc';

    // 1. Exact file name matches
    if (this.exactNameMap.has(lowerBase)) {
      return this.exactNameMap.get(lowerBase)!;
    }

    // 2. Prefix matches (e.g. Dockerfile.dev, license.txt)
    for (const entry of this.prefixNameMap) {
      if (lowerBase.startsWith(entry.prefix)) {
        return entry.iconKey;
      }
    }

    // 3. Extension matches
    const dotIdx = lowerBase.lastIndexOf('.');
    if (dotIdx > 0 && dotIdx < lowerBase.length - 1) {
      const ext = lowerBase.slice(dotIdx + 1);
      if (this.extensionMap.has(ext)) {
        return this.extensionMap.get(ext)!;
      }
    }

    return 'doc';
  }

  /**
   * Returns the SVG markup for a given IconKey
   */
  public getSvg(iconKey: IconKey): string {
    return ICON_SVGS[iconKey] || ICON_SVGS.doc;
  }

  /**
   * Resolves SVG markup directly from an item descriptor
   */
  public resolveSvg(item?: IconItemDescriptor | null): string {
    return this.getSvg(this.resolveIconKey(item));
  }

  /**
   * Seed standard programming language and developer file icons
   */
  public seedDefaults(): void {
    // Exact file names
    this.registerExactName('dockerfile', 'docker');
    this.registerExactName('docker-compose.yml', 'docker');
    this.registerExactName('docker-compose.yaml', 'docker');
    this.registerExactName('package.json', 'pkgJson');
    this.registerExactName('package-lock.json', 'pkgJson');
    this.registerExactName('vite.config.js', 'vite');
    this.registerExactName('vite.config.ts', 'vite');
    this.registerExactName('vite.config.mjs', 'vite');
    this.registerExactName('vite.config.cjs', 'vite');
    this.registerExactName('.gitignore', 'git');
    this.registerExactName('.gitattributes', 'git');
    this.registerExactName('.gitmodules', 'git');
    this.registerExactName('license', 'license');
    this.registerExactName('licence', 'license');
    this.registerExactName('copying', 'license');

    // Name prefixes
    this.registerNamePrefix('dockerfile.', 'docker');
    this.registerNamePrefix('license.', 'license');
    this.registerNamePrefix('licence.', 'license');

    // Programming Languages & Frameworks
    this.registerExtensions(['swift'], 'swift');
    this.registerExtensions(['go', 'gomod', 'gowork'], 'go');
    this.registerExtensions(['c', 'h'], 'c');
    this.registerExtensions(['cpp', 'cc', 'cxx', 'hpp', 'hh', 'hxx'], 'cpp');
    this.registerExtensions(['cs', 'csx'], 'csharp');
    this.registerExtensions(['java', 'jar', 'class'], 'java');
    this.registerExtensions(['kt', 'kts'], 'kotlin');
    this.registerExtensions(['php', 'phtml', 'php4', 'php5', 'php7', 'php8'], 'php');
    this.registerExtensions(['rb', 'erb', 'gemspec', 'rake'], 'ruby');
    this.registerExtensions(['dart'], 'dart');
    this.registerExtensions(['vue'], 'vue');
    this.registerExtensions(['svelte'], 'svelte');
    this.registerExtensions(['lua'], 'lua');
    this.registerExtensions(['graphql', 'gql'], 'graphql');
    this.registerExtensions(['sol'], 'solidity');
    this.registerExtensions(['rs'], 'rust');
    this.registerExtensions(['py', 'pyw', 'ipynb'], 'py');
    this.registerExtensions(['js', 'mjs', 'cjs'], 'js');
    this.registerExtensions(['ts', 'mts', 'cts'], 'ts');
    this.registerExtensions(['jsx', 'tsx'], 'react');
    this.registerExtensions(['sh', 'bash', 'zsh', 'fish', 'ps1', 'bat', 'cmd'], 'sh');
    this.registerExtensions(['r', 'scala', 'sc', 'ex', 'exs', 'erl', 'hs', 'lhs', 'clj', 'cljs', 'cljc', 'edn', 'pl', 'pm'], 'code');

    // Data & Config formats
    this.registerExtensions(['md', 'markdown', 'mdown', 'mkd'], 'md');
    this.registerExtensions(['yml', 'yaml'], 'yaml');
    this.registerExtensions(['json', 'json5', 'jsonc'], 'json');
    this.registerExtensions(['css', 'scss', 'sass', 'less', 'styl'], 'css');
    this.registerExtensions(['html', 'htm', 'xhtml'], 'html');
    this.registerExtensions(['toml', 'ini', 'env', 'conf', 'cfg', 'properties'], 'config');
    this.registerExtensions(['sql', 'sqlite', 'sqlite3', 'db', 'pgsql', 'mysql', 'prisma'], 'sql');

    // Media & Assets
    this.registerExtensions(
      ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'heic', 'heif', 'bmp', 'ico', 'tiff', 'tif', 'raw', 'cr2', 'nef', 'arw', 'dng', 'avif', 'psd', 'ai', 'eps', 'jfif', 'pjpeg', 'pjp'],
      'img'
    );
    this.registerExtensions(['mp3', 'wav', 'ogg', 'flac', 'aac', 'm4a', 'mp4', 'mkv', 'avi', 'mov', 'webm'], 'media');
    this.registerExtensions(['ttf', 'otf', 'woff', 'woff2', 'eot'], 'font');
    this.registerExtensions(['zip', 'tar', 'gz', 'bz2', 'xz', '7z', 'rar', 'tgz'], 'archive');
    this.registerExtensions(['bak', 'backup', 'bkp', 'old', 'orig', 'save', 'tmp_bak'], 'backup');
    this.registerExtensions(['pdf'], 'pdf');
  }
}

export const defaultIconRegistry = new IconRegistry();
