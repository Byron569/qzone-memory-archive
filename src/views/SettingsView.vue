<script setup lang="ts">
import { storeToRefs } from "pinia";
import { onMounted, ref, watch } from "vue";
import Button from "primevue/button";
import Dialog from "primevue/dialog";
import InputNumber from "primevue/inputnumber";
import InputText from "primevue/inputtext";
import { getVersion } from "@tauri-apps/api/app";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useAuthStore } from "../stores/auth";
import { DEFAULT_ARCHIVE_INTERVAL, MIN_ARCHIVE_INTERVAL, getArchiveInterval, resetAppSettings, setArchiveInterval } from "../utils/appSettings";
import { deleteAllAppData, exportRecoveryEvidence, getRemoteSyncState, importRecoveryEvidence, listRecoveryEvidenceCandidates, listRecoveryEvidencePackages, mergeRecoveryEvidenceItem, type RecoveryEvidenceCandidate, type RecoveryEvidencePackageSummary, type RemoteSyncState } from "../utils/qzone";
import { runAutoSync } from "../composables/useAutoSync";
import { getRemoteSyncConfig, registerRemoteDevice, saveRemoteSyncEndpoint, type RemoteSyncConfig } from "../utils/remoteSync";

const authStore = useAuthStore();
const { loggedIn, user } = storeToRefs(authStore);
const intervalMs = ref(getArchiveInterval());
const privacyVisible = ref(false);
const deleteVisible = ref(false);
const deleting = ref(false);
const error = ref("");
const appVersion = ref("");
const evidenceBusy = ref<"export" | "import" | null>(null);
const evidencePackages = ref<RecoveryEvidencePackageSummary[]>([]);
const evidenceNotice = ref("");
const evidenceTargetUin = ref("");
const evidenceCandidates = ref<RecoveryEvidenceCandidate[]>([]);
const candidateTotal = ref(0);
const candidatesLoading = ref(false);
const candidateBusy = ref<number | null>(null);
const candidateConfirmVisible = ref(false);
const selectedCandidate = ref<RecoveryEvidenceCandidate | null>(null);
const remoteConfig = ref<RemoteSyncConfig | null>(null);
const remoteEndpoint = ref("");
const remoteDeviceLabel = ref("");
const remoteBusy = ref<"save" | "register" | "refresh" | "sync" | null>(null);
const remoteNotice = ref("");
const remoteSyncState = ref<RemoteSyncState | null>(null);

const evidenceFilter = [{ name: "QQ 空间双端证据包", extensions: ["qzone-evidence", "json"] }];

function formatEvidenceTime(timestamp?: number | null) {
  if (!timestamp) return "时间未知";
  const date = new Date(timestamp * 1000);
  return Number.isNaN(date.getTime()) ? "时间未知" : date.toLocaleString();
}

async function refreshEvidencePackages() {
  try {
    evidencePackages.value = await listRecoveryEvidencePackages();
  } catch (reason) {
    // Listing is intentionally best-effort: the buttons remain usable even
    // when an older database has not finished its first migration yet.
    console.warn("读取双端证据包列表失败", reason);
  }
}

async function refreshEvidenceCandidates() {
  if (!loggedIn.value) {
    evidenceCandidates.value = [];
    candidateTotal.value = 0;
    return;
  }
  candidatesLoading.value = true;
  try {
    const result = await listRecoveryEvidenceCandidates(20, 0);
    evidenceCandidates.value = result.items;
    candidateTotal.value = result.total;
  } catch (reason) {
    // A missing session or an older backend should not make the settings page
    // unusable. The user can retry after logging in or upgrading the database.
    console.warn("读取双端恢复候选失败", reason);
  } finally {
    candidatesLoading.value = false;
  }
}

