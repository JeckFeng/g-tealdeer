# Tealdeer-Tile 用户答疑报告

## 1) 安装前是否必须预装 tldr / tealdeer？为什么
不强制，取决于你是否打包了 sidecar。

- 如果安装包**包含 sidecar**（`src-tauri/tauri.conf.json` 里的 `externalBin: ["bin/tldr"]`，并且你在打包前放了 `src-tauri/bin/tldr-<target>`），Tealdeer-Tile 会直接调用这个内置 `tldr`，不需要系统预装任何 tldr/tealdeer。
- 如果安装包**没有 sidecar**，应用会去系统里找 `tldr` 或 `tealdeer`（代码里优先找系统命令），找不到就会报错 “No tealdeer binary found”。因此这时必须预装其一。

原理：后端调用 `tldr` 是通过二进制执行，而不是内嵌库。应用只是 UI + 调度层。

## 2) 安装前已安装 tldr 会冲突吗？为什么
一般不会“运行冲突”，但有两类需要注意：

- **运行优先级**：系统已安装 `tldr` 时，应用会优先使用系统版本（代码里系统优先于 sidecar）。这意味着配置、缓存、custom pages 都走系统路径，不会自动隔离。
- **安装冲突**：如果你的安装包包含 sidecar，并且是 deb/rpm 这类系统安装包，它会安装 `/usr/bin/tldr`。如果系统已有 `tldr`，包管理器可能提示冲突或覆盖。

结论：不会导致应用崩溃，但可能影响“隔离性”或产生包管理冲突。

## 3) 安装前已安装 tealdeer 会冲突吗？为什么
逻辑同上：系统有 `tealdeer` 时，应用会把它当作系统后端优先使用。

- **运行行为**：Tealdeer-Tile 会调用系统 `tealdeer`，使用其配置与缓存。
- **安装冲突**：如果你的包里也安装了 `/usr/bin/tldr`，不一定和 `tealdeer` 包直接冲突，但可能和已有 `tldr` 包冲突。

## 4) Tealdeer-Tile 是否提供终端命令？会冲突吗
Tealdeer-Tile 本体是 GUI 应用，不提供新的 CLI 命令。

- 终端里可以直接运行它的可执行文件（如 `tealdeer-widget`），但只是启动 UI，并不是一个 CLI 工具。
- 真正的 CLI 命令是 `tldr`，如果安装包带 sidecar，它可能会在系统里安装 `tldr`，这才是可能与现有 `tldr`/`tealdeer` 发生包冲突的来源。

## 5) 离线是否可用？哪些功能必须联网
**可离线使用**，前提是本地已有缓存。

- **可离线**：搜索/渲染已缓存页面、自定义 page/patch 的创建与管理、查看路径、日志等。
- **必须联网**：`tldr --update`（更新缓存）、首次下载缓存、自动更新。

如果设备从未更新过缓存，离线时会提示“缓存不存在”，需先在线执行一次更新。

## 6) 安装包大小与运行内存
这两项与打包类型、是否带 sidecar、是否包含 WebView 依赖有关，无法给出固定值，但可以评估范围：

- **包大小**：
  - AppImage 往往较大（可能几十到上百 MB），因为会打包部分依赖。
  - deb/rpm 通常更小，但依赖系统库。
  - 你可以在 `src-tauri/target/release/bundle/` 下用 `ls -lh` 查看实际包体积。

- **运行内存**：
  - Tauri 使用系统 WebView（Linux 下通常是 WebKit2GTK），内存占用主要由 WebView 决定。
  - 轻量 UI 通常在几十到数百 MB 范围波动，具体受桌面环境、GPU 加速、页面内容复杂度影响。
  - 可用 `ps`/`htop` 查看实际 RSS。

---

如需进一步避免系统 tldr 冲突，可考虑：
1) 使用 AppImage 运行（不写入 `/usr/bin`）；
2) 打包时改用不同的 sidecar 名称并修改应用查找逻辑；
3) 允许用户在 UI 中选择“系统后端/sidecar 后端”。
