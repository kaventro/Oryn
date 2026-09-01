// src/modules/syntaxHighlighter.ts
import { escHtml } from './formatUtils.ts';

export type SupportedLang =
  | 'js'
  | 'ts'
  | 'jsx'
  | 'tsx'
  | 'rust'
  | 'python'
  | 'html'
  | 'xml'
  | 'svg'
  | 'css'
  | 'scss'
  | 'json'
  | 'yaml'
  | 'toml'
  | 'sql'
  | 'sh'
  | 'bash'
  | 'zsh'
  | 'c'
  | 'cpp'
  | 'csharp'
  | 'java'
  | 'go'
  | 'plaintext';

export function detectLanguage(filenameOrExt: string): SupportedLang {
  const ext = (filenameOrExt.split('.').pop() || '').toLowerCase();
  switch (ext) {
    case 'js':
    case 'mjs':
    case 'cjs':
      return 'js';
    case 'ts':
    case 'mts':
    case 'cts':
      return 'ts';
    case 'jsx':
      return 'jsx';
    case 'tsx':
      return 'tsx';
    case 'rs':
      return 'rust';
    case 'py':
    case 'pyw':
      return 'python';
    case 'html':
    case 'htm':
      return 'html';
    case 'xml':
    case 'svg':
    case 'plist':
      return 'xml';
    case 'css':
      return 'css';
    case 'scss':
    case 'sass':
    case 'less':
      return 'scss';
    case 'json':
    case 'json5':
    case 'jsonc':
      return 'json';
    case 'yaml':
    case 'yml':
      return 'yaml';
    case 'toml':
      return 'toml';
    case 'sql':
      return 'sql';
    case 'sh':
    case 'bash':
    case 'zsh':
      return 'bash';
    case 'c':
    case 'h':
      return 'c';
    case 'cpp':
    case 'hpp':
    case 'cc':
    case 'cxx':
      return 'cpp';
    case 'cs':
      return 'csharp';
    case 'java':
      return 'java';
    case 'go':
      return 'go';
    default:
      return 'plaintext';
  }
}

/**
 * Fast, secure token-based syntax highlighter for common programming languages.
 * Escapes all characters to prevent HTML/XSS injection.
 */
export function highlightCode(code: string, filenameOrLang: string): string {
  const lang = detectLanguage(filenameOrLang);
  if (lang === 'plaintext' || !code) {
    return escHtml(code);
  }

  // Tokenization rules for C-family / JS / TS / Rust / Python / etc.
  if (lang === 'html' || lang === 'xml' || lang === 'svg') {
    return highlightXml(code);
  }

  if (lang === 'json') {
    return highlightJson(code);
  }

  if (lang === 'yaml' || lang === 'toml') {
    return highlightYaml(code);
  }

  return highlightGeneral(code, lang);
}

function highlightJson(code: string): string {
  const tokens: { pattern: RegExp; cls: string }[] = [
    { pattern: /^"(?:\\.|[^"\\])*"(?=\s*:)/, cls: 'tok-prop' },
    { pattern: /^"(?:\\.|[^"\\])*"/, cls: 'tok-str' },
    { pattern: /^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/, cls: 'tok-num' },
    { pattern: /^(?:true|false|null)\b/, cls: 'tok-kw' },
    { pattern: /^[{}[\],:]/, cls: 'tok-op' },
    { pattern: /^\/\/.*|^\/\*[\s\S]*?\*\//, cls: 'tok-com' },
  ];
  return runLexer(code, tokens);
}

function highlightYaml(code: string): string {
  const tokens: { pattern: RegExp; cls: string }[] = [
    { pattern: /^#.*/, cls: 'tok-com' },
    { pattern: /^"(?:\\.|[^"\\])*"|^'(?:\\.|[^'\\])*'/, cls: 'tok-str' },
    { pattern: /^[\w.-]+(?=\s*:)/, cls: 'tok-prop' },
    { pattern: /^-?\d+(?:\.\d+)?\b/, cls: 'tok-num' },
    { pattern: /^(?:true|false|null|yes|no|on|off)\b/i, cls: 'tok-kw' },
    { pattern: /^[-:|>[\]{}?]/, cls: 'tok-op' },
  ];
  return runLexer(code, tokens);
}

