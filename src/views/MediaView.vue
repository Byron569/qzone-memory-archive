<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import Button from "primevue/button";
import Dialog from "primevue/dialog";
import Select from "primevue/select";
import QzoneText from "../components/QzoneText.vue";
import { loadRemoteImageBlob } from "../utils/archiveImage";
import { getArchivedFeed, listArchivedMedia, loadArchivedImage, loadArchivedVideo, type ArchiveItem, type ArchiveMediaItem } from "../utils/qzone";

const PAGE_SIZE = 60;
const media = ref<ArchiveMediaItem[]>([]);
const years = ref<number[]>([]);
const selectedYear = ref(0);
const total = ref(0);
const loading = ref(false);
const error = ref("");
const detailVisible = ref(false);
const detailLoading = ref(false);
const detail = ref<ArchiveItem>();
const imageSources = reactive<Record<string, string>>({});
const imageLoading = reactive<Record<string, boolean>>({});
const imageErrors = reactive<Record<string, string>>({});
const imageFallbackAttempted = reactive<Record<string, boolean>>({});
const videoSource = ref("");
const videoLoading = ref(false);
const videoError = ref("");
let imageObserver: IntersectionObserver | undefined;
let loadObserver: IntersectionObserver | undefined;

const hasMore = computed(() => media.value.length < total.value);
const yearOptions = computed(() => [{ label: "全部年份", value: 0 }, ...years.value.map((year) => ({ label: `${year} 年`, value: year }))]);
const formatTime = (seconds: number) => seconds ? new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(seconds * 1000)) : "时间未知";
const avatarUrl = (uin?: string) => uin ? `https://qlogo2.store.qq.com/qzone/${uin}/${uin}/50?${Date.now()}` : "";

async function load(reset = false) {
  if (loading.value || (!reset && !hasMore.value)) return;
  loading.value = true;
  error.value = "";
  try {
    const page = await listArchivedMedia(PAGE_SIZE, reset ? 0 : media.value.length, selectedYear.value || undefined);
    years.value = page.years;
    total.value = page.total;
    media.value = reset ? page.items : [...media.value, ...page.items];
    await nextTick();
    observeImages();
  } catch (reason) { error.value = `读取媒体失败：${String(reason)}`; }
  finally { loading.value = false; }
}

function observeImages() {
  imageObserver?.disconnect();
  imageObserver = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const element = entry.target as HTMLElement;
      const url = element.dataset.mediaImage;
      const dynamicId = Number(element.dataset.dynamicId);
      const pictureIndex = Number(element.dataset.pictureIndex);
      if (url) void loadImage(url, Number.isFinite(dynamicId) ? dynamicId : undefined, Number.isFinite(pictureIndex) ? pictureIndex : undefined);
      imageObserver?.unobserve(element);
    }
  }, { rootMargin: "360px 0px" });
  document.querySelectorAll<HTMLElement>(".media-page [data-media-image]").forEach((element) => imageObserver?.observe(element));
}

async function loadImage(url: string, dynamicId?: number, pictureIndex?: number) {
  if (!url || imageSources[url] || imageLoading[url]) return;
  imageLoading[url] = true;
  delete imageErrors[url];
  delete imageFallbackAttempted[url];
  try {
    if (dynamicId !== undefined && pictureIndex !== undefined) {
      try {
        imageSources[url] = convertFileSrc(await loadArchivedImage(dynamicId, pictureIndex));
        return;
      } catch (reason) {
        console.warn("本地媒体图片加载失败，改用原始地址", reason);
      }
    }
    await loadRemoteImage(url);
  } catch (reason) { imageErrors[url] = String(reason); console.error("媒体图片加载失败", reason); }
  finally { imageLoading[url] = false; }
}
async function loadRemoteImage(url: string) {
  imageSources[url] = await loadRemoteImageBlob(url);
}
async function handleImageError(url: string) {
  if (!url || imageLoading[url]) return;
  const source = imageSources[url];
  if (source?.startsWith("blob:")) URL.revokeObjectURL(source);
  delete imageSources[url];
  if (imageFallbackAttempted[url]) {
    imageErrors[url] = "图片文件无法显示，可点击重试";
    return;
  }
  imageFallbackAttempted[url] = true;
  imageLoading[url] = true;
  try { await loadRemoteImage(url); }
  catch (reason) { imageErrors[url] = String(reason); }
  finally { imageLoading[url] = false; }
}

