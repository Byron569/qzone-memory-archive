---
title: 概览
description: 了解 QQ Zone Restore Archive 的边界、功能和适用场景。
---

# QQ Zone Restore Archive

QQ Zone Restore Archive 是用于**个人 QQ 空间资料恢复与备份**的本地应用。它将可访问的动态、媒体和互动记录写入设备上的 SQLite 数据库，并支持导出独立 HTML。

本项目由 [https://github.com/xiaosu19](https://github.com/xiaosu19) 维护，基于 [Gaoshu705/QzoneArchive](https://github.com/Gaoshu705/QzoneArchive) 二次开发，并参考 [LibraHp/GetQzonehistory](https://github.com/LibraHp/GetQzonehistory) 的取数思路。本项目不是腾讯官方产品，也不保证能够恢复没有互动痕迹或已被服务端彻底清除的内容。

## 适合什么场景

- 备份本人账号或已获得明确授权的账号资料。
- 将互动记录整理为可离线浏览的个人档案。
- 在设备迁移前导出重要资料。

## 核心原则

所有归档数据保存在本机。使用前请阅读[数据与安全](../data-and-safety/)，并遵守平台规则和适用法律。

::: warning 可恢复范围
归档依赖 QQ 空间可返回的数据。未出现在可访问互动记录中的内容可能无法恢复，归档结果不应作为唯一备份。
:::
