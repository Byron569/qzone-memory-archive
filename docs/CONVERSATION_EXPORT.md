# QQ 空间恢复项目对话与实施记录

> 导出时间：2026-09-19  
> 项目仓库：https://github.com/Byron569/qzone-memory-archive  
> 当前分支：codex/remote-server-foundation

## 安全说明

本导出文件主动省略了服务器密码、SSH 私钥、注册令牌、设备令牌、QQ Cookie、g_tk 和其他可恢复账号或服务器访问权限的敏感内容。服务器地址和临时密钥路径也不在这里重复记录。若要分享本文件，请先确认没有追加粘贴敏感信息。

## 对话过程

### 1. 项目探索与恢复思路

用户提供 qzone-memory-archive 项目，希望探索可以增加的新功能，以及找回原来 QQ 空间记录的方法。

助手分析项目定位为 QQ 空间本地归档与恢复工具，重点关注互动消息、好友动态、点赞、评论、回复和历史接口。指出一个关键边界：从未产生互动、也没有出现在互动列表中的动态，通常无法仅靠互动接口恢复。

用户提出从“单方面查看自己的互动记录”改为“同时查看互动双方记录”的思路：如果 A 端没有记录、B 端有，就用 B 端记录补足 A 端。

助手认可双端证据交叉验证方向，并建议：

- 两端都只导出经过授权的相关记录；
- 先做稳定 recordId、事件时间、作者、互动类型和媒体指纹匹配；
- 对端证据先进入候选队列；
- 用户确认后再合并，不能静默覆盖本地内容。

### 2. 远程服务器方案

用户提出建设远程服务器，将一方拥有、另一方没有的说说信息上传数据库，另一方刷新后补齐，并询问建设成本。

助手设计远程同步服务：

- 客户端负责读取 QQ 空间和生成证据；
- 服务器只保存配对关系、密文、摘要、版本和同步游标；
- QQ Cookie、密码、g_tk 和原始登录会话绝不上传；
- 使用 PostgreSQL 保存服务端状态；
- 使用一次性配对码建立两台设备关系；
- 使用端到端加密传输证据；
- 服务器不能直接访问 QQ 空间，也不能替客户端绕过权限或风控。

### 3. Git 与版本保护

用户要求先把 Git 弄好，防止修改错误，并保存 GitHub 库内容。

助手创建独立开发分支，保留原有版本和发布分支，所有后续改动通过短期分支提交，不直接修改 main。

此前还处理过：

- macOS 上从错误目录运行 npm run tauri dev 导致找不到 package.json；
- 本地未安装 Rust/Cargo 导致 Tauri 无法执行 cargo metadata；
- 将 macOS 程序构建和版本发布作为独立流程处理。

### 4. 服务器准备

用户提供 Ubuntu 服务器和临时 SSH 公钥，希望配置远程同步服务。

助手通过用户本机临时 SSH 密钥连接服务器，完成以下工作：

- 检查 SSH 服务、监听端口和防火墙状态；
- 安装并确认 PostgreSQL；
- 创建同步数据库和专用数据库角色；
- 将 PostgreSQL 限制为只监听服务器本机；
- 创建 qzone-sync 系统用户；
- 创建 systemd 服务；
- 创建服务端环境变量文件，并设置严格权限；
- 服务 API 监听 127.0.0.1:8787；
- 健康检查返回数据库正常。

腾讯云控制台的公网 5432 规则当时仍待后续收紧或删除；服务端设计本身不需要公网 PostgreSQL。

### 5. 服务端基础能力

助手在 server/ 中实现远程同步服务：

- GET /healthz
- POST /v1/devices
- GET /v1/devices/me
- POST /v1/pairings
- POST /v1/pairings/claim
- GET /v1/pairings
- POST /v1/pairings/{id}/changes
- GET /v1/pairings/{id}/changes
- POST /v1/pairings/{id}/ack

服务器实现了：

- 注册令牌校验；
- 设备令牌哈希存储；
- 配对码哈希存储；
- 幂等变更写入；
- 版本冲突返回；
- 拉取游标；
- tombstone 删除标记；
- PostgreSQL 数据迁移；
- systemd 自动重启。

服务器基础提交：

    d22a9e4 feat(sync): add remote pairing service foundation
    396e193 feat(sync): add encrypted change stream endpoints

### 6. 客户端安全凭据和公钥配对

助手在 Tauri Rust 后端新增远程同步模块：

- 服务器地址存入系统安全凭据库；
- 设备身份存入系统安全凭据库；
- 设备令牌存入系统安全凭据库；
- X25519 私钥只存入系统安全凭据库；
- Vue 层不会读取或持久化这些秘密；
- 删除全部应用数据时同时清理远程同步凭据。

同时新增：

- 设备注册；
- 创建配对码；
- 接受配对码；
- 配对列表；
- 配对双方公钥返回；
- 设置页远程同步区域。

端到端加密使用：

- X25519 派生共享密钥；
- HKDF-SHA256 派生配对密钥；
- XChaCha20-Poly1305 加密；
- AAD 绑定协议版本、配对 ID、记录 ID、版本、密钥版本和操作类型；
- SHA-256 校验密文摘要。

