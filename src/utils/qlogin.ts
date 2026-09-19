import { fetch } from "@tauri-apps/plugin-http";
import { invoke } from "@tauri-apps/api/core";

export interface LoginCredentials {
  uin: string;
  gTk: number;
  cookies: Record<string, string>;
  userAgent: string;
}

export interface WebLoginStatus {
  status: "success" | "error" | "webLoginOpened" | "webLoginWaiting" | "webLoginCancelled";
  message: string;
  auth?: LoginCredentials;
}

export const openWebLogin = () => invoke<WebLoginStatus>("open_web_login");
export const checkWebLogin = () => invoke<WebLoginStatus>("check_web_login");
export const syncCookiesToWebview = () => invoke<void>("sync_cookies_to_webview");

export interface QzoneLoginUser {
  uin: string;
  nickname: string;
  avatarImage?: string;
}

interface UserInfoResponse {
  code: number;
  message?: string;
  data?: Record<string, unknown>;
}

function cookieHeader(cookies: Record<string, string>) {
  return Object.entries(cookies).map(([key, value]) => `${key}=${value}`).join("; ");
}

function parseJsonp(text: string): UserInfoResponse {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start < 0 || end < start) throw new Error("用户资料响应格式不正确");
  const result = JSON.parse(text.slice(start, end + 1)) as UserInfoResponse;
  if (result.code !== 0 || !result.data) {
    throw new Error(result.message || "QQ 用户资料接口返回错误");
  }
  return result;
}

async function decodeResponseText(response: Response) {
  const bytes = new Uint8Array(await response.arrayBuffer());
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    // QQ 空间部分旧接口仍使用 GBK/GB18030 返回中文内容。
    return new TextDecoder("gb18030").decode(bytes);
  }
}

async function requestUserInfo(url: string, auth: LoginCredentials) {
  const response = await fetch(url, {
    method: "GET",
    headers: {
      Accept: "application/json, text/javascript, */*; q=0.01",
      Cookie: cookieHeader(auth.cookies),
      Referer: `https://user.qzone.qq.com/${auth.uin}`,
      "User-Agent": auth.userAgent,
    },
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return parseJsonp(await decodeResponseText(response));
}

async function getUserInfo(auth: LoginCredentials) {
  const vipUrl = new URL("https://h5.qzone.qq.com/proxy/domain/vip.qzone.qq.com/fcg-bin/fcg_get_vipinfo_mobile");
  vipUrl.searchParams.set("get_all", "1");
  vipUrl.searchParams.set("uin", auth.uin);
  vipUrl.searchParams.set("g_tk", String(auth.gTk));
  try {
    return await requestUserInfo(vipUrl.toString(), auth);
  } catch {
    const legacyUrl = new URL("https://h5.qzone.qq.com/proxy/domain/base.qzone.qq.com/cgi-bin/user/cgi_userinfo_get_all");
    legacyUrl.searchParams.set("uin", auth.uin);
    legacyUrl.searchParams.set("vuin", auth.uin);
    legacyUrl.searchParams.set("fupdate", "1");
    legacyUrl.searchParams.set("rd", String(Math.random()));
    legacyUrl.searchParams.set("g_tk", String(auth.gTk));
    return requestUserInfo(legacyUrl.toString(), auth);
  }
}

async function fetchAvatar(url: string, auth: LoginCredentials) {
  const response = await fetch(url, {
    headers: { Cookie: cookieHeader(auth.cookies), "User-Agent": auth.userAgent },
  });
  if (!response.ok) return undefined;
  const bytes = new Uint8Array(await response.arrayBuffer());
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return `data:${response.headers.get("content-type") || "image/jpeg"};base64,${btoa(binary)}`;
}

export async function getQzoneLoginUser(auth: LoginCredentials): Promise<QzoneLoginUser> {
  const response = await getUserInfo(auth);
  const data = response.data!;
  const nickname = String(data.nickname ?? data.nick ?? data.name ?? "QQ 用户");
  const avatarUrl = String(data.avatar ?? data.face ?? `https://q1.qlogo.cn/g?b=qq&nk=${auth.uin}&s=100`);
  let avatarImage: string | undefined;
  try {
    avatarImage = await fetchAvatar(avatarUrl, auth);
  } catch {
    // 头像失败不应阻止登录。
  }
  return { uin: auth.uin, nickname, avatarImage };
}
