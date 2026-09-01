# Oryn Keyboard Shortcuts Reference

A comprehensive, categorized guide to default keyboard shortcuts in **Oryn**.

> [!TIP]
> All hotkeys can be customized via **Settings & Preferences (`⌘,` / `Ctrl+,`) → Hotkeys**.

---

## 🗂 Navigation & Pane Management

| Action | macOS Shortcut | Windows / Linux Shortcut | Description |
| :--- | :--- | :--- | :--- |
| **Switch Active Panel** | `Tab` | `Tab` | Toggle active focus between Left and Right panes |
| **Swap Panels** | `⌘U` | `Ctrl+U` | Swap Left and Right directories |
| **Go to Root** | `⌘\` | `Ctrl+\` | Navigate to root drive / filesystem root |
| **Flat Branch View** | `⌘B` | `Ctrl+B` | Display flat list of all subdirectories and files |
| **List View** | `⌘1` | `Ctrl+1` | High-density virtualized list mode |
| **Grid / Icons View** | `⌘2` | `Ctrl+2` | Visual icon grid view mode |
| **Miller Columns View** | `⌘3` | `Ctrl+3` | Cascading column hierarchy navigation |
| **History Back** | `⌘[` or `Alt+←` | `Alt+Left` | Navigate to previously visited folder |
| **History Forward** | `⌘]` or `Alt+→` | `Alt+Right` | Navigate forward in folder history |
| **Parent Directory** | `Backspace` or `⌘↑` | `Backspace` or `Alt+Up` | Go up one directory level (`..`) |
| **Change Drive (Left)** | `⌥F1` | `Alt+F1` | Open drive selection popup for Left pane |
| **Change Drive (Right)** | `⌥F2` | `Alt+F2` | Open drive selection popup for Right pane |
| **Reload / Refresh** | `⌘R` | `Ctrl+R` | Refresh directory listings in open panes |
| **Edit Address Bar** | `⌘L` | `Ctrl+L` | Focus address bar to edit or paste path directly |
| **Open in New Tab** | `⌘T` | `Ctrl+T` | Duplicate active folder in a new tab |
| **Close Tab** | `⌘W` | `Ctrl+W` | Close active tab |
| **Cycle Tabs** | `Ctrl+Tab` / `Ctrl+⇧Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` | Switch between open tabs |

---

## 📁 File Operations & Selection

| Action | macOS Shortcut | Windows / Linux Shortcut | Description |
| :--- | :--- | :--- | :--- |
| **Quick Look / Preview** | `Space` or `F3` | `Space` or `F3` | Open file preview (or calculate folder size) |
| **Quick View Pane** | `⌘Q` | `Ctrl+Q` | Toggle persistent preview panel in opposite pane |
| **Calculate Folder Size** | `Space` (on folder) | `Space` (on folder) | Calculate directory size in background |
| **Toggle Selection** | `Insert` | `Insert` | Select/deselect item and advance cursor down |
| **Select All** | `⌘A` | `Ctrl+A` | Select all files and folders in active pane |
| **Select by Pattern** | `⌥S` | `Alt+S` | Select items matching a wildcard/mask (e.g. `*.png`) |
| **Deselect by Pattern**| `⌥D` | `Alt+D` | Deselect items matching wildcard pattern |
| **Invert Selection** | `⌥I` | `Alt+I` | Invert current selection in active pane |
| **Clear Selection** | `⌥A` | `Alt+A` | Deselect all items |
| **Rename** | `F2` | `F2` | In-place rename selected item |
| **Open in Editor** | `F4` | `F4` | Open selected file in external code editor |
| **Copy** | `F5` | `F5` | Copy selection to opposite pane |
| **Clone / Duplicate** | `⇧F5` | `Shift+F5` | Clone file/folder in the same directory |
| **Compress to ZIP** | `⌥F5` | `Alt+F5` | Pack selected files into a `.zip` archive |
| **Move** | `F6` | `F6` | Move selection to opposite pane |
| **Extract Archive** | `⌥F9` | `Alt+F9` | Extract archive into opposite pane |
| **Create Folder** | `F7` | `F7` | Create new directory (`mkdir`) |
| **Create File** | `⇧F7` | `Shift+F7` | Create new blank file |
| **Delete (Trash)** | `F8` or `Delete` | `F8` or `Delete` | Move selection to OS Trash / Recycle Bin |
| **Delete (Permanent)**| `⇧F8` or `⇧Delete`| `Shift+F8` or `Shift+Delete` | Permanently delete file bypassing Trash |
| **Properties** | `⌥Enter` or `⌘I` | `Alt+Enter` | Open file properties / permissions inspector |

---

## 🛠 Tools, Search & Overlays

| Action | macOS Shortcut | Windows / Linux Shortcut | Description |
| :--- | :--- | :--- | :--- |
| **Command Palette** | `⌘P` or `⌘K` | `Ctrl+P` or `Ctrl+K` | Spotlight-style launcher for all actions |
| **Find / Search Files**| `⌥F7` | `Alt+F7` | Multi-threaded name & regex content search |
| **Quick Filter Bar** | `⌘F` | `Ctrl+F` | Instant live name filter on active pane |
| **Multi-Rename** | `⌘M` | `Ctrl+M` | Batch pattern & regex file renamer |
| **Git Panel** | `⌘G` | `Ctrl+G` | Open Git repository inspector, diff, and blame |
| **Directory Compare** | `⌘D` | `Ctrl+D` | Side-by-side directory difference comparison |
| **Disk Space Analyzer**| `⌘⇧D` | `Ctrl+Shift+D` | Visual treemap disk space scanner |
| **Terminal Drawer** | `F9` or `Ctrl+\`` | `F9` or `Ctrl+\`` | Integrated command terminal drawer |
| **Remote Connections** | `⌘⇧S` | `Ctrl+Shift+S` | SFTP and SSH cloud server profiles |
| **Settings / Hotkeys** | `⌘,` | `Ctrl+,` | Preferences, themes, and keymap configuration |

---

## ⚡ Additional Keyboard Capabilities

### Quick Type-to-Jump
- While a pane list is focused, typing any letters or digits immediately jumps to the first matching file or folder name.

### VIM Navigation Mode
*(Can be enabled in Preferences)*
- `j` — Move cursor down
- `k` — Move cursor up
- `l` — Enter directory / execute item
- `h` — Go to parent directory
