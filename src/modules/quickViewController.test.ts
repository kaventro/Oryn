import test from 'node:test';
import assert from 'node:assert/strict';
import { QuickViewController } from './quickViewController.ts';

function createNode(tag = 'div') {
  const children: any[] = [];
  const node: any = {
    tagName: tag.toUpperCase(),
    className: '',
    textContent: '',
    innerHTML: '',
    title: '',
    alt: '',
    src: '',
    type: '',
    controls: false,
    style: {},
    children,
    append(...parts: any[]) {
      children.push(...parts);
    },
    appendChild(child: any) {
      children.push(child);
      return child;
    },
    replaceChildren(...parts: any[]) {
      children.length = 0;
      children.push(...parts);
    },
    addEventListener() {},
  };
  return node;
}

function setupDom() {
  const hosts: Record<string, any> = {
    'list-left': createNode('div'),
    'list-right': createNode('div'),
  };
  const created: any[] = [];
  const orig = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => hosts[id] || null,
    createElement: (tag: string) => {
      const node = createNode(tag);
      node.tag = tag;
      created.push(node);
      return node;
    },
  };
  return {
    hosts,
    created,
    restore: () => {
      globalThis.document = orig;
    },
  };
}

function controllerFor(api: any, item: { base: string; isDir?: boolean; size?: number }) {
  const state: any = {
    left: { quickViewActive: true },
    right: { path: '/pics' },
  };
  return new QuickViewController({
    state,
    api: () => api,
    otherSide: (side) => (side === 'left' ? 'right' : 'left'),
    getFilteredSelection: () => ({ item }),
    fullPath: async () => `/pics/${item.base}`,
  });
}

function bodyOf(host: any) {
  const container = host.children[0];
  return container.children.find((child: any) => String(child.className).includes('quick-view-body'));
}

test('QuickViewController getFileSrc grants a preview URL or returns empty', async () => {
  const granted = controllerFor({ assetUrl: async (p: string) => `asset://${p}` }, { base: 'a.png' });
  assert.equal(await granted.getFileSrc('/pics/a.png'), 'asset:///pics/a.png');
  assert.equal(await granted.getFileSrc(''), '');

  const denied = controllerFor(
    {
      assetUrl: async () => {
        throw new Error('denied');
      },
    },
    { base: 'a.png' },
  );
  assert.equal(await denied.getFileSrc('/pics/a.png'), '');

  const missing = new QuickViewController({
    state: { left: { quickViewActive: true }, right: {} },
    api: () => ({}),
    otherSide: (side) => (side === 'left' ? 'right' : 'left'),
    getFilteredSelection: () => ({ item: null }),
    fullPath: async () => null,
  });
  assert.equal(await missing.getFileSrc('/pics/a.png'), '');
});

test('QuickViewController previews media and falls back when the asset grant fails', async () => {
  const dom = setupDom();
  const calls: string[] = [];
  const api = {
    assetUrl: async (p: string) => {
      calls.push(p);
      if (p.endsWith('denied.png') || p.endsWith('song.mp3') || p.endsWith('clip.mp4')) return '';
      return `asset://${p}`;
    },
    readMediaDataUrl: async (p: string) => {
      if (p.endsWith('broken.png')) throw new Error('decode failed');
      return `data:application/octet-stream;base64,${p}`;
    },
  };

  try {
    const image = controllerFor(api, { base: 'photo.png', size: 20 });
    assert.equal(await image.render('left'), true);
    const imageBody = bodyOf(dom.hosts['list-left']);
    const imageWrap = imageBody.children[0];
    const img = imageWrap.children[0];
    assert.equal(img.src, 'asset:///pics/photo.png');
    img.naturalWidth = 4;
    img.naturalHeight = 5;
    img.onload();
    assert.ok(imageWrap.children[1].textContent.includes('4 × 5'));

    const denied = controllerFor(api, { base: 'denied.png', size: 20 });
    assert.equal(await denied.render('left'), true);
    const deniedImg = bodyOf(dom.hosts['list-left']).children[0].children[0];
    assert.equal(deniedImg.src, 'data:application/octet-stream;base64,/pics/denied.png');
    deniedImg.onerror();
    assert.ok(String(bodyOf(dom.hosts['list-left']).innerHTML).includes('Cannot preview image'));

    const broken = controllerFor(api, { base: 'broken.png', size: 20 });
    assert.equal(await broken.render('left'), true);
    const brokenImg = bodyOf(dom.hosts['list-left']).children[0].children[0];
    await brokenImg.onerror();
    assert.ok(String(bodyOf(dom.hosts['list-left']).innerHTML).includes('Cannot preview image'));

    const audio = controllerFor(api, { base: 'song.mp3', size: 20 });
    assert.equal(await audio.render('left'), true);
    const audioEl = bodyOf(dom.hosts['list-left']).children[0].children[0];
    assert.equal(audioEl.tagName, 'AUDIO');
    assert.equal(audioEl.src, 'data:application/octet-stream;base64,/pics/song.mp3');

    const video = controllerFor(api, { base: 'clip.mp4', size: 20 });
    assert.equal(await video.render('left'), true);
    const videoEl = bodyOf(dom.hosts['list-left']).children[0].children[0];
    assert.equal(videoEl.tagName, 'VIDEO');
    assert.equal(videoEl.src, 'data:application/octet-stream;base64,/pics/clip.mp4');

    const noData = controllerFor({ assetUrl: async () => '' }, { base: 'quiet.mp3', size: 8 });
    assert.equal(await noData.render('left'), true);
    const quiet = bodyOf(dom.hosts['list-left']).children[0].children[0];
    assert.equal(quiet.src, '');
    await quiet.onerror();
    assert.equal(quiet.src, '');
  } finally {
    dom.restore();
  }
});