客户端提交：

    a849740 feat(sync): add encrypted client pairing transport

服务器配对公钥迁移和部署也在该阶段完成，并通过真实双设备测试：

    pairing=accepted initiator_key=true claimant_key=true listed=1
    encrypted_stream=accepted:1 pulled:1 ack:1

### 7. 接入恢复候选队列

用户要求继续推进。

助手将远程同步接入现有恢复逻辑：

- 从本地归档按对方 QQ 号生成证据包；
- 证据按批次在客户端加密；
- 加密变更上传服务器；
- 对端按游标拉取；
- 对端在本机解密；
- 重新组装证据包；
- 写入现有 recovery_evidence_packages 和 recovery_evidence_items；
- 进入“待确认恢复候选”；
- 用户手动确认后才合并到正式归档。

设置页新增：

- “加密上传证据”
- “拉取并加入候选”
- 远程同步游标
- 配对状态
- 设备注册状态

最新提交：

    746f85b feat(sync): connect encrypted evidence to recovery queue

## 当前代码与分支

当前分支：

https://github.com/Byron569/qzone-memory-archive/tree/codex/remote-server-foundation

当前提交链：

    746f85b feat(sync): connect encrypted evidence to recovery queue
    a849740 feat(sync): add encrypted client pairing transport
    396e193 feat(sync): add encrypted change stream endpoints
    d22a9e4 feat(sync): add remote pairing service foundation

## 已验证项目

    npm run build
    cargo check --manifest-path src-tauri/Cargo.toml
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --manifest-path server/Cargo.toml
    cargo fmt --manifest-path server/Cargo.toml -- --check

服务端还完成了真实接口回归：

1. 两台设备注册；
2. 创建配对码；
3. 另一台设备接受配对；
4. 上传密文；
5. 对端拉取密文；
6. 确认同步游标；
7. 清理测试数据。

## 尚未完成的下一步

当前服务器 API 仍监听 127.0.0.1，尚未通过公网 HTTPS 暴露。后续需要：

1. 准备域名；
2. 配置 Nginx 或 Caddy 反向代理；
3. 配置 HTTPS 证书；
4. 只开放 443；
5. 删除或限制公网 PostgreSQL 5432 规则；
6. 在客户端填写 HTTPS 服务地址；
7. 用两个实际授权账号做小范围验证；
8. 再考虑自动同步频率、保留期和服务器备份清理。

远程同步仍然遵守“只补缺失，不静默覆盖”的恢复原则。

## 用户原始关键请求（敏感信息已脱敏）

以下保留了当前项目对话中的关键用户请求原意；服务器密码、私钥、令牌和可登录信息均已删除或替换为“已脱敏”。

1. “这个是我的项目，这项目你觉得有什么可以增加新功能的地方？”
2. “你可以探索一下，还有什么方法可以找回原来的记录”
3. “继续”
4. “可以的”
5. “先把git弄好，防止修改错误”
6. “先保存GitHub库里面的东西吧”
7. “OKOK”
8. “来吧，讲讲你要怎么修改”
9. “可以的可以的，这些已经只你能想象到尽力你最大可能进行恢复了是吗？？？”
10. “原本的逻辑是跟好友互动就有记录嘛……现在我想更换思路，通过互动消息，在互动两端同时进行原来信息的查询。懂我意思吗”
11. “可以的，按照你的来”
12. “现在有双方验证这个功能。有时候 a 没有但是 b 有，那就把 b 的那条说说信息上传数据库，然后 a 后面进行重新刷新，发现 b 有 a 没有，就在 a 那边补上。”
13. “可以可以，就按这个来执行，代码的话你多发布几个字代理进行做，这样更快”
14. “可以的，继续吧”
15. “可以git一下，我发布新版本，然后下载一下我测试一下”
16. “还有，到时候要远程服，我的服务器上面需要下载什么数据库进行存储？”
17. “现在先来看看，把分支帮我拉取下来，我用一下看看”
18. 用户报告从错误目录运行 npm run tauri dev，出现找不到 package.json。
19. 用户报告运行 Tauri 时出现 cargo metadata 找不到，说明本机缺少 Rust/Cargo。
20. “上个版本可以直接下载程序，分支帮我弄成这种吧，直接弄成程序算了”
21. “把打包的 mac 程序发布到分支的版本上面 V1.11”
22. “OK的，看来是能用的这个版本，现在我要引入远程服务器了，这个工程量很大”
23. 用户提供服务器登录信息和临时 SSH 公钥（本导出已全部脱敏）。
24. “你可以用我本地mac进行连接服务器啊”
25. 用户提供 SSH 服务状态、22 端口监听和 UFW 状态。
26. “在腾讯云控制台添加入站规则，怎么弄？”
27. “OKOK了，你试试”
28. “可以可以，按你的来”
29. “不管了，后面再来，你先吧产品给弄好，开始你的伟大工程吧”
30. “OK进行下一步吧”
31. “可以的，继续吧”