async function openOriginal(item: ArchiveMediaItem) {
  detailVisible.value = true;
  detailLoading.value = true;
  detail.value = undefined;
  releaseVideo();
  try {
    detail.value = await getArchivedFeed(item.dynamicId);
    await Promise.all(detail.value.pictureUrls.map((url, index) => loadImage(url, detail.value?.id, index)));
    if (detail.value.videoCoverUrl) await loadImage(detail.value.videoCoverUrl);
  } catch (reason) { error.value = `读取原始动态失败：${String(reason)}`; detailVisible.value = false; }
  finally { detailLoading.value = false; }
}

async function playVideo() {
  if (!detail.value?.videoUrl || videoLoading.value) return;
  videoLoading.value = true;
  videoError.value = "";
  try { videoSource.value = convertFileSrc(await loadArchivedVideo(detail.value.id)); }
  catch (reason) { videoError.value = `视频加载失败：${String(reason)}`; }
  finally { videoLoading.value = false; }
}

function releaseVideo() {
  if (videoSource.value.startsWith("blob:")) URL.revokeObjectURL(videoSource.value);
  videoSource.value = "";
  videoError.value = "";
}

watch(selectedYear, () => void load(true));
watch(detailVisible, (visible) => { if (!visible) releaseVideo(); });
onMounted(() => {
  void load(true);
  loadObserver = new IntersectionObserver((entries) => { if (entries.some((entry) => entry.isIntersecting)) void load(); }, { rootMargin: "500px 0px" });
  const sentinel = document.querySelector(".media-load-sentinel");
  if (sentinel) loadObserver.observe(sentinel);
});
onBeforeUnmount(() => {
  imageObserver?.disconnect(); loadObserver?.disconnect(); releaseVideo();
  Object.values(imageSources).forEach((url) => { if (url.startsWith("blob:")) URL.revokeObjectURL(url); });
});
</script>

