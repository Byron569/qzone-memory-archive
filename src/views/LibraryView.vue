<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import Button from "primevue/button";
import InputText from "primevue/inputtext";
import Paginator, { type PageState } from "primevue/paginator";
import Select from "primevue/select";
import { loadRemoteImageBlob } from "../utils/archiveImage";
import {
  listQzoneLibrary,
  listQzoneLibraryYears,
  syncQzoneLibrary,
  type LibraryItem,
  type LibraryModule,
  type LibraryPage,
} from "../utils/qzone";

const route = useRoute();
const router = useRouter();
const items = ref<LibraryItem[]>([]);
const page = ref<LibraryPage>();
const years = ref<number[]>([]);
const selectedYear = ref(0);
const query = ref("");
const first = ref(0);
const pageSize = ref(24);
const loading = ref(false);
const syncing = ref(false);
const error = ref("");
const notice = ref("");
const imageSources = reactive<Record<number, string>>({});
const failedGuestbookAvatars = reactive(new Set<number>());
let searchTimer: ReturnType<typeof setTimeout> | undefined;
let loadSequence = 0;

const module = computed(() => String(route.meta.module || "albums") as LibraryModule);
const parentKey = computed(() => typeof route.params.albumId === "string" ? route.params.albumId : undefined);
const albumTitle = computed(() => typeof route.query.title === "string" ? route.query.title : "相册照片");
const moduleInfo = computed(() => ({
  albums: { title: "相册", kicker: "ALBUMS", icon: "pi pi-images", description: "保存相册目录，进入相册后继续同步其中的照片。", action: "同步相册目录" },
  photos: { title: albumTitle.value, kicker: "ALBUM PHOTOS", icon: "pi pi-image", description: "逐页保存该相册当前可访问的照片与视频地址。", action: "同步此相册" },
  videos: { title: "视频", kicker: "VIDEOS", icon: "pi pi-video", description: "归档 QQ 空间独立视频列表及可用媒体地址。", action: "同步视频" },
  guestbook: { title: "留言板", kicker: "GUESTBOOK", icon: "pi pi-envelope", description: "合并当前留言接口与历史互动残留，尽量找回已删除留言。", action: "同步留言" },
  favorites: {
    title: "QQ 空间收藏",
    kicker: "QZONE FAVORITES",
    icon: "pi pi-bookmark",
    description: "同步 QQ 空间网页端旧收藏；不等同于手机 QQ 通用收藏，已删除项目仅保留此前保存的本地快照。",
    action: "同步空间收藏",
  },
}[module.value]));
const yearOptions = computed(() => [{ label: "全部年份", value: 0 }, ...years.value.map((year) => ({ label: `${year} 年`, value: year }))]);
const statusText = computed(() => {
  if (!page.value?.syncedAt) return "尚未同步";
  const local = `${page.value.total} 条已保存`;
  const remote = page.value.remoteTotal ? ` / 接口报告 ${page.value.remoteTotal} 条` : "";
  return `${local}${remote} · ${page.value.complete ? "本轮完整" : "待继续核验"}`;
});
const formatTime = (seconds: number) => seconds
  ? new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(seconds * 1000))
  : "时间未保留";
const syncedTime = computed(() => page.value?.syncedAt ? formatTime(page.value.syncedAt) : "—");
const guestbookAvatarUrl = (uin?: string) => uin ? `https://qlogo2.store.qq.com/qzone/${uin}/${uin}/100` : "";
const guestbookInitial = (item: LibraryItem) => (item.authorName || item.title || "留").slice(0, 1);