function highlightXml(code: string): string {
  const tokens: { pattern: RegExp; cls: string }[] = [
    { pattern: /^<!--[\s\S]*?-->/, cls: 'tok-com' },
    { pattern: /^<!\[CDATA\[[\s\S]*?\]\]>/, cls: 'tok-str' },
    { pattern: /^<\/?[\w:-]+/, cls: 'tok-tag' },
    { pattern: /^[\w:-]+(?==)/, cls: 'tok-attr' },
    { pattern: /^"[^"]*"|^'[^']*'/, cls: 'tok-str' },
    { pattern: /^[<>/=]/, cls: 'tok-op' },
  ];
  return runLexer(code, tokens);
}

function highlightGeneral(code: string, lang: SupportedLang): string {
  const keywordsByLang: Record<string, string[]> = {
    js: ['import', 'export', 'from', 'default', 'const', 'let', 'var', 'function', 'class', 'extends', 'return', 'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break', 'continue', 'new', 'try', 'catch', 'finally', 'throw', 'typeof', 'instanceof', 'async', 'await', 'yield', 'this', 'super', 'null', 'undefined', 'true', 'false'],
    ts: ['import', 'export', 'from', 'default', 'const', 'let', 'var', 'function', 'class', 'extends', 'implements', 'interface', 'type', 'enum', 'namespace', 'return', 'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break', 'continue', 'new', 'try', 'catch', 'finally', 'throw', 'typeof', 'instanceof', 'async', 'await', 'yield', 'this', 'super', 'null', 'undefined', 'true', 'false', 'public', 'private', 'protected', 'readonly', 'as', 'is', 'keyof'],
    rust: ['fn', 'let', 'mut', 'pub', 'struct', 'enum', 'impl', 'trait', 'type', 'use', 'mod', 'match', 'if', 'else', 'for', 'while', 'loop', 'return', 'break', 'continue', 'as', 'where', 'unsafe', 'async', 'await', 'move', 'ref', 'const', 'static', 'extern', 'crate', 'self', 'Self', 'true', 'false'],
    python: ['def', 'class', 'return', 'if', 'elif', 'else', 'for', 'while', 'try', 'except', 'finally', 'raise', 'import', 'from', 'as', 'with', 'pass', 'break', 'continue', 'lambda', 'yield', 'global', 'nonlocal', 'assert', 'async', 'await', 'True', 'False', 'None', 'and', 'or', 'not', 'is', 'in'],
    sql: ['SELECT', 'FROM', 'WHERE', 'INSERT', 'INTO', 'UPDATE', 'DELETE', 'JOIN', 'LEFT', 'RIGHT', 'INNER', 'OUTER', 'ON', 'GROUP', 'BY', 'ORDER', 'HAVING', 'LIMIT', 'OFFSET', 'CREATE', 'TABLE', 'ALTER', 'DROP', 'INDEX', 'PRIMARY', 'KEY', 'FOREIGN', 'REFERENCES', 'AND', 'OR', 'NOT', 'NULL', 'IS', 'IN', 'EXISTS', 'UNION', 'ALL', 'AS', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END'],
    bash: ['if', 'then', 'else', 'elif', 'fi', 'for', 'in', 'do', 'done', 'while', 'until', 'case', 'esac', 'function', 'return', 'exit', 'export', 'source', 'local', 'alias'],
    c: ['auto', 'break', 'case', 'char', 'const', 'continue', 'default', 'do', 'double', 'else', 'enum', 'extern', 'float', 'for', 'goto', 'if', 'int', 'long', 'register', 'return', 'short', 'signed', 'sizeof', 'static', 'struct', 'switch', 'typedef', 'union', 'unsigned', 'void', 'volatile', 'while', 'NULL', 'true', 'false'],
    cpp: ['auto', 'break', 'case', 'class', 'const', 'continue', 'default', 'delete', 'do', 'else', 'enum', 'explicit', 'export', 'extern', 'for', 'friend', 'if', 'inline', 'mutable', 'namespace', 'new', 'noexcept', 'nullptr', 'operator', 'private', 'protected', 'public', 'register', 'reinterpret_cast', 'return', 'static', 'static_cast', 'struct', 'switch', 'template', 'this', 'throw', 'try', 'typedef', 'typeid', 'typename', 'union', 'using', 'virtual', 'void', 'volatile', 'while'],
    go: ['break', 'default', 'func', 'interface', 'select', 'case', 'defer', 'go', 'map', 'struct', 'chan', 'else', 'goto', 'package', 'switch', 'const', 'fallthrough', 'if', 'range', 'type', 'continue', 'for', 'import', 'return', 'var', 'true', 'false', 'nil'],
    csharp: ['abstract', 'as', 'async', 'await', 'base', 'bool', 'break', 'byte', 'case', 'catch', 'char', 'checked', 'class', 'const', 'continue', 'decimal', 'default', 'delegate', 'do', 'double', 'else', 'enum', 'event', 'explicit', 'extern', 'false', 'finally', 'fixed', 'float', 'for', 'foreach', 'goto', 'if', 'implicit', 'in', 'int', 'interface', 'internal', 'is', 'lock', 'long', 'namespace', 'new', 'null', 'object', 'operator', 'out', 'override', 'params', 'private', 'protected', 'public', 'readonly', 'ref', 'return', 'sbyte', 'sealed', 'short', 'sizeof', 'stackalloc', 'static', 'string', 'struct', 'switch', 'this', 'throw', 'true', 'try', 'typeof', 'uint', 'ulong', 'unchecked', 'unsafe', 'ushort', 'using', 'virtual', 'void', 'volatile', 'while'],
    java: ['abstract', 'assert', 'boolean', 'break', 'byte', 'case', 'catch', 'char', 'class', 'const', 'continue', 'default', 'do', 'double', 'else', 'enum', 'extends', 'final', 'finally', 'float', 'for', 'goto', 'if', 'implements', 'import', 'instanceof', 'int', 'interface', 'long', 'native', 'new', 'package', 'private', 'protected', 'public', 'return', 'short', 'static', 'strictfp', 'super', 'switch', 'synchronized', 'this', 'throw', 'throws', 'transient', 'try', 'void', 'volatile', 'while', 'true', 'false', 'null'],
    css: ['@import', '@media', '@keyframes', '@font-face', '@supports', '!important'],
    scss: ['@import', '@media', '@keyframes', '@font-face', '@supports', '@mixin', '@include', '@extend', '@function', '@return', '@if', '@else', '!important'],
  };

  const kwList = keywordsByLang[lang] || keywordsByLang.js;
  const kwRegex = new RegExp(`^(?:${kwList.join('|')})\\b`, lang === 'sql' ? 'i' : '');

  const tokens: { pattern: RegExp; cls: string }[] = [
    // Comments
    { pattern: lang === 'python' || lang === 'bash' ? /^#.*/ : /^\/\/.*|^\/\*[\s\S]*?\*\//, cls: 'tok-com' },
    // Strings
    { pattern: /^"(?:\\.|[^"\\])*"|^'(?:\\.|[^'\\])*'|^`(?:\\.|[^`\\])*`/, cls: 'tok-str' },
    // Types / Decorators / Attributes
    { pattern: /^@[a-zA-Z_]\w*/, cls: 'tok-attr' },
    { pattern: /^[A-Z][a-zA-Z0-9_]*(?=[<(,\s;&]|$)/, cls: 'tok-type' },
    // Function calls
    { pattern: /^[a-zA-Z_$][\w$]*(?=\s*\()/, cls: 'tok-fn' },
    // Keywords
    { pattern: kwRegex, cls: 'tok-kw' },
    // Numbers
    { pattern: /^0x[0-9a-fA-F]+|^0b[01]+|^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b/, cls: 'tok-num' },
    // Operators
    { pattern: /^(=>|->|::|==|!=|<=|>=|\+\+|--|&&|\|\||[+\-*\/%=<>!&|^~?:])/, cls: 'tok-op' },
  ];

  return runLexer(code, tokens);
}

function runLexer(code: string, tokenRules: { pattern: RegExp; cls: string }[]): string {
  let i = 0;
  let out = '';
  const len = code.length;

  while (i < len) {
    const sub = code.slice(i);
    let matched = false;

    for (const rule of tokenRules) {
      const m = rule.pattern.exec(sub);
      if (m && m[0].length > 0) {
        const text = m[0];
        out += `<span class="${rule.cls}">${escHtml(text)}</span>`;
        i += text.length;
        matched = true;
        break;
      }
    }

    if (!matched) {
      // Advance by one char and escape
      out += escHtml(code[i]);
      i++;
    }
  }

  return out;
}