async function refreshRemoteSync() {
  remoteBusy.value = "refresh";
  try {
    remoteConfig.value = await getRemoteSyncConfig();
    remoteEndpoint.value = remoteConfig.value.endpoint || remoteEndpoint.value;
    remoteDeviceLabel.value = remoteConfig.value.label || remoteDeviceLabel.value;
    if (remoteConfig.value.registered) {
      remoteSyncState.value = await getRemoteSyncState();
    } else {
      remoteSyncState.value = null;
    }
  } catch (reason) {
    // The settings page should remain usable if the system credential store
    // is unavailable or the server has not been configured yet.
    console.warn("读取远程同步状态失败", reason);
  } finally {
    remoteBusy.value = null;
  }
}

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (reason) {
    console.warn("读取应用版本失败", reason);
  }
  await refreshEvidencePackages();
  await refreshEvidenceCandidates();
  await refreshRemoteSync();
});

watch(intervalMs, (value) => { intervalMs.value = setArchiveInterval(value); });

async function deleteEverything() {
  deleting.value = true; error.value = ""; evidenceNotice.value = "";
  try {
    await deleteAllAppData();
    resetAppSettings(); intervalMs.value = DEFAULT_ARCHIVE_INTERVAL;
    await authStore.logout();
    deleteVisible.value = false;
  } catch (reason) { error.value = String(reason); }
  finally { deleting.value = false; }
}

async function exportEvidencePackage() {
  if (!loggedIn.value || evidenceBusy.value) return;
  error.value = ""; evidenceNotice.value = "";
  let path: string | null;
  try {
    const defaultPath = `QQ空间双端证据-${user.value?.uin ?? "账号"}-${new Date().toISOString().slice(0, 10)}.qzone-evidence.json`;
    path = await save({ defaultPath, filters: evidenceFilter });
    if (!path) return;
    evidenceBusy.value = "export";
    const targetUin = evidenceTargetUin.value.trim();
    if (!targetUin) {
      throw new Error("请先填写经过授权的对方 QQ 号");
    }
    if (targetUin && !/^\d+$/.test(targetUin)) {
      throw new Error("对方 QQ 号只能填写数字");
    }
    const result = await exportRecoveryEvidence(path, targetUin || undefined);
    evidenceNotice.value = `双端证据包已导出：${result.itemCount} 条记录。请将文件交给另一位经过授权的账号导入。`;
  } catch (reason) {
    error.value = `导出双端证据包失败：${String(reason)}`;
  } finally {
    evidenceBusy.value = null;
  }
}

async function importEvidencePackage() {
  if (evidenceBusy.value) return;
  error.value = ""; evidenceNotice.value = "";
  try {
    const selected = await open({ multiple: false, directory: false, filters: evidenceFilter });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;
    evidenceBusy.value = "import";
    const result = await importRecoveryEvidence(path);
    await refreshEvidencePackages();
    await refreshEvidenceCandidates();
    evidenceNotice.value = `双端证据包已导入：${result.itemCount} 条记录。后续匹配时会保留原始记录，不会覆盖本地内容。`;
  } catch (reason) {
    error.value = `导入双端证据包失败：${String(reason)}`;
  } finally {
    evidenceBusy.value = null;
  }
}

function requestCandidateMerge(candidate: RecoveryEvidenceCandidate) {
  if (candidateBusy.value !== null) return;
  selectedCandidate.value = candidate;
  candidateConfirmVisible.value = true;
}

async function confirmCandidateMerge() {
  const candidate = selectedCandidate.value;
  if (!candidate || candidateBusy.value !== null) return;
  candidateBusy.value = candidate.id;
  error.value = "";
  evidenceNotice.value = "";
  try {
    const result = await mergeRecoveryEvidenceItem(candidate.id);
    candidateConfirmVisible.value = false;
    selectedCandidate.value = null;
    await refreshEvidenceCandidates();
    evidenceNotice.value = result.message || "候选记录已合并到本地归档。";
  } catch (reason) {
    error.value = `合并候选记录失败：${String(reason)}`;
  } finally {
    candidateBusy.value = null;
  }
}

