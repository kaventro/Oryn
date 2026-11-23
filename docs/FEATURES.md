# Oryn features

## Workspace

- Two independent panes, each with its own tabs, history and selection. Switch with Tab, or go single-pane with Ctrl+\\.
- List, grid and column views. Grid view shows image thumbnails.
- A sidebar with favourites, drives, saved SFTP servers and colour tags. Add a folder to Favourites with + or Ctrl+D, remove it with the × on hover, and click a section title to collapse it.
- Breadcrumbs, Go to Folder (Ctrl+L or the pencil in the path bar), a filter box per pane, and a command palette (Ctrl+P) that runs any command or jumps to a typed path such as `~/Downloads`.
- The More menu (…) holds view, sort and hidden-file options and the folder commands.
- Git status letters and tag colours appear next to each file.
- Oryn reopens the folders you had open last time.
- Folders refresh on their own when files change on disk.
- Six accent colours and the other preferences live in Settings (Ctrl+,).

## File operations

- Enter or double-click opens a file in its default app. Archives (`.zip`, `.tar`, `.tar.gz`) open like folders; you can preview, open and copy out the files inside.
- Cut, Copy and Paste (Ctrl+X, Ctrl+C, Ctrl+V), Swap Panes (Ctrl+U), Open in Terminal, and Properties (Alt+Enter).
- Copy or move to the other pane (F5, F6), drag and drop between panes and onto folders, duplicate, rename, new file and new folder.
- Conflicts ask whether to skip or replace before anything is overwritten.
- Long operations run in the background with a progress bar and a Cancel button.
- Move to Trash or delete permanently, with an optional confirmation.
- Batch rename with a live preview.
- Compress a selection to ZIP, and extract `.zip`, `.tar` and `.tar.gz` archives into a new folder.
- Select by mask, by extension, invert, or select all.

## Quick View

Press F3 or Ctrl+Y. Arrow keys step through the folder.

- Code with syntax highlighting, plain text, and rendered Markdown.
- Images, office documents (text extracted from Word, Excel and PowerPoint files), and info cards for audio, video and PDF with an Open button.
- A hex view for any file (Tab).

## Power tools

- **Search** (Ctrl+Shift+F) by name or contents, with regex and case options.
- **Find and replace** (Ctrl+Shift+H) across files, with a preview before writing.
- **Git** (Ctrl+Shift+G): status, stage, unstage, discard, commit, diff, branches, pull and push.
- **Compare folders** (Ctrl+Shift+D) and sync them in either direction or mirror them.
- **Duplicates** (Ctrl+Shift+U) finds identical files by content and moves extras to the Trash.
- **Checksums** (Ctrl+Shift+K): SHA-256 and SHA-512, with a field to paste an expected value.
- **Disk space** (Ctrl+Shift+S) shows what takes up room in a folder.
- **Tags**: colour-tag files and list everything with a tag.
- **SFTP** (Ctrl+Shift+R): saved profiles, host key check on first connect, browse, download and upload.

## Terminal

Ctrl+\` or F9 opens a terminal drawer in the active pane's folder. It follows the pane as you move, restarts after the shell exits, and supports paste.

## Updates

Oryn checks GitHub for a newer release when it starts and shows an Update button in the header. Turn this off in Settings.
