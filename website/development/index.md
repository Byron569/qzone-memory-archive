---
title: 开发
---

# 开发

## 前置条件

- Node.js 20+
- Rust 1.77+
- Windows 开发环境需 WebView2
- Android 开发还需要 Android Studio、SDK 与 NDK

```bash
npm ci
npm run tauri dev
```

## 文档站

文档站是独立工作区，不会参与 Tauri 打包：

```bash
cd website
npm ci
npm run dev
```

文档内容使用 Markdown，放在 `website/` 下的目录索引页。提交前执行 `npm run build`。

## 提交与 Pull Request

遵循仓库的 [贡献指南](https://github.com/xiaosu19/QQ-Zone-Restore-Archive/blob/main/CONTRIBUTING.md)。每个 PR 聚焦一个主题，使用 Conventional Commits 标题，并说明验证方式和风险。
