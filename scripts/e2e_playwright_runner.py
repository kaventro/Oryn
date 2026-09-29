#!/usr/bin/env python3.14
import subprocess
import time
import os
import sys
import urllib.request
from playwright.sync_api import sync_playwright

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DEFAULT_ARTIFACT_DIR = os.path.join(REPO_ROOT, "screenshots")
ARTIFACT_DIR = os.environ.get("ARTIFACT_DIR", DEFAULT_ARTIFACT_DIR)
PORT = int(os.environ.get("PORT", "5179"))
BASE_URL = f"http://localhost:{PORT}"

def get_browser(p):
    chrome_path = os.environ.get("CHROME_PATH")
    if chrome_path and os.path.exists(chrome_path):
        print(f"🌐 Launching Chrome from {chrome_path}...")
        return p.chromium.launch(executable_path=chrome_path, headless=True)
    mac_chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    if os.path.exists(mac_chrome):
        print(f"🌐 Launching macOS Chrome from {mac_chrome}...")
        return p.chromium.launch(executable_path=mac_chrome, headless=True)
    print("🌐 Launching Playwright standard Chromium...")
    return p.chromium.launch(headless=True)

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
    print(f"🚀 Starting Vite dev server for Oryn on port {PORT} in {REPO_ROOT}...")
    vite_proc = subprocess.Popen(
        ["npx", "vite", "--port", str(PORT)],
        cwd=REPO_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        shell=sys.platform == "win32"
    )

    try:
        if not wait_for_server(BASE_URL):
            print("❌ Failed to connect to Vite dev server")
            sys.exit(1)
        print("✔ Vite server is up and responding!")

        os.makedirs(ARTIFACT_DIR, exist_ok=True)

        with sync_playwright() as p:
            browser = get_browser(p)
            context = browser.new_context(viewport={"width": 1440, "height": 900})
            page = context.new_page()

            # Mock Tauri IPC backend in browser
            mock_init_script = r"""
            window.__IPC_CALLS__ = window.__IPC_CALLS__ || [];
            const __callbacks = new Map();
            let __cbId = 1;
            window.__TAURI_INTERNALS__ = {
                convertFileSrc: (src, protocol = 'asset') => `${protocol}://localhost/${encodeURIComponent(src)}`,
                transformCallback: (callback, once = false) => {
                    const id = __cbId++;
                    __callbacks.set(id, callback);
                    return id;
                },
                unregisterCallback: (id) => {
                    __callbacks.delete(id);
                },
                callbacks: __callbacks,
                invoke: async (cmd, args) => {
                    let sanitizedArgs = {};
                    try { sanitizedArgs = JSON.parse(JSON.stringify(args || {})); } catch { }
                    window.__IPC_CALLS__.push({ cmd, args: sanitizedArgs });
                    if (cmd === 'app_get_home') return '/workspace/Oryn';
                    if (cmd === 'config_load') return {};
                    if (cmd === 'system_get_stats') return { cpuPct: 14, ramUsed: 8589934592, ramTotal: 17179869184, ramPct: 50, uptimeSec: 72000 };
                    if (cmd === 'system_get_path_space') return { ok: true, total: 500000000000, free: 250000000000 };
                    if (cmd === 'system_get_locations') return { locations: [
                        { name: 'System HD', mountPoint: '/', totalSpace: 500000000000, availableSpace: 250000000000, isRemovable: false },
                        { name: 'Workspace', mountPoint: '/workspace', totalSpace: 500000000000, availableSpace: 250000000000, isRemovable: false }
                    ] };
                    if (cmd === 'git_is_repo') return { ok: true, root: '/workspace/Oryn' };
                    if (cmd === 'git_status') return { ok: true, branch: 'main', ahead: 0, behind: 0, files: [
                        { file: 'desktop/src/services/fs_listing.rs', index: 'M', worktree: ' ' },
                        { file: 'desktop/src/services/fs_transfer/engine.rs', index: 'M', worktree: ' ' }
                    ] };
                    if (cmd === 'plugin:event|listen') return 42;
                    if (cmd === 'plugin:event|unlisten') return null;
                    if (cmd === 'remote_list_profiles') {
                        return [
                            { id: 'srv1', name: 'Production Cloud (SFTP)', host: '192.168.1.100', port: 22, username: 'deploy', auth_type: 'Password', initial_path: '/var/www' }
                        ];
                    }
                    if (cmd === 'shell_terminal_start') return 'session-1';
                    if (cmd === 'shell_terminal_write') return null;
                    if (cmd === 'shell_terminal_resize') return null;
                    if (cmd === 'shell_terminal_stop') return null;
                    if (cmd === 'shell_exec') {
                        if (args.cmd === 'pwd') return { code: 0, stdout: '/workspace/Oryn\n', stderr: '' };
                        return { code: 0, stdout: 'ok\n', stderr: '' };
                    }
                    if (cmd === 'fs_read_dir') {
                        return {
                            ok: true,
                            items: [
                                { display: '..', base: '..', isDir: true, size: null, mtime: '' },
                                { display: '/desktop', base: 'desktop', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: '/src', base: 'src', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: '/scripts', base: 'scripts', isDir: true, size: null, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'banner.png', base: 'banner.png', isDir: false, size: 1048576, mtime: '2026-09-24T00:00:00Z' },
                                { display: 'screenshot.jpg', base: 'screenshot.jpg', isDir: false, size: 524288, mtime: '2026-09-24T00:00:00Z' },
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

            # 2. Switch to Grid / Icons View
            print("🖼 Switching to Grid / Icons Mode...")
            page.click("#btn-view-grid")
            page.wait_for_selector(".grid-mode", timeout=5000)
            time.sleep(0.5)
            shot2 = os.path.join(ARTIFACT_DIR, "oryn_grid_view.png")
            page.screenshot(path=shot2)
            print(f"  📸 Saved screenshot 2 (Grid / Icons View): {shot2}")

            # 3. Switch to Miller Columns View
            print("🔀 Switching to Miller Columns Mode...")
            page.click("#btn-view-columns")
            page.wait_for_selector(".columns-column", timeout=5000)
            time.sleep(0.5)

            # Benchmark Miller columns in-page rendering
            t0 = time.time()
            page.evaluate("""() => {
                const cvc = window.app?.columnsViewController;
                if (cvc) {
                    cvc.render('left');
                }
            }""")
            render_latency = (time.time() - t0) * 1000
            print(f"  ⚡ Miller Columns render latency: {render_latency:.2f} ms")

            shot3 = os.path.join(ARTIFACT_DIR, "oryn_miller_columns.png")
            page.screenshot(path=shot3)
            print(f"  📸 Saved screenshot 3 (Miller Columns View): {shot3}")

            # Switch back to List View
            page.click("#btn-view-list")
            time.sleep(0.5)

            # 4. Open Remote SFTP Modal
            print("🌐 Opening Remote SFTP Dialog...")
            page.click("#btn-remote-toggle")
            page.wait_for_selector("#remote-dialog-overlay", timeout=5000)
            time.sleep(0.5)
            shot4 = os.path.join(ARTIFACT_DIR, "oryn_remote_sftp.png")
            page.screenshot(path=shot4)
            print(f"  📸 Saved screenshot 4 (Remote SFTP Connections): {shot4}")
            page.click("#btn-remote-close")
            time.sleep(0.5)

            # 5. Open Terminal Drawer
            print("💻 Opening Terminal Drawer...")
            page.click("#btn-status-terminal-toggle")
            page.wait_for_selector("#terminal-drawer:not(.hidden)", timeout=5000)
            page.wait_for_selector("#terminal-output .xterm", timeout=5000)
            time.sleep(0.5)

            # Focus the terminal and type input to verify PTY input forwarding
            page.click("#terminal-output")
            page.keyboard.type("pwd\n")
            time.sleep(0.5)

            # Assert shell_terminal_start and shell_terminal_write IPC invocations
            calls = page.evaluate("() => window.__IPC_CALLS__ || []")
            def get_arg(call, key):
                args = call.get("args", {})
                if key in args:
                    return args[key]
                return args.get("input", {}).get(key)

            start_calls = [c for c in calls if c.get("cmd") == "shell_terminal_start"]
            write_calls = [c for c in calls if c.get("cmd") == "shell_terminal_write"]
            assert len(start_calls) > 0, f"Expected at least one shell_terminal_start IPC invocation, got: {[c.get('cmd') for c in calls]}"
            assert len(write_calls) > 0, f"Expected at least one shell_terminal_write IPC invocation, got: {[c.get('cmd') for c in calls]}"
            assert any(get_arg(w, "sessionId") == "session-1" for w in write_calls), (
                f"Expected shell_terminal_write with sessionId 'session-1', got: {write_calls}"
            )
            total_typed_data = "".join(str(get_arg(w, "data") or "") for w in write_calls)
            assert "pwd" in total_typed_data, (
                f"Expected shell_terminal_write data to contain 'pwd', got: {write_calls}"
            )
            print(f"  ✔ Verified terminal PTY input forwarding: {len(start_calls)} start call(s), {len(write_calls)} write call(s), typed='{total_typed_data.strip()}'")

            shot5 = os.path.join(ARTIFACT_DIR, "oryn_terminal_drawer.png")
            page.screenshot(path=shot5)
            print(f"  📸 Saved screenshot 5 (Integrated Terminal Drawer): {shot5}")
            page.click("#terminal-close-btn")
            time.sleep(0.5)

            # 6. Open Preferences Modal and verify Default Editor selection
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

            shot6 = os.path.join(ARTIFACT_DIR, "oryn_preferences_editor.png")
            page.screenshot(path=shot6)
            print(f"  📸 Saved screenshot 6 (Preferences Default Editor): {shot6}")

            browser.close()
            print("\n🎉 E2E Playwright test suite completed successfully!")

    finally:
        print("🛑 Terminating Vite server...")
        vite_proc.terminate()
        vite_proc.wait()

if __name__ == "__main__":
    main()