function releaseImages() {
  Object.values(imageSources).forEach((url) => { if (url.startsWith("blob:")) URL.revokeObjectURL(url); });
  Object.keys(imageSources).forEach((key) => delete imageSources[Number(key)]);
}
async function loadCover(item: LibraryItem) {
  if (!item.coverUrl || imageSources[item.id]) return;
  try { imageSources[item.id] = await loadRemoteImageBlob(item.coverUrl); } catch { /* card keeps its media placeholder */ }
}
async function load() {
  const sequence = ++loadSequence;
  loading.value = true;
  error.value = "";
  releaseImages();
  try {
    const [result, availableYears] = await Promise.all([
      listQzoneLibrary(module.value, parentKey.value, query.value.trim() || undefined, selectedYear.value || undefined, pageSize.value, first.value),
      listQzoneLibraryYears(module.value, parentKey.value),
    ]);
    if (sequence !== loadSequence) return;
    page.value = result;
    items.value = result.items;
    years.value = availableYears;
    failedGuestbookAvatars.clear();
    void Promise.allSettled(result.items.map(loadCover));
  } catch (reason) {
    if (sequence === loadSequence) error.value = String(reason);
  } finally {
    if (sequence === loadSequence) loading.value = false;
  }
}
async function sync() {
  if (syncing.value) return;
  syncing.value = true;
  error.value = "";
  notice.value = "";
  try {
    const result = await syncQzoneLibrary(module.value, parentKey.value);
    notice.value = result.message;
    first.value = 0;
    await load();
  } catch (reason) { error.value = String(reason); await load(); }
  finally { syncing.value = false; }
}
function openItem(item: LibraryItem) {
  if (module.value === "albums") {
    void router.push({ name: "album-photos", params: { albumId: item.itemKey }, query: { title: item.title } });
  }
}
async function openMedia(item: LibraryItem) {
  const url = item.mediaUrls.find((value) => /\.(?:mp4|m3u8)(?:[?#]|$)/i.test(value)) || item.mediaUrls[0] || item.coverUrl;
  if (!url) return;
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
function changePage(event: PageState) { first.value = event.first; pageSize.value = event.rows; void load(); }

onMounted(load);
watch(() => [route.name, route.params.albumId], () => { first.value = 0; selectedYear.value = 0; query.value = ""; void load(); });
watch(selectedYear, () => { first.value = 0; void load(); });
watch(query, () => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => { first.value = 0; void load(); }, 260);
});
onBeforeUnmount(() => { if (searchTimer) clearTimeout(searchTimer); releaseImages(); });
</script>

<template>
  <div class="library-page">
    <section class="library-command surface-card">
      <div class="library-heading">
        <button v-if="module === 'photos'" type="button" class="library-back" aria-label="返回相册列表" @click="router.push('/albums')"><i class="pi pi-arrow-left" /></button>
        <span class="library-heading-icon"><i :class="moduleInfo.icon" /></span>
        <div><p class="section-kicker">{{ moduleInfo.kicker }}</p><h2>{{ moduleInfo.title }}</h2><p>{{ moduleInfo.description }}</p></div>
      </div>
      <div class="library-command-actions">
        <span class="library-sync-state"><strong>{{ statusText }}</strong><small>上次同步：{{ syncedTime }}</small></span>
        <Button :label="moduleInfo.action" icon="pi pi-cloud-download" :loading="syncing" @click="sync" />
      </div>
    </section>

    <section class="library-toolbar surface-card" aria-label="筛选资料归档">
      <label class="search-box"><span class="sr-only">搜索</span><i class="pi pi-search" /><InputText v-model="query" placeholder="搜索标题、正文、昵称或 QQ 号" /></label>
      <Select v-model="selectedYear" :options="yearOptions" option-label="label" option-value="value" aria-label="按年份筛选" />
      <Button icon="pi pi-refresh" label="刷新本地" severity="secondary" text :loading="loading" @click="load" />
    </section>

    <p v-if="notice" class="library-notice"><i class="pi pi-check-circle" />{{ notice }}</p>
    <p v-if="error" class="archive-error"><i class="pi pi-exclamation-circle" />{{ error }}</p>
    <p v-if="page?.lastError && !error" class="library-warning"><i class="pi pi-info-circle" />上次同步未完整结束：{{ page.lastError }}</p>

    <section v-if="items.length" class="library-grid" :class="`module-${module}`">
      <article v-for="item in items" :key="item.id" class="library-card surface-card" :class="{ 'is-clickable': module === 'albums' }" @click="openItem(item)">
        <button v-if="module === 'albums'" type="button" class="library-card-hit" :aria-label="`打开相册：${item.title}`" @click.stop="openItem(item)" />
        <div v-if="module !== 'guestbook' && (imageSources[item.id] || item.mediaUrls.length || module === 'albums' || module === 'photos' || module === 'videos')" class="library-cover" :class="{ 'has-image': imageSources[item.id] }">
          <img v-if="imageSources[item.id]" :src="imageSources[item.id]" :alt="`${item.title} 封面`" loading="lazy" width="560" height="360" />
          <i v-else :class="moduleInfo.icon" />
          <span v-if="item.mediaUrls.length" class="library-media-count">{{ item.mediaUrls.length }} 个媒体地址</span>
        </div>
        <div class="library-card-body" :class="{ 'guestbook-card-body': module === 'guestbook' }">
          <template v-if="module === 'guestbook'">
            <div class="guestbook-author">
              <span class="guestbook-avatar" aria-hidden="true">
                <img v-if="item.authorUin && !failedGuestbookAvatars.has(item.id)" :src="guestbookAvatarUrl(item.authorUin)" loading="lazy" referrerpolicy="no-referrer" @error="failedGuestbookAvatars.add(item.id)" />
                <span v-else>{{ guestbookInitial(item) }}</span>
              </span>
              <span><strong :title="item.authorUin ? `QQ ${item.authorUin}` : undefined">{{ item.authorName || item.title || 'QQ 用户' }}</strong><small>{{ item.authorUin ? `QQ ${item.authorUin} · ` : '' }}{{ formatTime(item.createdAt) }}</small></span>
            </div>
            <p class="guestbook-message">{{ item.summary || '（留言正文未保留）' }}</p>
          </template>
          <template v-else>
            <div class="library-card-meta"><span v-if="item.authorName || item.authorUin" :title="item.authorUin ? `QQ ${item.authorUin}` : undefined">{{ item.authorName || item.authorUin }}</span><time>{{ formatTime(item.createdAt) }}</time></div>
            <h3>{{ item.title }}</h3>
            <p v-if="item.summary">{{ item.summary }}</p>
          </template>
          <div class="library-card-footer">
            <span>{{ module === 'albums' ? '进入相册查看与同步' : module === 'guestbook' && item.itemKey.startsWith('history:') ? '历史互动残留（可能已从留言板删除）' : page?.complete ? '已保存原始记录' : '本轮仍待核验' }}</span>
            <Button v-if="module !== 'albums' && item.mediaUrls.length" icon="pi pi-external-link" label="打开媒体" severity="secondary" text size="small" @click.stop="openMedia(item)" />
            <i v-else-if="module === 'albums'" class="pi pi-arrow-right" />
          </div>
        </div>
      </article>
    </section>
    <section v-else class="library-empty surface-card">
      <span><i :class="moduleInfo.icon" /></span><h2>{{ loading ? "正在读取本地归档" : "还没有保存内容" }}</h2>
      <p>{{ loading ? "请稍候…" : `点击“${moduleInfo.action}”从 QQ 空间分页读取；已有归档不会因接口暂时失败而被清空。` }}</p>
    </section>
    <Paginator v-if="(page?.total || 0) > pageSize" class="archive-paginator" :first="first" :rows="pageSize" :total-records="page?.total || 0" :rows-per-page-options="[12,24,48,96]" @page="changePage" />
  </div>
</template>