async function saveRemoteEndpoint() {
  if (remoteBusy.value) return;
  remoteBusy.value = "save";
  remoteNotice.value = "";
  try {
    remoteConfig.value = await saveRemoteSyncEndpoint(remoteEndpoint.value);
    remoteEndpoint.value = remoteConfig.value.endpoint || remoteEndpoint.value;
    remoteNotice.value = "服务器地址已保存到系统安全凭据库。";
  } catch (reason) {
    error.value = `保存远程服务器地址失败：${String(reason)}`;
  } finally {
    remoteBusy.value = null;
  }
}

async function registerRemote() {
  if (remoteBusy.value) return;
  if (!loggedIn.value || !user.value?.uin) {
    error.value = "请先登录 QQ 空间，登录的 QQ 号会自动作为同步账号";
    return;
  }
  remoteBusy.value = "register";
  remoteNotice.value = "";
  try {
    const result = await registerRemoteDevice(remoteEndpoint.value, user.value.uin, remoteDeviceLabel.value);
    remoteConfig.value = await getRemoteSyncConfig();
    remoteSyncState.value = await getRemoteSyncState();
    remoteNotice.value = `设备注册成功（账号 ${result.accountUin}，密钥版本 ${result.keyVersion}）。`;
  } catch (reason) {
    error.value = `注册远程设备失败：${String(reason)}`;
  } finally {
    remoteBusy.value = null;
  }
}

async function runSyncAll() {
  if (remoteBusy.value) return;
  if (!loggedIn.value || !user.value?.uin) {
    error.value = "请先登录 QQ 空间";
    return;
  }
  remoteBusy.value = "sync";
  remoteNotice.value = "";
  error.value = "";
  try {
    const summary = await runAutoSync();
    await refreshEvidenceCandidates();
    remoteSyncState.value = await getRemoteSyncState();
    remoteNotice.value = `同步完成：上传 ${summary.uploaded} 条${summary.skippedTargets ? `，跳过 ${summary.skippedTargets} 个未注册的对方账号` : ""}；拉取处理 ${summary.pulled} 条。`;
  } catch (reason) {
    error.value = `同步失败：${String(reason)}`;
  } finally {
    remoteBusy.value = null;
  }
}

function candidateLabel(candidate: RecoveryEvidenceCandidate) {
  return candidate.originalAuthorName || candidate.actorName || candidate.originalAuthorUin || candidate.actorUin || "未知账号";
}

function candidatePreview(candidate: RecoveryEvidenceCandidate) {
  const content = candidate.content || candidate.eventSummary || candidate.title || "（无文字内容）";
  return content.replace(/\s+/g, " ").trim().slice(0, 100);
}
</script>

