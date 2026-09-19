import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { getQzoneLoginUser, type LoginCredentials, openWebLogin, checkWebLogin, syncCookiesToWebview } from "../utils/qlogin";
import { useRecycleSessionStore } from "./recycle";

export interface LoginUser {
  uin: string;
  nickname: string;
  avatarImage?: string;
}

interface LoginStatus {
  status: "waiting" | "scanned" | "expired" | "success" | "error" | "loggedOut" | "webLoginOpened" | "webLoginWaiting" | "webLoginCancelled";
  message: string;
  auth?: LoginCredentials;
}

interface QrLoginStart {
  qrImage: string;
}

const delay = (milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds));

export const useAuthStore = defineStore("auth", () => {
  const dialogVisible = ref(false);
  const loading = ref(false);
  const qrImage = ref("");
  const status = ref<LoginStatus["status"]>("loggedOut");
  const message = ref("使用手机 QQ 扫码登录");
  const user = ref<LoginUser>();
  const credentials = ref<LoginCredentials>();
  const webLoginMode = ref(false);
  let pollingRun = 0;
  let restoringSession = false;

  const loggedIn = computed(() => status.value === "success" && Boolean(user.value));

  async function restoreSession() {
    if (restoringSession || loggedIn.value) return;
    restoringSession = true;
    try {
      const result = await invoke<LoginStatus>("get_login_status");
      status.value = result.status;
      message.value = result.message;
      if (result.status === "success" && result.auth) {
        credentials.value = result.auth;
        user.value = await getQzoneLoginUser(result.auth);
        syncCookiesToWebview().catch(() => {});
      }
    } catch {
      status.value = "loggedOut";
    } finally {
      restoringSession = false;
    }
  }

  async function openLogin() {
    dialogVisible.value = true;
    if (!loggedIn.value) await refreshQrCode();
  }

  function closeLogin() {
    dialogVisible.value = false;
    pollingRun += 1;
  }

  async function refreshQrCode() {
    const run = ++pollingRun;
    loading.value = true;
    qrImage.value = "";
    status.value = "waiting";
    message.value = "正在获取登录二维码…";
    try {
      const result = await invoke<QrLoginStart>("start_qr_login");
      if (run !== pollingRun) return;
      qrImage.value = result.qrImage;
      message.value = "请使用手机 QQ 扫描二维码";
      loading.value = false;
      while (run === pollingRun && dialogVisible.value) {
        await delay(1800);
        if (run !== pollingRun || !dialogVisible.value) return;
        const result = await invoke<LoginStatus>("poll_qr_login");
        if (run !== pollingRun) return;
        status.value = result.status;
        message.value = result.message;
        if (result.status === "success") {
          if (!result.auth) throw new Error("Rust 后端未返回完整登录凭证");
          message.value = "正在获取用户资料…";
          credentials.value = result.auth;
          user.value = await getQzoneLoginUser(result.auth);
          syncCookiesToWebview().catch(() => {});
          await delay(700);
          dialogVisible.value = false;
          pollingRun += 1;
          return;
        }
        if (result.status === "expired" || result.status === "error") return;
      }
    } catch (error) {
      if (run !== pollingRun) return;
      status.value = "error";
      message.value = typeof error === "string"
        ? error
        : error instanceof Error
          ? error.message
          : "登录服务暂时不可用";
    } finally {
      if (run === pollingRun) loading.value = false;
    }
  }

  async function startWebLogin() {
    const run = ++pollingRun;
    loading.value = true;
    webLoginMode.value = true;
    qrImage.value = "";
    status.value = "webLoginOpened";
    message.value = "正在打开登录窗口…";
    try {
      const result = await openWebLogin();
      if (run !== pollingRun) return;
      status.value = result.status;
      message.value = result.message;
      loading.value = false;
      while (run === pollingRun && dialogVisible.value && webLoginMode.value) {
        await delay(2000);
        if (run !== pollingRun || !dialogVisible.value || !webLoginMode.value) return;
        const result = await checkWebLogin();
        if (run !== pollingRun) return;
        status.value = result.status;
        message.value = result.message;
        if (result.status === "success") {
          if (!result.auth) throw new Error("后端未返回登录凭证");
          credentials.value = result.auth;
          user.value = await getQzoneLoginUser(result.auth);
          syncCookiesToWebview().catch(() => {});
          await delay(700);
          dialogVisible.value = false;
          webLoginMode.value = false;
          pollingRun += 1;
          return;
        }
        if (result.status === "webLoginCancelled") {
          webLoginMode.value = false;
          return;
        }
        if (result.status === "error") return;
      }
    } catch (error) {
      if (run !== pollingRun) return;
      status.value = "error";
      message.value =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "网页登录服务暂时不可用";
    } finally {
      if (run === pollingRun) loading.value = false;
    }
  }

  function cancelWebLogin() {
    pollingRun += 1;
    webLoginMode.value = false;
    status.value = "loggedOut";
    message.value = "使用手机 QQ 扫码登录";
    loading.value = false;
  }

  async function logout() {
    pollingRun += 1;
    loading.value = true;
    dialogVisible.value = false;
    try {
      await invoke("logout_qzone");
    } finally {
      useRecycleSessionStore().clear();
      user.value = undefined;
      credentials.value = undefined;
      qrImage.value = "";
      status.value = "loggedOut";
      message.value = "尚未登录";
      loading.value = false;
    }
  }

  return { dialogVisible, loading, qrImage, status, message, user, webLoginMode, loggedIn, restoreSession, openLogin, closeLogin, refreshQrCode, startWebLogin, cancelWebLogin, logout };
});
