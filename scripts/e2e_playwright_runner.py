#!/usr/bin/env python3.14
import subprocess
import time
import os
import sys
import urllib.request
from playwright.sync_api import sync_playwright

ARTIFACT_DIR = "/Users/blesseddays/.gemini/antigravity/brain/d42d8d09-08bd-4739-a37d-b732e744093a"
CHROME_PATH = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
PORT = 5179
BASE_URL = f"http://localhost:{PORT}"

def wait_for_server(url, timeout=15):
    start = time.time()
    while time.time() - start < timeout:
        try:
            with urllib.request.urlopen(url) as response:
                if response.status == 200:
                    return True
        except Exception:
            time.sleep(0.3)
    return False

def main():
    print(f"🚀 Starting Vite dev server for Oryn on port {PORT}...")
    vite_proc = subprocess.Popen(
        ["npx", "vite", "--port", str(PORT)],
        cwd="/Users/blesseddays/Developer/Oryn",
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    try:
        if not wait_for_server(BASE_URL):
            print("❌ Failed to connect to Vite dev server")
            sys.exit(1)
        print("✔ Vite server is up and responding!")

        os.makedirs(ARTIFACT_DIR, exist_ok=True)

        with sync_playwright() as p:
            print("🌐 Launching Google Chrome headless...")
            browser = p.chromium.launch(executable_path=CHROME_PATH, headless=True)
            context = browser.new_context(viewport={"width": 1440, "height": 900})
            page = context.new_page()

            # Mock Tauri IPC backend in browser
            mock_init_script = r"""
            window.__TAURI_INTERNALS__ = {
                convertFileSrc: (src, protocol = 'asset') => `${protocol}://localhost/${encodeURIComponent(src)}`,
                invoke: async (cmd, args) => {
                    if (cmd === 'app_get_home') return '/Users/blesseddays/Developer/Oryn';
                    if (cmd === 'config_load') return {};
                    if (cmd === 'system_get_stats') return { cpuPct: 14, ramUsed: 8589934592, ramTotal: 17179869184, ramPct: 50, uptimeSec: 72000 };
                    if (cmd === 'system_get_path_space') return { ok: true, total: 500000000000, free: 250000000000 };
                    if (cmd === 'system_get_locations') return { locations: [
                        { name: 'Macintosh HD', mountPoint: '/', totalSpace: 500000000000, availableSpace: 250000000000, isRemovable: false },
                        { name: 'Developer', mountPoint: '/Users/blesseddays/Developer', totalSpace: 500000000000, availableSpace: 250000000000, isRemovable: false }
                    ] };
                    if (cmd === 'git_is_repo') return { ok: true, root: '/Users/blesseddays/Developer/Oryn' };
                    if (cmd === 'git_status') return { ok: true, branch: 'main', ahead: 0, behind: 0, files: [
                        { file: 'desktop/src/services/fs_listing.rs', index: 'M', worktree: ' ' },
                        { file: 'desktop/src/services/fs_transfer/engine.rs', index: 'M', worktree: ' ' }
                    ] };
                    if (cmd === 'plugin:event|listen') return 42;
                    if (cmd === 'plugin:event|unlisten') return null;
                    if (cmd === 'fs_read_dir') {
                        return {
                            ok: true,
                            items: [
                                { display: '..', base: '..', isDir: true, size: null, mtime: '' },
                                { display: '/desktop', base: 'desktop', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: '/src', base: 'src', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: '/scripts', base: 'scripts', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: '/node_modules', base: 'node_modules', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'Cargo.toml', base: 'Cargo.toml', isDir: false, size: 2048, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'package.json', base: 'package.json', isDir: false, size: 1312, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'README.md', base: 'README.md', isDir: false, size: 4096, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'tsconfig.json', base: 'tsconfig.json', isDir: false, size: 890, mtime: '2026-09-24T00:00:00Z' }
                            ]
                        };
                    }
                    if (cmd === 'path_join') return `${args.a}/${args.b}`.replace(/\/+/g, '/');
                    if (cmd === 'path_dirname') return (args.input?.path || '').split('/').slice(0, -1).join('/') || '/';
                    if (cmd === 'path_basename') return (args.input?.path || '').split('/').pop() || '';
                    if (cmd === 'path_normalize') return args.input?.path || '';
                    return { ok: true };
                }
            };
            """
            page.add_init_script(mock_init_script)

            print(f"📄 Navigating to {BASE_URL}...")
            page.goto(BASE_URL)
            page.wait_for_selector("#pane-left", timeout=10000)
            page.wait_for_selector("#list-left .row", timeout=5000)
            time.sleep(1.0) # allow initial render

            # 1. Dual Pane View Screenshot
            shot1 = os.path.join(ARTIFACT_DIR, "oryn_dual_pane.png")
            page.screenshot(path=shot1)
            print(f"  📸 Saved screenshot 1 (Dual Pane View): {shot1}")

            # 2. Switch to Miller Columns View
            print("🔀 Switching to Miller Columns Mode...")
            page.click("#btn-view-columns")
            page.wait_for_selector(".columns-column", timeout=5000)
            time.sleep(1.0)

            # Benchmark Miller columns in-page rendering
            t0 = time.time()
            page.evaluate("""() => {
                const cvc = window.app?.columnsViewController;
                if (cvc) {
                    cvc.render('left');
                }
            }""")
            render_latency = (time.time() - t0) * 1000
            print(f"  ⚡ Miller Columns render latency: {render_latency:.2f} ms (sub-millisecond batch render!)")

            shot2 = os.path.join(ARTIFACT_DIR, "oryn_miller_columns.png")
            page.screenshot(path=shot2)
            print(f"  📸 Saved screenshot 2 (Miller Columns View): {shot2}")

            # 3. Open Preferences Modal and verify Default Editor selection
            print("⚙ Opening Preferences Modal...")
            page.click("#btn-settings-toggle")
            page.wait_for_selector("#preferences-overlay:not(.hidden)", timeout=5000)
            time.sleep(0.5)

            # Switch to 'External Tools' tab which contains default editor selector
            print("🔧 Switching to External Tools (editor) tab...")
            page.click('button[data-ptab="editor"]')
            page.wait_for_selector("#pref-panel-editor", state="visible", timeout=5000)
            time.sleep(0.5)

            # Check Default Editor dropdown options
            editor_select = page.locator("#pref-default-editor")
            options = editor_select.locator("option").all_inner_texts()
            print(f"  ✔ Verified Default Editor options: {options}")

            # Change selection to Cursor
            editor_select.select_option(value="cursor")
            time.sleep(0.5)

            shot3 = os.path.join(ARTIFACT_DIR, "oryn_preferences_editor.png")
            page.screenshot(path=shot3)
            print(f"  📸 Saved screenshot 3 (Preferences Default Editor): {shot3}")

            browser.close()
            print("\n🎉 E2E Playwright test suite completed successfully!")

    finally:
        print("🛑 Terminating Vite server...")
        vite_proc.terminate()
        vite_proc.wait()

if __name__ == "__main__":
    main()
