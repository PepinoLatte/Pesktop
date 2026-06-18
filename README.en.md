# Dasktop

English | [中文](README.md)

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Windows 11](https://img.shields.io/badge/Windows-11-0078D4.svg)](#requirements) [![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB.svg)](https://tauri.app/) [![Vue 3](https://img.shields.io/badge/Vue-3-42B883.svg)](https://vuejs.org/) [![TypeScript](https://img.shields.io/badge/TypeScript-6-3178C6.svg)](https://www.typescriptlang.org/)

Dasktop is a lightweight Windows desktop organizer. It uses movable, resizable Boxes to group desktop files, folders, and shortcuts by purpose, keeping the desktop tidy while preserving the original file locations and Windows opening behavior.

Download the latest installer from [GitHub Releases](https://github.com/AugSakura/Dasktop/releases). See the full release history in [CHANGELOG.md](CHANGELOG.md).

![Dasktop cover](docs/images/Dasktop_Cover.png)



## Name Origin

The name Dasktop is a small signature hidden inside the desktop. It starts with `Da` from the author's name and lands on the Desktop that people open every day. The result is a tool with a personal mark and a direct goal: make the desktop feel less like a pile of files and more like a controllable workspace.

## Positioning

Dasktop is designed for users who often keep temporary materials, project files, app shortcuts, or client documents on the desktop. It is not a traditional file manager. It is an organization layer on top of the Windows desktop, so users can keep their familiar workflow while using Boxes to find different types of content more quickly.

## Core Features

- **Desktop grouping:** Create multiple Boxes and organize desktop items by work, projects, materials, tools, or other scenarios.
- **Free placement:** Move and resize Boxes on the desktop, with snapping support near screen edges or other Boxes.
- **Quick collection:** Drag desktop files into a Box to create a clear visual grouping.
- **Native system behavior:** Files still open with the Windows default apps, and context menus keep native Windows capabilities.
- **Adjustable appearance:** Switch between light mode, dark mode, and system theme, and tune opacity, radius, icon size, text size, and spacing.
- **Cleaner desktop:** Optionally hide native desktop icons and keep only the Dasktop organization view visible.
- **Startup restore:** Enable launch on startup and automatically restore the last desktop layout after signing in to Windows.
- **Tray controls:** Use the system tray to open settings, create a Box, toggle desktop icon visibility, or quit the app.

## How To Use

1. Open Dasktop and enter the settings window.
2. Click **Add Box** to create a desktop group.
3. Drag desktop files, folders, or shortcuts into the Box.
4. Adjust the Box position, size, appearance, and lock state as needed.
5. Enable **Launch on startup** to restore the organized desktop layout after the next Windows sign-in.

## Safety Notes

- Deleting a Box only removes the group. It does not delete the real files.
- Hiding native desktop icons only changes visibility. It does not move, empty, or delete desktop files.
- Opening files and using context menus rely on Windows system capabilities, keeping the experience aligned with the native desktop.

## Requirements

- Operating system: Windows
- App name: Dasktop

## Feedback

Dasktop is still in an early public version. If you run into file drag-and-drop issues, runtime dependency problems, window layering behavior, uninstall leftovers, or compatibility issues across Windows versions, feel free to open an [Issue](https://github.com/AugSakura/Dasktop/issues) with reproduction steps.

## Author

- Developer: Aug_Sakura
- Personal blog: [余某人 | 咕咕咕中~](https://www.syand.cn/)

## License

This project is open source under the [MIT License](LICENSE).
