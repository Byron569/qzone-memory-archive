# Qzone Sync Server

这是 QQ 空间双端恢复的远程同步服务基础版。服务器只保存设备信息、配对关系和客户端加密后的同步载荷；QQ Cookie、`g_tk`、密码和二维码票据永远不应上传。

## 当前能力

- `GET /healthz`：数据库健康检查
- `POST /v1/devices`：使用注册令牌注册设备，并返回一次性设备令牌
- `GET /v1/devices/me`：读取当前设备信息
- `POST /v1/pairings`：创建 15 分钟有效的配对码
- `POST /v1/pairings/claim`：使用配对码完成双端配对
- `GET /v1/pairings`：列出当前设备参与的配对
- `POST /v1/pairings/{id}/changes`：幂等上传客户端加密变更
- `GET /v1/pairings/{id}/changes?cursor=0&limit=100`：按服务端游标拉取对端变更
- `POST /v1/pairings/{id}/ack`：确认客户端已安全处理游标
- PostgreSQL migrations：设备、配对、同步载荷、游标、删除标记和审计表

同步载荷表中的 `payload` 必须是客户端端到端加密后的二进制；服务端不会尝试解析 QQ 内容。
服务端游标与客户端记录版本分开维护，同一 `recordId` 的重复上传会幂等接受，版本冲突会返回冲突列表。

## 本地运行

```bash
cp .env.example .env
# 编辑 DATABASE_URL 和 REGISTRATION_TOKEN
cargo run --manifest-path server/Cargo.toml
```

注册设备需要发送 `X-Registration-Token`。返回的 `device_token` 只展示一次，应保存在客户端系统安全存储中。

## 部署原则

- API 只通过 HTTPS 对公网开放，PostgreSQL 仅监听 `127.0.0.1`。
- `REGISTRATION_TOKEN` 和 `DATABASE_URL` 只能放在服务器上的 `600` 权限环境文件中。
- 先完成双端配对和同步协议测试，再开放公网入口。