<template>
  <section class="settings-stack">
    <article class="surface-card settings-card">
      <div class="settings-copy"><span class="settings-icon tone-blue"><i class="pi pi-user" /></span><div><h3>QQ 空间账号</h3><p>{{ loggedIn ? `${user?.nickname}（QQ ${user?.uin}）` : "尚未登录 QQ 空间" }}</p></div></div>
      <Button v-if="loggedIn" label="退出登录" icon="pi pi-sign-out" severity="danger" outlined @click="authStore.logout" />
      <Button v-else label="登录" icon="pi pi-link" @click="authStore.openLogin" />
    </article>

    <article class="surface-card settings-card interval-setting">
      <div class="settings-copy"><span class="settings-icon tone-green"><i class="pi pi-clock" /></span><div><h3>单页获取间隔</h3><p>每读取一页后等待一段时间再请求下一页，间隔越久越稳定。</p></div></div>
      <div class="interval-control"><InputNumber v-model="intervalMs" :min="MIN_ARCHIVE_INTERVAL" :max="30000" :step="500" suffix=" ms" show-buttons button-layout="horizontal" decrement-button-icon="pi pi-minus" increment-button-icon="pi pi-plus" /><small>最低 2000ms，建议 3000–5000ms</small></div>
    </article>

    <article class="surface-card settings-card evidence-setting">
      <div class="settings-copy"><span class="settings-icon tone-blue"><i class="pi pi-sync" /></span><div><h3>双端协作恢复</h3><p>让两个经过授权的账号交换本地证据，补足“对方有、自己没有”的互动记录。证据包不包含 Cookie 或登录凭证。</p><p class="evidence-summary">填写对方 QQ 号后，只导出与对方动态和互动相关的记录，避免分享无关内容。</p><p class="evidence-summary" v-if="evidencePackages.length">本机已有 {{ evidencePackages.length }} 个证据包，最近导入于 {{ formatEvidenceTime(evidencePackages[0].importedAt) }}。</p><p class="evidence-summary" v-else>尚未导入其他账号的证据包。</p></div></div>
      <div class="evidence-actions">
        <InputText v-model.trim="evidenceTargetUin" class="evidence-target" inputmode="numeric" placeholder="对方 QQ 号（必填）" aria-label="对方 QQ 号" />
        <Button label="导出我的证据" icon="pi pi-upload" :loading="evidenceBusy === 'export'" :disabled="!loggedIn || Boolean(evidenceBusy)" @click="exportEvidencePackage" />
        <Button label="导入对方证据" icon="pi pi-download" severity="secondary" outlined :loading="evidenceBusy === 'import'" :disabled="Boolean(evidenceBusy)" @click="importEvidencePackage" />
      </div>
    </article>
    <p v-if="evidenceNotice" class="evidence-notice"><i class="pi pi-check-circle" />{{ evidenceNotice }}</p>

    <article class="surface-card settings-card candidate-setting">
      <div class="settings-copy"><span class="settings-icon tone-orange"><i class="pi pi-search" /></span><div><h3>待确认的恢复候选</h3><p>导入对方证据后，系统只会列出本地暂时缺失的候选记录。每条记录都必须由你手动确认，应用不会静默覆盖已有内容。</p></div></div>
      <div class="candidate-toolbar">
        <span v-if="loggedIn && candidateTotal" class="candidate-count">{{ candidateTotal }} 条待确认</span>
        <span v-else-if="loggedIn" class="candidate-count">暂无待确认候选</span>
        <span v-else class="candidate-count">登录后查看候选</span>
        <Button label="刷新候选" icon="pi pi-refresh" severity="secondary" text :loading="candidatesLoading" :disabled="!loggedIn || Boolean(evidenceBusy)" @click="refreshEvidenceCandidates" />
      </div>
      <div v-if="evidenceCandidates.length" class="candidate-list">
        <div v-for="candidate in evidenceCandidates" :key="candidate.id" class="candidate-row">
          <div class="candidate-content">
            <div class="candidate-heading"><strong>{{ candidateLabel(candidate) }}</strong><small>{{ formatEvidenceTime(candidate.eventTime) }}</small></div>
            <p>{{ candidatePreview(candidate) }}</p>
            <small v-if="candidate.matchReason" class="candidate-reason">{{ candidate.matchReason }}</small>
          </div>
          <Button label="查看并合并" icon="pi pi-check" size="small" outlined :loading="candidateBusy === candidate.id" :disabled="candidateBusy !== null" @click="requestCandidateMerge(candidate)" />
        </div>
      </div>
      <small v-if="candidateTotal > evidenceCandidates.length" class="candidate-more">仅显示前 {{ evidenceCandidates.length }} 条，请先处理当前候选。</small>
    </article>

    <article class="surface-card settings-card remote-sync-setting">
      <div class="settings-copy"><span class="settings-icon tone-purple"><i class="pi pi-cloud-upload" /></span><div><h3>账号级自动同步</h3><p>程序登录 QQ 空间后自动用当前 QQ 号注册设备，把本账号的互动记录加密同步到相关账号。服务器只保存密文、摘要和游标，不保存 QQ Cookie 或明文内容。</p></div></div>
      <div class="remote-sync-form">
        <InputText v-model.trim="remoteEndpoint" placeholder="服务器地址，例如 http://193.112.172.120:8787" aria-label="远程同步服务器地址" />
        <InputText v-model.trim="remoteDeviceLabel" placeholder="设备名称（可选）" aria-label="设备名称" />
        <div class="remote-sync-actions">
          <Button label="保存地址" icon="pi pi-save" severity="secondary" outlined :loading="remoteBusy === 'save'" :disabled="Boolean(remoteBusy) || !remoteEndpoint" @click="saveRemoteEndpoint" />
          <Button label="注册本设备" icon="pi pi-key" :loading="remoteBusy === 'register'" :disabled="Boolean(remoteBusy) || !remoteEndpoint || !loggedIn" @click="registerRemote" />
          <Button label="立即同步" icon="pi pi-sync" :loading="remoteBusy === 'sync'" :disabled="Boolean(remoteBusy) || !loggedIn" @click="runSyncAll" />
          <Button label="刷新" icon="pi pi-refresh" severity="secondary" text :loading="remoteBusy === 'refresh'" :disabled="Boolean(remoteBusy)" @click="refreshRemoteSync" />
        </div>
        <small v-if="remoteConfig?.registered" class="remote-sync-state"><i class="pi pi-check-circle" /> 已注册：账号 {{ remoteConfig.accountUin }}，密钥版本 {{ remoteConfig.keyVersion }}。对方账号注册后即可互相同步。</small>
        <small v-else class="remote-sync-state"><i class="pi pi-info-circle" /> 保存服务器地址后点击「注册本设备」，登录的 QQ 号会自动成为同步账号。</small>
      </div>
      <div v-if="remoteSyncState" class="remote-sync-stats">
        <span><i class="pi pi-upload" /> 已上传 {{ remoteSyncState.uploadedCount }} 条</span>
        <span><i class="pi pi-clock" /> 待上传 {{ remoteSyncState.pendingCount }} 条</span>
        <span><i class="pi pi-history" /> 上次同步：{{ remoteSyncState.lastSyncAt ? formatEvidenceTime(remoteSyncState.lastSyncAt) : "从未" }}</span>
      </div>
    </article>
    <p v-if="remoteNotice" class="evidence-notice"><i class="pi pi-check-circle" />{{ remoteNotice }}</p>

    <article class="surface-card settings-card">
      <div class="settings-copy"><span class="settings-icon tone-purple"><i class="pi pi-shield" /></span><div><h3>隐私协议</h3><p>了解登录凭证、归档内容和网络请求的处理方式。</p></div></div>
      <Button label="查看协议" icon="pi pi-angle-right" icon-pos="right" severity="secondary" text @click="privacyVisible = true" />
    </article>

    <article class="surface-card settings-card danger-settings-card">
      <div class="settings-copy"><span class="settings-icon tone-red"><i class="pi pi-trash" /></span><div><h3>删除所有数据</h3><p>删除全部账号的归档、续传记录、媒体缓存和本地登录状态。</p></div></div>
      <Button label="删除所有数据" icon="pi pi-trash" severity="danger" outlined @click="deleteVisible = true" />
    </article>

    <p v-if="error" class="archive-error"><i class="pi pi-exclamation-circle" />{{ error }}</p>
    <article class="surface-card settings-card about-card">
      <div class="about-main">
        <div class="settings-copy"><span class="settings-icon"><i class="pi pi-info-circle" /></span><div><h3>关于</h3><p>QQ Zone Restore Archive · 跨平台空间恢复归档工具</p><p class="author-line">作者：<button class="author-link" type="button" @click="openUrl('https://github.com/xiaosu19')">https://github.com/xiaosu19 <i class="pi pi-external-link" /></button></p><p class="author-line">项目：<button class="author-link" type="button" @click="openUrl('https://github.com/xiaosu19/QQ-Zone-Restore-Archive')">QQ-Zone-Restore-Archive <i class="pi pi-external-link" /></button></p><p class="author-line">基于：<button class="author-link" type="button" @click="openUrl('https://github.com/Gaoshu705/QzoneArchive')">Gaoshu705/QzoneArchive <i class="pi pi-external-link" /></button></p><p class="author-line">参考：<button class="author-link" type="button" @click="openUrl('https://github.com/LibraHp/GetQzonehistory')">LibraHp/GetQzonehistory <i class="pi pi-external-link" /></button> · <button class="author-link" type="button" @click="openUrl('https://github.com/ShunCai/QZoneExport')">ShunCai/QZoneExport <i class="pi pi-external-link" /></button></p></div></div>
        <span class="version-badge">{{ appVersion ? `v${appVersion}` : "版本未知" }}</span>
      </div>
    </article>
  </section>

  <Dialog v-model:visible="privacyVisible" modal :draggable="false" class="privacy-dialog" header="隐私协议">
    <div class="privacy-content">
      <p>QQ Zone Restore Archive 是一款本地恢复归档工具。我们重视你的账号与空间内容安全。</p>
      <h4>1. 数据存储</h4><p>QQ 空间动态、留言、点赞、评论、登录会话和媒体缓存保存在你的设备本地，不会上传至本项目的开发者服务器。</p>
      <h4>2. 网络请求</h4><p>应用仅在登录、读取空间资料、归档内容及下载相关媒体时直接请求腾讯 QQ、QQ 空间及其媒体域名。</p>
      <h4>3. 登录凭证</h4><p>扫码登录产生的 Cookie 仅用于访问当前账号有权查看的 QQ 空间内容。退出登录或删除所有数据后，本地会话会被清除。</p>
      <h4>4. 导出与分享</h4><p>导出的 HTML、保存的图片和双端证据包由你自行保管。双端证据包不包含 Cookie 或登录凭证，但可能包含昵称、QQ 号和空间内容，请只交给经过授权的对方。</p>
      <h4>5. 数据删除</h4><p>你可以随时使用“删除所有数据”清理全部本地归档、双端证据、任务续传位置、视频缓存与登录状态，此操作无法撤销。</p>
    </div>
    <template #footer><Button label="我已了解" @click="privacyVisible = false" /></template>
  </Dialog>

  <Dialog v-model:visible="candidateConfirmVisible" modal :draggable="false" class="candidate-dialog" header="确认合并恢复记录？">
    <div v-if="selectedCandidate" class="candidate-confirm-content">
      <p>将把以下来自 <strong>{{ candidateLabel(selectedCandidate) }}</strong> 的候选记录写入当前本地归档：</p>
      <blockquote>{{ candidatePreview(selectedCandidate) }}</blockquote>
      <small>这一步不会上传数据，也不会覆盖已经存在的本地记录。确认后可在归档页面查看。</small>
    </div>
    <template #footer><Button label="取消" severity="secondary" text :disabled="candidateBusy !== null" @click="candidateConfirmVisible = false" /><Button label="确认合并" icon="pi pi-check" :loading="candidateBusy !== null" @click="confirmCandidateMerge" /></template>
  </Dialog>

  <Dialog v-model:visible="deleteVisible" modal :closable="!deleting" :draggable="false" class="delete-dialog" header="删除所有数据？">
    <div class="delete-dialog-content"><span class="delete-warning"><i class="pi pi-exclamation-triangle" /></span><div><p>所有账号的本地归档和媒体缓存都将被永久删除。</p><small>包括动态、留言、评论、点赞、续传记录、视频缓存及登录状态。此操作无法撤销。</small></div></div>
    <template #footer><Button label="取消" severity="secondary" text :disabled="deleting" @click="deleteVisible = false" /><Button label="确认全部删除" icon="pi pi-trash" severity="danger" :loading="deleting" @click="deleteEverything" /></template>
  </Dialog>
