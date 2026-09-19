---
title: 发布流程
---

# 发布流程

1. 在功能分支完成一个可审查的变更。
2. 运行应用和文档站各自的构建检查。
3. 提交使用 Conventional Commits，例如 `docs: add installation guide`。
4. 使用 PR 模板说明动机、测试、风险与关联 Issue。
5. 合并至 `main` 后，文档工作流会发布 GitHub Pages；推送 `v*` 标签时，发布工作流构建 Windows、macOS、Linux 与移动端安装包。

文档部署只使用 `main` 分支的 `website/` 内容。开发或预览中的文档分支不会覆盖公开站点。

Linux 发布产物包括：

- `.deb`：Debian / Ubuntu
- `.rpm`：Fedora / openSUSE / RHEL 系
- `.AppImage`：通用 Linux 包

本地制作 Linux 包时，需要先安装 Linux 系统依赖，再执行：

```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
npm ci
npx tauri build --bundles appimage,deb,rpm
```
