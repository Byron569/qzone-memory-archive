# 第三方项目与许可证说明

QQ Zone Restore Archive 基于或参考了以下开源项目。项目名称、作者与许可证归各自权利人所有；这些项目及其作者不对本项目提供背书或担保。

## 主要上游

- [Gaoshu705/QzoneArchive](https://github.com/Gaoshu705/QzoneArchive) — GPL-3.0；本项目的主要上游与二次开发基础。

## 接口与解析参考

- [LibraHp/GetQzonehistory](https://github.com/LibraHp/GetQzonehistory) — 参考旧历史消息、评论及互动数据的读取与解析思路。
- [ShunCai/QZoneExport](https://github.com/ShunCai/QZoneExport) — Apache License 2.0；参考相册、照片、视频、留言板和收藏等 QQ 空间资料接口的分页参数与响应结构。
- [salt-fishes/qzone-archiver](https://github.com/salt-fishes/qzone-archiver) — 参考 QQ 空间数据归档思路。
- [11273/QzonePhoto](https://github.com/11273/QzonePhoto) — GPL-3.0-only；参考当前 QQ 空间旧收藏接口在无 `qzonetoken` 时使用登录 Cookie 与 `g_tk` 的兼容请求契约。
- [Gu-Heping/onebot-qzone](https://github.com/Gu-Heping/onebot-qzone) — 参考部分 QQ 空间数据结构与解析思路。

本仓库整体继续按根目录 [LICENSE](LICENSE) 中的 GPL-3.0 条款发布。Apache-2.0 参考项目的许可证原文可在其上游仓库中查看：[QZoneExport LICENSE](https://github.com/ShunCai/QZoneExport/blob/master/LICENSE)。