<template>
  <div class="media-page">
    <section class="media-hero surface-card">
      <div class="media-hero-copy"><span class="media-hero-icon"><i class="pi pi-images" /></span><div><h2>说说媒体</h2><p>集中浏览说说归档中的照片与视频；QQ 相册内容请在“相册”中查看</p></div></div>
      <div class="media-filter"><label for="media-year">拍摄年份</label><Select id="media-year" v-model="selectedYear" :options="yearOptions" option-label="label" option-value="value" /><span>共 {{ total }} 项</span></div>
    </section>

    <p v-if="error" class="archive-error"><i class="pi pi-exclamation-circle" />{{ error }}</p>
    <section v-if="media.length" class="media-waterfall" aria-label="归档媒体">
      <button v-for="item in media" :key="item.key" type="button" class="media-tile" :aria-label="`查看 ${item.authorName || 'QQ 用户'} 于 ${formatTime(item.publishedAt)} 发布的${item.mediaType === 'video' ? '视频' : '照片'}`" @click="openOriginal(item)">
        <div class="media-tile-visual" :data-media-image="item.mediaType === 'photo' ? item.url : item.coverUrl" :data-dynamic-id="item.mediaType === 'photo' ? item.dynamicId : undefined" :data-picture-index="item.mediaType === 'photo' ? item.pictureIndex : undefined">
          <img v-if="imageSources[item.mediaType === 'photo' ? item.url : (item.coverUrl || '')]" :src="imageSources[item.mediaType === 'photo' ? item.url : (item.coverUrl || '')]" :alt="`${item.authorName || 'QQ 用户'}的归档${item.mediaType === 'video' ? '视频封面' : '照片'}`" width="640" height="480" loading="lazy" decoding="async" @error="handleImageError(item.mediaType === 'photo' ? item.url : (item.coverUrl || ''))" />
          <span v-else class="media-placeholder"><i :class="item.mediaType === 'video' ? 'pi pi-video' : 'pi pi-image'" /><span v-if="item.mediaType === 'photo' && imageErrors[item.url]" class="media-image-retry" :title="imageErrors[item.url]" role="button" tabindex="0" @click.stop="loadImage(item.url, item.dynamicId, item.pictureIndex)" @keydown.enter.stop="loadImage(item.url, item.dynamicId, item.pictureIndex)">加载失败，重试</span><template v-else>{{ imageLoading[item.mediaType === 'photo' ? item.url : (item.coverUrl || '')] ? '正在尝试多个图片地址' : item.mediaType === 'video' ? '视频' : '照片' }}</template></span>
          <span v-if="item.mediaType === 'video'" class="media-video-mark"><i class="pi pi-play" /></span>
          <time>{{ new Date(item.publishedAt * 1000).getFullYear() }}</time>
        </div>
        <span class="media-tile-meta"><strong>{{ item.authorName || "QQ 用户" }}</strong><small>{{ formatTime(item.publishedAt) }}</small><span v-if="item.content">{{ item.content }}</span></span>
      </button>
    </section>
    <div v-else-if="!loading" class="media-empty surface-card"><i class="pi pi-images" /><h3>暂无媒体内容</h3><p>{{ selectedYear ? `${selectedYear} 年没有归档照片或视频` : "完成动态归档后，照片和视频会显示在这里" }}</p></div>
    <div class="media-load-sentinel"><i v-if="loading" class="pi pi-spin pi-spinner" /><Button v-else-if="hasMore" label="加载更多" severity="secondary" text @click="load()" /><span v-else-if="media.length">已经到底了</span></div>
  </div>

  <Dialog v-model:visible="detailVisible" modal :draggable="false" class="media-detail-dialog" header="原始动态">
    <div v-if="detailLoading" class="media-detail-loading"><i class="pi pi-spin pi-spinner" /><span>正在读取原始动态…</span></div>
    <article v-else-if="detail" class="media-original">
      <header><span class="archive-avatar"><img v-if="detail.authorUin" :src="avatarUrl(detail.authorUin)" referrerpolicy="no-referrer" /><i v-else class="pi pi-user" /></span><div><strong>{{ detail.authorName || "QQ 用户" }}</strong><small><span v-if="detail.authorUin">QQ {{ detail.authorUin }} · </span>{{ formatTime(detail.publishedAt) }}</small></div></header>
      <p v-if="detail.content" class="media-original-content"><QzoneText :value="detail.content" /></p>
      <div v-if="detail.pictureUrls.length" class="media-original-pictures"><template v-for="(url, index) in detail.pictureUrls" :key="url"><img v-if="imageSources[url]" :src="imageSources[url]" @error="handleImageError(url)" /><button v-else type="button" class="media-detail-image-placeholder" :title="imageErrors[url]" @click="loadImage(url, detail.id, index)"><i class="pi pi-image" />{{ imageErrors[url] ? "加载失败，重试" : "图片加载中" }}</button></template></div>
      <video v-if="videoSource" class="archive-video" :src="videoSource" controls autoplay playsinline />
      <button v-else-if="detail.videoUrl" type="button" class="archive-video-cover media-detail-video" @click="playVideo"><img v-if="detail.videoCoverUrl && imageSources[detail.videoCoverUrl]" :src="imageSources[detail.videoCoverUrl]" /><span class="video-cover-shade"><span class="video-play-button"><i :class="videoLoading ? 'pi pi-spin pi-spinner' : 'pi pi-play'" /></span><strong>{{ videoLoading ? "正在加载视频…" : "点击播放视频" }}</strong><small>{{ videoError || "视频将在点击后下载" }}</small></span></button>
      <div class="archive-assets"><span><i class="pi pi-heart" />{{ detail.likeCount }} 个赞</span><span><i class="pi pi-comment" />{{ detail.commentCount }} 条评论</span></div>
      <section v-if="detail.comments.length" class="archive-comments"><div v-for="comment in detail.comments" :key="`${comment.uin}-${comment.createdAt}-${comment.content}`" class="archive-comment"><span class="archive-comment-avatar"><img v-if="comment.uin" :src="avatarUrl(comment.uin)" referrerpolicy="no-referrer" /><i v-else class="pi pi-user" /></span><div class="archive-comment-body"><p><strong>{{ comment.nickname || "QQ 用户" }}</strong><QzoneText :value="comment.content" /></p><time>{{ formatTime(comment.createdAt) }}</time><div v-if="comment.replies.length" class="archive-comment-replies"><div v-for="reply in comment.replies" :key="`${reply.uin}-${reply.createdAt}-${reply.content}`" class="archive-reply"><p><strong>{{ reply.nickname || "QQ 用户" }}</strong><QzoneText :value="reply.content" /></p></div></div></div></div></section>
    </article>
  </Dialog>
</template>
