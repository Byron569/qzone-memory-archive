import { defineConfig } from "vitepress";

export default defineConfig({
  lang: "zh-CN",
  title: "QQ Zone Restore Archive",
  description: "把值得留存的 QQ 空间记忆，安静地保存在你的设备上。",
  base: "/QQ-Zone-Restore-Archive/",
  outDir: "./dist",
  themeConfig: {
    nav: [
      { text: "文档", link: "/intro/" },
      { text: "GitHub", link: "https://github.com/xiaosu19/QQ-Zone-Restore-Archive" },
    ],
    sidebar: [
      { text: "开始使用", items: [{ text: "概览", link: "/intro/" }, { text: "安装", link: "/install/" }, { text: "首次归档", link: "/first-archive/" }, { text: "数据与安全", link: "/data-and-safety/" }] },
      { text: "参与项目", items: [{ text: "开发", link: "/development/" }, { text: "发布流程", link: "/release-process/" }] },
    ],
    search: { provider: "local" },
  },
});