</template>

<style scoped>
.evidence-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; }
.evidence-target { width: 190px; }
.evidence-summary { margin-top: 5px !important; font-size: 11px !important; }
.evidence-notice { display: flex; align-items: center; gap: 8px; margin: 0; padding: 11px 14px; color: #16875d; background: #edfff6; border: 1px solid #c7f4dd; border-radius: 11px; font-size: 13px; }
.candidate-setting { display: block; }
.remote-sync-setting { display: block; }
.remote-sync-form { display: grid; gap: 10px; width: min(100%, 620px); margin-top: 14px; margin-left: 55px; }
.remote-sync-actions { display: flex; flex-wrap: wrap; align-items: center; gap: 9px; }
.remote-sync-actions > .p-inputtext { min-width: 190px; flex: 1 1 210px; }
.remote-sync-state { color: var(--muted); font-size: 11px; }
.remote-sync-state .pi { margin-right: 4px; color: #169766; }
.remote-sync-hint { flex: 1 1 100%; color: var(--muted); font-size: 11px; }
.remote-sync-stats { display: flex; flex-wrap: wrap; gap: 6px 16px; margin-top: 12px; margin-left: 55px; }
.remote-sync-stats span { display: inline-flex; align-items: center; gap: 5px; color: var(--text-color-secondary, #64748b); font-size: 12px; }
.remote-sync-stats .pi { color: #169766; }
.remote-pairing-panel { display: grid; gap: 10px; margin-top: 13px; margin-left: 55px; }
.remote-code-input { max-width: 220px; letter-spacing: .08em; text-transform: uppercase; }
.remote-invitation { display: flex; flex-wrap: wrap; align-items: baseline; gap: 10px; padding: 11px 13px; color: var(--heading); background: var(--app-bg); border-radius: 10px; }
.remote-invitation strong { letter-spacing: .16em; }
.remote-invitation small, .remote-pairing-row small { color: var(--muted); }
.remote-pairing-list { display: grid; gap: 6px; }
.remote-pairing-row { display: flex; justify-content: space-between; padding: 8px 10px; color: var(--heading); background: var(--app-bg); border-radius: 8px; font-size: 12px; }
.candidate-toolbar { display: flex; align-items: center; justify-content: flex-end; gap: 10px; margin-top: 12px; }
.candidate-count { margin-right: auto; color: var(--text-color-secondary, #64748b); font-size: 12px; }
.candidate-list { display: grid; gap: 8px; margin-top: 4px; }
.candidate-row { display: flex; align-items: center; gap: 12px; padding: 11px 12px; border: 1px solid var(--surface-border, #e2e8f0); border-radius: 10px; background: var(--surface-ground, #f8fafc); }
.candidate-content { min-width: 0; flex: 1; }
.candidate-heading { display: flex; align-items: center; gap: 8px; }
.candidate-heading strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.candidate-heading small, .candidate-reason, .candidate-more { color: var(--text-color-secondary, #64748b); font-size: 11px; }
.candidate-content p { overflow: hidden; margin: 4px 0 0; color: var(--text-color, #334155); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
.candidate-reason { display: block; margin-top: 3px; }
.candidate-more { display: block; margin-top: 9px; }
.candidate-confirm-content blockquote { margin: 10px 0; padding: 10px 12px; border-left: 3px solid #f59e0b; background: #fffaf0; color: var(--text-color, #334155); }
.candidate-confirm-content small { color: var(--text-color-secondary, #64748b); }
@media (max-width: 760px) {
  .evidence-actions { width: 100%; }
  .evidence-target { width: 100%; }
  .evidence-actions .p-button { flex: 1 1 170px; }
  .candidate-row { align-items: flex-start; flex-direction: column; }
  .candidate-row .p-button { width: 100%; }
  .remote-sync-form, .remote-pairing-panel { width: 100%; margin-left: 0; }
  .remote-sync-actions > .p-inputtext { min-width: 0; flex-basis: 100%; }
  .remote-sync-actions .p-button { flex: 1 1 160px; }
}
</style>
