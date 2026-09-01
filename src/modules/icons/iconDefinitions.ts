// src/modules/icons/iconDefinitions.ts

export type IconKey =
  | 'parent'
  | 'folder'
  | 'swift'
  | 'go'
  | 'c'
  | 'cpp'
  | 'csharp'
  | 'java'
  | 'kotlin'
  | 'php'
  | 'ruby'
  | 'dart'
  | 'vue'
  | 'svelte'
  | 'lua'
  | 'docker'
  | 'graphql'
  | 'solidity'
  | 'md'
  | 'yaml'
  | 'js'
  | 'ts'
  | 'react'
  | 'json'
  | 'pkgJson'
  | 'css'
  | 'git'
  | 'license'
  | 'vite'
  | 'html'
  | 'rust'
  | 'py'
  | 'sh'
  | 'config'
  | 'media'
  | 'font'
  | 'pdf'
  | 'img'
  | 'archive'
  | 'backup'
  | 'code'
  | 'sql'
  | 'doc';

export const ICON_SVGS: Record<IconKey, string> = {
  parent: `<svg class="mac-icon mac-icon--parent" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14l-4-4 4-4"/><path d="M5 10h11a4 4 0 1 1 0 8h-1"/></svg>`,
  folder: `<svg class="mac-icon mac-icon--folder" viewBox="0 0 24 24" width="16" height="16" fill="#0a84ff" stroke="#0a84ff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" fill="#0a84ff" fill-opacity="0.4"/></svg>`,
  backup: `<svg class="mac-icon mac-icon--backup" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l3 2"/></svg>`,
  swift: `<svg class="mac-icon mac-icon--swift" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M21.7 18.2c-.4.4-3.1 2.3-6.8 1.4 5.3-2.6 6.8-7.3s-4.3 4.2-8.9 2c4.7-2.8 5.7-7 5.7-7S13 12.3 8.3 8.8c.4 1 1 2.3 2.1 3.4-3.5-1.9-5.1-4.7-5.1-4.7s1.3 3.6 4.7 6.1C5.4 14.1 2.5 10.9 2 10.3c0 0 1.2 4.4 6 7 2.9 1.6 5.8 1.8 7.3 1.6-4.6 2.7-10.1.7-10.1.7s6.1 4.5 13.8 1.4c1.2-.5 2.1-1.3 2.7-1.8v-1z"/></svg>`,
  go: `<svg class="mac-icon mac-icon--go" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><text x="12" y="16.5" font-size="12" font-weight="900" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle" letter-spacing="-0.5">GO</text></svg>`,
  c: `<svg class="mac-icon mac-icon--c" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M17.5 7.5A7.5 7.5 0 1 0 17.5 16.5l-1.8-2a4.8 4.8 0 1 1 0-5l1.8-2z"/></svg>`,
  cpp: `<svg class="mac-icon mac-icon--cpp" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M11 7.5A6 6 0 1 0 11 16.5l-1.4-1.8a3.7 3.7 0 1 1 0-4.4L11 7.5zM14 11h1.5V9.5h1V11H18v1h-1.5v1.5h-1V12H14v-1zm4.5 0H20V9.5h1V11h1.5v1H21v1.5h-1V12h-1.5v-1z"/></svg>`,
  csharp: `<svg class="mac-icon mac-icon--csharp" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M11 7.5A6 6 0 1 0 11 16.5l-1.4-1.8a3.7 3.7 0 1 1 0-4.4L11 7.5z"/><path d="M15 8h1.2l-.4 2.2h1.5l.4-2.2H19l-.4 2.2H20v1.2h-1.6l-.3 1.8H19.5v1.2H17.9l-.4 2.2H16.3l.4-2.2h-1.5l-.4 2.2H13.6l.4-2.2H13v-1.2h1.2l.3-1.8H13V10.2h1.7l.4-2.2zm1.1 3.4l-.3 1.8h1.5l.3-1.8h-1.5z"/></svg>`,
  java: `<svg class="mac-icon mac-icon--java" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M4 19c3 1 10 1 13 0l-1-1c-2.5.5-8.5.5-11 0l-1 1zm0-3c4 .8 9 .8 13 0l-.8-.8c-3.4.5-8 .5-11.4 0L4 16zm6.5-6.5C10 8 11 6.5 12.5 5c0 0-.5 1.5-.5 3s1 2.5 1 4c0 1.5-1.5 2.5-1.5 2.5s.5-1 .5-2-1.5-2-1.5-3zm-3 2C7 10 8 8.5 9.5 7c0 0-.5 1.5-.5 3s1 2.5 1 4c0 1.5-1.5 2.5-1.5 2.5s.5-1 .5-2-1.5-2-1.5-3z"/></svg>`,
  kotlin: `<svg class="mac-icon mac-icon--kotlin" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3 3h18L12 12l9 9H3V3zm0 0l9 9-9 9V3z"/></svg>`,
  php: `<svg class="mac-icon mac-icon--php" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><text x="12" y="16.5" font-size="10" font-weight="900" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle" letter-spacing="-0.5">PHP</text></svg>`,
  ruby: `<svg class="mac-icon mac-icon--ruby" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M6.5 3L2 9.5 12 21 22 9.5 17.5 3h-11zM7 5h4.5l-2 4H4.5L7 5zm5.5 0h4.5l2.5 4h-5l-2-4zm-1 5l1.8 7.5L5.5 10h6zm2.5 0h6l-7.8 7.5 1.8-7.5z"/></svg>`,
  dart: `<svg class="mac-icon mac-icon--dart" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M4 14l5-5h6l5 5-8 8-8-8zm5-9l3-3 8 8-3 3-8-8z"/></svg>`,
  vue: `<svg class="mac-icon mac-icon--vue" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M2 3h4.5L12 13.5 17.5 3H22L12 20.5 2 3zm4.5 0h3L12 7.5 14.5 3h3L12 12 6.5 3z"/></svg>`,
  svelte: `<svg class="mac-icon mac-icon--svelte" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M19.8 7.2c-.7-2.1-2.5-3.6-4.7-3.9-3.2-.4-6.3 1.2-7.5 4.1-.4 1-.4 2.1-.1 3.1l-2.6 1.5c-1.3 1.4-1.8 3.3-1.4 5.2.7 2.8 3.2 4.7 6.1 4.7 1.8 0 3.6-.7 4.9-2l2.6-1.5c1.4-1.4 1.8-3.4 1.4-5.3-.2-.8-.6-1.6-1.1-2.2l2.4-1.4c.8-.7 1.2-1.6 1.4-2.3zm-6.2 9.3c-.6.6-1.4.9-2.2.9-1.3 0-2.5-.9-2.8-2.1-.2-.9.1-1.8.7-2.4l3.1-1.8c.4.6.6 1.3.5 2-.1.9-.6 1.6-1.3 2.1l2 1.3zm1.1-6.1l-2-1.3c.6-.6 1.4-.9 2.2-.9 1.3 0 2.5.9 2.8 2.1.2.9-.1 1.8-.7 2.4l-3.1 1.8c-.4-.6-.6-1.3-.5-2 .1-.9.6-1.6 1.3-2.1z"/></svg>`,
  lua: `<svg class="mac-icon mac-icon--lua" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="7"/><circle cx="17" cy="7" r="2.5" fill="currentColor"/></svg>`,
  docker: `<svg class="mac-icon mac-icon--docker" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M22.5 10.5c-.4-.3-1.6-.4-2.5.2-.2-1.1-.9-2.1-2-2.7l-.6-.3-.3.6c-.5 1-1.3 1.7-2.3 2H2c-.5 0-1 .4-1 1 0 3.3 2 6.2 5.5 7.2 4.5 1.3 10.5 1.3 15-1.5 1.6-1.1 2.5-3 2.5-5 0-.9-.2-1.9-1.5-2.5zM6 8.5H4V6.5h2v2zm3 0H7V6.5h2v2zm3 0h-2V6.5h2v2zm3 0h-2V6.5h2v2zm-6-3H7V3.5h2V5.5zm3 0h-2V3.5h2V5.5zm3 0h-2V3.5h2V5.5z"/></svg>`,
  graphql: `<svg class="mac-icon mac-icon--graphql" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M12 2l8.6 5v10L12 22 3.4 17V7L12 2zm0 2.3L5.4 8.2v7.6L12 19.7l6.6-3.9V8.2L12 4.3z"/><circle cx="12" cy="3" r="1.5"/><circle cx="21" cy="7.5" r="1.5"/><circle cx="21" cy="16.5" r="1.5"/><circle cx="12" cy="21" r="1.5"/><circle cx="3" cy="16.5" r="1.5"/><circle cx="3" cy="7.5" r="1.5"/></svg>`,
  solidity: `<svg class="mac-icon mac-icon--solidity" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M12 2L6 12l6 4 6-4-6-10zm0 14.5L6 13l6 9 6-9-6 3.5z"/></svg>`,
  md: `<svg class="mac-icon mac-icon--md" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M2.5 7.5h2.5l2.5 4 2.5-4h2.5v9h-2.2v-5.2l-2.3 3.6h-1l-2.3-3.6V16.5H2.5v-9z"/><path d="M18.5 7.5v5.2h2l-3.5 4.3-3.5-4.3h2V7.5h3z"/></svg>`,
  yaml: `<svg class="mac-icon mac-icon--yaml" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M6 5.5h3.4l2.6 4.8 2.6-4.8H18l-4.5 7.7V18.5h-3v-5.3L6 5.5z"/></svg>`,
  js: `<svg class="mac-icon mac-icon--js" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M5 6.5h2.8v7.2c0 1.2-.6 1.8-1.8 1.8-.6 0-1.2-.2-1.6-.5l.4-2.1c.3.2.6.3.8.3.4 0 .6-.2.6-.6V6.5z"/><path d="M10.5 13.6c.6.7 1.4 1.1 2.3 1.1 1 0 1.6-.5 1.6-1.2 0-.8-.6-1.1-1.8-1.6-1.8-.6-2.6-1.4-2.6-2.8 0-1.7 1.3-2.9 3.4-2.9 1.1 0 2 .3 2.7.9l-.7 2.1c-.5-.4-1.2-.7-2-.7-.8 0-1.3.4-1.3 1 0 .6.4.9 1.5 1.4 2 .7 2.9 1.6 2.9 3 0 1.9-1.4 3-3.6 3-1.4 0-2.5-.5-3.3-1.3l.9-2z"/></svg>`,
  ts: `<svg class="mac-icon mac-icon--ts" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3.5 6.5h7.5v2.4H8.5V17H6V8.9H3.5V6.5z"/><path d="M11.5 13.6c.6.7 1.4 1.1 2.3 1.1 1 0 1.6-.5 1.6-1.2 0-.8-.6-1.1-1.8-1.6-1.8-.6-2.6-1.4-2.6-2.8 0-1.7 1.3-2.9 3.4-2.9 1.1 0 2 .3 2.7.9l-.7 2.1c-.5-.4-1.2-.7-2-.7-.8 0-1.3.4-1.3 1 0 .6.4.9 1.5 1.4 2 .7 2.9 1.6 2.9 3 0 1.9-1.4 3-3.6 3-1.4 0-2.5-.5-3.3-1.3l.9-2z"/></svg>`,
  react: `<svg class="mac-icon mac-icon--react" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6"><ellipse cx="12" cy="12" rx="9" ry="3.5"/><ellipse cx="12" cy="12" rx="9" ry="3.5" transform="rotate(60 12 12)"/><ellipse cx="12" cy="12" rx="9" ry="3.5" transform="rotate(120 12 12)"/><circle cx="12" cy="12" r="1.8" fill="currentColor" stroke="none"/></svg>`,
  json: `<svg class="mac-icon mac-icon--json" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 5c-1.5 0-2.5 1-2.5 2.5v2c0 1-.8 1.8-2 2 1.2.2 2 1 2 2v2c0 1.5 1 2.5 2.5 2.5"/><path d="M16 5c1.5 0 2.5 1 2.5 2.5v2c0 1 .8 1.8 2 2-1.2.2-2 1-2 2v2c0 1.5-1 2.5-2.5 2.5"/></svg>`,
  pkgJson: `<svg class="mac-icon mac-icon--pkg-json" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l8.5 4.9v9.8L12 21.6 3.5 16.7V6.9L12 2z"/><path d="M9 10v4c0 .8-.4 1.2-1.2 1.2-.4 0-.8-.1-1-.3l.3-1.4c.2.1.4.2.5.2.3 0 .4-.1.4-.4V10h1z" fill="currentColor" stroke="none"/><path d="M12.5 14.5c.4.4.9.7 1.5.7.7 0 1.1-.3 1.1-.8 0-.5-.4-.7-1.2-1-1.2-.4-1.7-1-1.7-1.9 0-1.1.9-1.9 2.3-1.9.7 0 1.3.2 1.8.6l-.5 1.4c-.3-.3-.8-.5-1.3-.5-.5 0-.9.3-.9.7 0 .4.3.6 1 .9 1.3.5 1.9 1.1 1.9 2 0 1.3-.9 2-2.4 2-.9 0-1.7-.3-2.2-.9l.6-1.3z" fill="currentColor" stroke="none"/></svg>`,
  css: `<svg class="mac-icon mac-icon--css" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="4" y1="9" x2="20" y2="9"/><line x1="4" y1="15" x2="20" y2="15"/><line x1="10" y1="3" x2="8" y2="21"/><line x1="16" y1="3" x2="14" y2="21"/></svg>`,
  git: `<svg class="mac-icon mac-icon--git" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M22.5 10.7l-9.2-9.2c-.7-.7-1.9-.7-2.6 0L8.4 3.8l3.3 3.3c.7-.2 1.5 0 2.1.6.6.6.8 1.4.6 2.1l3.2 3.2c.7-.2 1.5 0 2.1.6.8.8.8 2.1 0 2.9s-2.1.8-2.9 0c-.6-.6-.8-1.4-.6-2.1l-3-3v4.6c.4.3.7.8.7 1.4 0 1.1-.9 2-2 2s-2-.9-2-2c0-.6.3-1.1.7-1.4v-4.7c-.4-.3-.7-.8-.7-1.4 0-.6.3-1.1.6-1.5L7.1 2.5 1.5 8.1c-.7.7-.7 1.9 0 2.6l9.2 9.2c.7.7 1.9.7 2.6 0l9.2-9.2c.7-.7.7-1.9 0-2.6z"/></svg>`,
  license: `<svg class="mac-icon mac-icon--license" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9.5"/><path d="M14.5 9.5a3.5 3.5 0 1 0 0 5"/></svg>`,
  vite: `<svg class="mac-icon mac-icon--vite" viewBox="0 0 24 24" width="16" height="16" fill="#fbbf24" stroke="#a855f7" stroke-width="1.5" stroke-linejoin="round"><path d="M13 2L3 14h8l-2 8 12-12h-8l2-8z"/></svg>`,
  html: `<svg class="mac-icon mac-icon--html" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="7 8 3 12 7 16"/><polyline points="17 8 21 12 17 16"/><line x1="14" y1="4" x2="10" y2="20"/></svg>`,
  rust: `<svg class="mac-icon mac-icon--rust" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><text x="12" y="16.5" font-size="13" font-weight="800" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle" letter-spacing="-0.5">RS</text></svg>`,
  py: `<svg class="mac-icon mac-icon--py" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M11.9 2c-3.1 0-2.9 1.3-2.9 1.3v1.4h5.9v.9H7.6s-1.8.2-2.8 2c-1.1 2-.9 3.5-.9 3.5h1.8v-.9c0-1.1.9-1.3 1.3-1.3h4.9v2.2s.2 1.3-1.3 1.3H8.3s-2 .2-3.1-1.3c-.9-1.3-.9-2.7-.9-2.7H2.5s-.3 2.7 1.5 4.5c1.7 1.7 3.9 1.5 3.9 1.5h1.9v-1.4h5.9v-.9H7.8s-1.8-.2-2.8-2c-1.1-2-.9-3.5-.9-3.5h1.8v.9c0 1.1.9 1.3 1.3 1.3h4.9V11s-.2-1.3 1.3-1.3h2.3s2-.2 3.1 1.3c.9 1.3.9 2.7.9 2.7h1.8s.3-2.7-1.5-4.5C23.1 7.5 20.9 7.7 20.9 7.7H19V9.1h-5.9v.9h7.9s1.8-.2 2.8-2c1.1-2 .9-3.5.9-3.5h-1.8v.9c0 1.1-.9 1.3-1.3 1.3h-4.9V4.5S16.5 3.2 15 3.2H11.9z" opacity="0.95"/></svg>`,
  sh: `<svg class="mac-icon mac-icon--sh" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`,
  config: `<svg class="mac-icon mac-icon--config" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`,
  media: `<svg class="mac-icon mac-icon--media" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`,
  font: `<svg class="mac-icon mac-icon--font" viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M9.5 5h5l5 14h-2.5l-1.2-3.5H8.2L7 19H4.5L9.5 5zm2.5 3.3L9.1 13.5h5.8L12 8.3z"/></svg>`,
  pdf: `<svg class="mac-icon mac-icon--pdf" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="rgba(255,255,255,0.06)"/><polyline points="14 2 14 8 20 8"/><line x1="9" y1="13" x2="15" y2="13"/><line x1="9" y1="17" x2="13" y2="17"/></svg>`,
  img: `<svg class="mac-icon mac-icon--img" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="4" fill="rgba(255,255,255,0.06)"/><circle cx="16" cy="8.5" r="1.8" fill="currentColor"/><path d="M4 18l6.8-6.8a1.6 1.6 0 0 1 2.4 0L20 18"/></svg>`,
  archive: `<svg class="mac-icon mac-icon--archive" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 8v13H3V8" fill="rgba(255,255,255,0.06)"/><path d="M1 3h22v5H1z"/><line x1="10" y1="12" x2="14" y2="12"/></svg>`,
  code: `<svg class="mac-icon mac-icon--code" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="rgba(255,255,255,0.06)"/><polyline points="14 2 14 8 20 8"/><polyline points="10 13 8 15 10 17"/><polyline points="14 13 16 15 14 17"/></svg>`,
  sql: `<svg class="mac-icon mac-icon--sql" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="7" rx="7.5" ry="3.5" fill="rgba(255,255,255,0.06)"/><path d="M4.5 7v10c0 1.93 3.36 3.5 7.5 3.5s7.5-1.57 7.5-3.5V7"/></svg>`,
  doc: `<svg class="mac-icon mac-icon--doc" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="rgba(255,255,255,0.06)"/><polyline points="14 2 14 8 20 8"/><line x1="9" y1="15" x2="15" y2="15"/></svg>`,
};
