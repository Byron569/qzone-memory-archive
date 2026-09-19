<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import { platform } from "@tauri-apps/plugin-os";
import Button from "primevue/button";
import { useAuthStore } from "../stores/auth";

const authStore = useAuthStore();
const windowOpen = ref(false);

function qzoneUrl() {
  const uin = authStore.user?.uin;
  if (platform() === "android") {
    return uin ? `https://m.qzone.qq.com/${uin}` : "https://m.qzone.qq.com";
  }
  return uin ? `https://user.qzone.qq.com/${uin}` : "https://user.qzone.qq.com";
}

async function openQzoneWindow() {
  try {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("qzone-browser");
    if (existing) {
      await existing.setFocus();
      windowOpen.value = true;
      return;
    }
    const win = new WebviewWindow("qzone-browser", {
      url: qzoneUrl(),
      title: "QQ 空间",
      width: 1000,
      height: 720,
      minWidth: 480,
      minHeight: 500,
      center: true,
    });
    await win.once("tauri://created", () => { windowOpen.value = true; });
    await win.once("tauri://destroyed", () => { windowOpen.value = false; });
    windowOpen.value = true;
  } catch {
    // Fallback: use system browser
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(qzoneUrl());
    windowOpen.value = true;
  }
}

async function closeQzoneWindow() {
  try {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const win = await WebviewWindow.getByLabel("qzone-browser");
    win?.close();
  } catch { /* ignore */ }
  windowOpen.value = false;
}

onMounted(openQzoneWindow);
onBeforeUnmount(closeQzoneWindow);
</script>

<template>
  <div class="qzone-page qzone-external-page">
    <div class="qzone-external">
      <span class="qzone-external-icon"><i class="pi pi-globe" /></span>
      <h3 v-if="windowOpen">QQ 空间已在独立窗口中打开</h3>
      <h3 v-else>正在打开 QQ 空间…</h3>
      <p>由于腾讯限制，QQ 空间无法嵌入到本软件内，将以独立窗口打开。</p>
      <div class="qzone-external-actions">
        <Button
          :label="windowOpen ? '重新打开' : '打开 QQ 空间'"
          :icon="windowOpen ? 'pi pi-refresh' : 'pi pi-external-link'"
          @click="openQzoneWindow"
        />
        <Button
          v-if="windowOpen"
          label="关闭窗口"
          icon="pi pi-times"
          severity="secondary"
          text
          @click="closeQzoneWindow"
        />
      </div>
    </div>
  </div>
</template>
