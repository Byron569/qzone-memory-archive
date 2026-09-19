# 贡献指南

## 分支

- 从最新 `main` 创建短生命周期分支。
- 文档改动使用 `docs/<主题>`，功能使用 `feat/<主题>`，修复使用 `fix/<主题>`，CI 使用 `ci/<主题>`。
- 不直接推送 `main`；所有改动通过 Pull Request 审查。

## 提交

采用 Conventional Commits：`type(scope): summary`。常用类型为 `feat`、`fix`、`docs`、`refactor`、`test`、`ci` 与 `chore`。摘要使用祈使语气，长度不超过 72 个字符。

## 验证

修改 Vue/Tauri 前端后运行 `npm run build`。修改文档站后运行 `npm --prefix website run build`。涉及 Rust 时还应运行 `cargo fmt --check` 和 `cargo check`。

## Pull Request

- 一个 PR 只解决一个明确的问题。
- 标题遵循 Conventional Commits，并与最近的 `fix:`、`ci:` PR 形式一致。
- 描述说明动机、变更、验证命令、风险或迁移影响。
- 关联 Issue；用户可见变更应附截图或录屏。
- 合并前，所有必需检查必须通过，且不包含无关格式化或生成文件。
