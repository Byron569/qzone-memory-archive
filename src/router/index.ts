import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "dashboard",
      component: () => import("../views/DashboardView.vue"),
      meta: { title: "概览" },
    },
    {
      path: "/archives",
      name: "archives",
      component: () => import("../views/ArchivesView.vue"),
      meta: { title: "归档内容" },
    },
    {
      path: "/contacts",
      name: "contacts",
      component: () => import("../views/ContactsView.vue"),
      meta: { title: "联系人" },
    },
    {
      path: "/media",
      name: "media",
      component: () => import("../views/MediaView.vue"),
      meta: { title: "说说媒体" },
    },
    {
      path: "/albums",
      name: "albums",
      component: () => import("../views/LibraryView.vue"),
      meta: { title: "相册", module: "albums" },
    },
    {
      path: "/albums/:albumId",
      name: "album-photos",
      component: () => import("../views/LibraryView.vue"),
      meta: { title: "相册照片", module: "photos" },
    },
    {
      path: "/videos",
      name: "videos",
      component: () => import("../views/LibraryView.vue"),
      meta: { title: "视频", module: "videos" },
    },
    {
      path: "/guestbook",
      name: "guestbook",
      component: () => import("../views/LibraryView.vue"),
      meta: { title: "留言板", module: "guestbook" },
    },
    {
      path: "/favorites",
      name: "favorites",
      component: () => import("../views/LibraryView.vue"),
      meta: { title: "收藏", module: "favorites" },
    },
    {
      path: "/tasks",
      name: "tasks",
      component: () => import("../views/TasksView.vue"),
      meta: { title: "归档任务" },
    },
    {
      path: "/recycle-bin",
      name: "recycle-bin",
      component: () => import("../views/RecycleBinView.vue"),
      meta: { title: "相册回收站" },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/SettingsView.vue"),
      meta: { title: "设置" },
    },
  ],
});

export default router;
