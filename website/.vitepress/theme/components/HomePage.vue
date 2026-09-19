<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { withBase } from "vitepress";

type Platform = "windows" | "macos" | "android" | "linux";
type ThemeMode = "system" | "light" | "dark";

const githubUrl = "https://github.com/xiaosu19/QQ-Zone-Restore-Archive";
const releaseUrl = `${githubUrl}/releases/latest`;
const scrolled = ref(false);
const selectedPlatform = ref<Platform>("windows");
const themeMode = ref<ThemeMode>("system");
const revealObserver = ref<IntersectionObserver | null>(null);

const platforms = [
  { id: "windows", label: "Windows", format: ".exe", arch: "x64", version: "Windows 10+", note: "推荐版本，安装后即可开始归档。" },
  { id: "macos", label: "macOS", format: ".dmg", arch: "Apple / Intel", version: "macOS 11+", note: "请按芯片架构选择对应安装包。" },
  { id: "android", label: "Android", format: ".apk", arch: "arm64", version: "Android 8+", note: "适合在移动设备上查看与管理归档。" },
  { id: "linux", label: "Linux", format: ".AppImage", arch: "x86_64", version: "主流发行版", note: "无需安装，赋予执行权限后直接运行。" },
] as const;

const themeModes: { id: ThemeMode; label: string }[] = [
  { id: "system", label: "跟随系统" },
  { id: "light", label: "浅色" },
  { id: "dark", label: "深色" },
];

const techStack = ["Tauri", "Vue 3", "Rust", "SQLite", "Vite", "TypeScript", "PrimeVue", "reqwest", "HTML Export", "Pinia"];
const marqueeItems = computed(() => [...techStack, ...techStack]);

const activePlatform = computed(() => platforms.find((item) => item.id === selectedPlatform.value) ?? platforms[0]);

function updateScrollState() {
  scrolled.value = window.scrollY > 8;
}

function setDocumentTheme(mode: ThemeMode) {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.toggle("dark", mode === "dark" || (mode === "system" && prefersDark));
}

function onSystemThemeChange() {
  if (themeMode.value === "system") setDocumentTheme("system");
}

function setupRevealObservers() {
  revealObserver.value = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        entry.target.classList.add("is-visible");
        revealObserver.value?.unobserve(entry.target);
      }
    });
  }, { threshold: 0.12, rootMargin: "0px 0px -8% 0px" });

  document.querySelectorAll(".js-reveal").forEach((el) => revealObserver.value?.observe(el));
}

watch(themeMode, (mode) => {
  localStorage.setItem("qzonearchive-theme", mode);
  setDocumentTheme(mode);
});

onMounted(() => {
  const storedTheme = localStorage.getItem("qzonearchive-theme");
  if (storedTheme === "light" || storedTheme === "dark" || storedTheme === "system") {
    themeMode.value = storedTheme;
  }

  setDocumentTheme(themeMode.value);
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", onSystemThemeChange);

  const agent = navigator.userAgent.toLowerCase();
  if (agent.includes("android")) selectedPlatform.value = "android";
  else if (agent.includes("mac")) selectedPlatform.value = "macos";
  else if (agent.includes("linux")) selectedPlatform.value = "linux";

  updateScrollState();
  window.addEventListener("scroll", updateScrollState, { passive: true });
  setupRevealObservers();
});

onBeforeUnmount(() => {
  window.removeEventListener("scroll", updateScrollState);
  window.matchMedia("(prefers-color-scheme: dark)").removeEventListener("change", onSystemThemeChange);
  revealObserver.value?.disconnect();
});
</script>

<template>
  <div class="tech-home">
    <header class="tech-nav" :class="{ 'is-scrolled': scrolled }">
      <div class="tech-nav__inner">
        <a class="tech-brand" :href="withBase('/')" aria-label="QQ Zone Restore Archive 首页">
          <img class="tech-brand__mark" :src="withBase('/qzone-archive-icon.png')" alt="" />
          <span>QQ Zone Restore Archive</span>
        </a>
        <nav class="tech-nav__links" aria-label="首页导航">
          <a href="#features">能力</a>
          <a href="#how">流程</a>
          <a href="#stack">技术栈</a>
          <a href="#data">数据</a>
          <a :href="withBase('/intro/')">文档</a>
        </nav>
        <div class="tech-theme" role="group" aria-label="颜色模式切换">
          <button v-for="mode in themeModes" :key="mode.id" type="button" :class="{ active: themeMode === mode.id }" :title="mode.label" :aria-label="mode.label" @click="themeMode = mode.id">
            <svg v-if="mode.id === 'system'" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4" /></svg>
            <svg v-else-if="mode.id === 'light'" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8Z" /></svg>
          </button>
        </div>
        <a class="tech-button tech-button--small" href="#download">下载</a>
      </div>
    </header>

    <main>
      <section class="tech-hero">
        <div class="tech-hero__scene" aria-hidden="true"><span class="tech-hero__beam tech-hero__beam--one"></span><span class="tech-hero__beam tech-hero__beam--two"></span></div>
        <div class="tech-shell tech-hero__content js-reveal">
          <p class="tech-eyebrow">QQ ZONE RESTORE ARCHIVE / LOCAL-FIRST</p>
          <h1>QQ 空间恢复归档</h1>
          <p class="tech-hero__lead">把 QQ 空间动态、照片和互动，转化为留在你设备上的本地档案。没有云端副本，没有账号系统。</p>
          <div class="tech-actions">
            <a class="tech-button" href="#download">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v12m0 0 5-5m-5 5-5-5M5 20h14" /></svg>
              下载 Windows 版
            </a>
            <a class="tech-button tech-button--ghost" :href="withBase('/intro/')">查看使用文档</a>
          </div>
          <div class="tech-terminal" aria-label="快速开始命令">
            <div class="tech-terminal__bar"><span></span><span></span><span></span><b>quickstart.ps1</b></div>
            <pre><code><i># 本地运行桌面端</i>
npm run tauri dev

<i># 构建 Windows 安装包</i>
npm run tauri:build:windows</code></pre>
          </div>
        </div>

        <div class="tech-shell tech-hero__visual">
          <div class="tech-shot js-reveal" style="transition-delay: 90ms">
            <div class="tech-shot__bar"><span></span><span></span><span></span><b>QQ Zone Restore Archive / 总览</b></div>
            <img :src="withBase('/screens/dashboard.png')" alt="QQ Zone Restore Archive 总览界面" />
            <span class="tech-scanline" aria-hidden="true"></span>
          </div>
          <div class="tech-annotations js-reveal" style="transition-delay: 160ms" aria-label="产品能力">
            <span><i></i>本地 SQLite</span>
            <span><i></i>断点续传</span>
            <span><i></i>HTML 导出</span>
          </div>
        </div>
      </section>

      <section id="features" class="tech-section tech-features">
        <div class="tech-shell">
          <div class="tech-section__head js-reveal">
            <p class="tech-index">01 / CAPABILITIES</p>
            <div><h2>不是下载器，而是一套本地归档系统</h2><p>内容按原始关系落入 SQLite，媒体缓存在设备，导出后可以离线翻阅。</p></div>
          </div>
          <div class="tech-grid">
            <article class="tech-card js-reveal"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h10" /></svg><h3>完整归档</h3><p>动态正文、图片、视频、评论和互动记录按可见范围归档。</p></article>
            <article class="tech-card js-reveal" style="transition-delay: 70ms"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v7l4 2m5-2a9 9 0 1 1-3-6.7" /></svg><h3>可恢复进度</h3><p>网络中断或限流后，从已完成位置继续，不重新开始。</p></article>
            <article class="tech-card js-reveal" style="transition-delay: 140ms"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14M13 5l7 7-7 7" /></svg><h3>离线导出</h3><p>按分类或选中内容导出独立 HTML，浏览器直接打开。</p></article>
            <article class="tech-card js-reveal" style="transition-delay: 210ms"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 4h16v16H4zM4 9h16M9 9v11" /></svg><h3>本地数据库</h3><p>数据保存在设备应用目录，不提供项目方云端存储。</p></article>
            <article class="tech-card js-reveal" style="transition-delay: 280ms"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9h12M6 15h8M5 5l14 14" /></svg><h3>频率保护</h3><p>请求间隔和限流暂停机制降低被封风险，不鼓励高频抓取。</p></article>
            <article class="tech-card js-reveal" style="transition-delay: 350ms"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 12h18M12 3v18" /></svg><h3>媒体时光轴</h3><p>图片和视频按年份重新排列，离线后继续浏览已缓存内容。</p></article>
          </div>
        </div>
      </section>

      <section id="how" class="tech-section tech-how">
        <div class="tech-shell">
          <div class="tech-section__head js-reveal">
            <p class="tech-index">02 / WORKFLOW</p>
            <div><h2>从登录到离线档案的四步管线</h2><p>每一步都留在本机，任务状态可追踪，中断后可以继续。</p></div>
          </div>
          <div class="tech-steps">
            <article class="tech-step js-reveal"><span>01</span><h3>登录账号</h3><p>二维码或网页登录，Cookie 仅用于当前会话。</p><code>qzone.login</code></article>
            <article class="tech-step js-reveal" style="transition-delay: 80ms"><span>02</span><h3>启动归档</h3><p>选择任务类型，等待应用按频率保护策略抓取。</p><code>archive.start</code></article>
            <article class="tech-step js-reveal" style="transition-delay: 160ms"><span>03</span><h3>断点续传</h3><p>中断后从上次位置继续，已归档数据不丢失。</p><code>archive.resume</code></article>
            <article class="tech-step js-reveal" style="transition-delay: 240ms"><span>04</span><h3>离线导出</h3><p>导出 HTML 或继续在应用中检索本地内容。</p><code>archive.export</code></article>
          </div>
        </div>
      </section>

      <section id="stack" class="tech-section tech-stack-section">
        <div class="tech-shell">
          <div class="tech-section__head js-reveal">
            <p class="tech-index">03 / STACK</p>
            <div><h2>可信赖的本地技术栈</h2><p>桌面端、移动端与归档引擎共用同一套边界清晰的实现。</p></div>
          </div>
          <div class="tech-marquee" aria-label="技术栈列表">
            <div class="tech-marquee__track">
              <span v-for="(item, index) in marqueeItems" :key="`${item}-${index}`">{{ item }}</span>
            </div>
          </div>
        </div>
      </section>

      <section id="data" class="tech-section tech-data">
        <div class="tech-shell">
          <div class="tech-section__head js-reveal">
            <p class="tech-index">04 / DATA</p>
            <div><h2>数据在本机完成一次完整旅程</h2><p>没有同步服务器，没有隐藏的云端副本，导出文件由你自己保管。</p></div>
          </div>
          <div class="tech-flow js-reveal" aria-label="本地数据流">
            <div><small>01</small><strong>QQ 空间会话</strong><span>Cookie 仅用于当前会话</span></div><i>→</i>
            <div><small>02</small><strong>本机归档任务</strong><span>请求与进度在设备上处理</span></div><i>→</i>
            <div><small>03</small><strong>SQLite + 媒体缓存</strong><span>数据保存在设备目录</span></div><i>→</i>
            <div><small>04</small><strong>离线 HTML</strong><span>导出后无需联网浏览</span></div>
          </div>
          <div class="tech-local-proof js-reveal"><span>LOCAL ONLY</span><p>没有账号系统，没有同步服务器，没有隐藏的云端副本。</p><a :href="withBase('/data-and-safety/')">阅读数据说明 →</a></div>
        </div>
      </section>

      <section id="download" class="tech-section tech-download">
        <div class="tech-shell">
          <div class="tech-section__head js-reveal">
            <p class="tech-index">05 / DOWNLOAD</p>
            <div><h2>选择你的设备</h2><p>从 GitHub Release 获取最新安装包。</p></div>
          </div>
          <div class="tech-downloader js-reveal">
            <div class="tech-platform-tabs" role="tablist" aria-label="选择平台">
              <button v-for="platform in platforms" :key="platform.id" type="button" role="tab" :aria-selected="selectedPlatform === platform.id" :class="{ active: selectedPlatform === platform.id }" @click="selectedPlatform = platform.id">{{ platform.label }}</button>
            </div>
            <Transition name="platform-switch" mode="out-in">
              <div class="tech-download__body" :key="selectedPlatform">
                <div>
                  <p class="tech-download__eyebrow">LATEST RELEASE / {{ activePlatform.label.toUpperCase() }}</p>
                  <h3>{{ activePlatform.label }}</h3>
                  <p>{{ activePlatform.note }}</p>
                  <div class="tech-specs"><span><small>格式</small>{{ activePlatform.format }}</span><span><small>架构</small>{{ activePlatform.arch }}</span><span><small>系统</small>{{ activePlatform.version }}</span></div>
                </div>
                <a class="tech-button tech-button--download" :href="releaseUrl" target="_blank" rel="noreferrer"><span>前往 Release 下载</span><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7m0 0H8m9 0v9" /></svg></a>
              </div>
            </Transition>
          </div>
          <aside class="tech-warning js-reveal" style="transition-delay: 120ms"><strong>恢复范围说明</strong><p>归档依赖 QQ 空间当前可见的互动列表。受权限、删除状态或平台接口限制，部分历史内容可能无法恢复。</p></aside>
        </div>
      </section>
    </main>

    <footer class="tech-footer">
      <div class="tech-shell">
        <div class="tech-brand"><img class="tech-brand__mark" :src="withBase('/qzone-archive-icon.png')" alt="" /><span>QQ Zone Restore Archive</span></div>
        <p>把个人记忆，留在自己的设备上。</p>
        <nav aria-label="页脚导航"><a :href="withBase('/intro/')">使用文档</a><a :href="githubUrl">GitHub</a><a :href="`${githubUrl}/blob/main/LICENSE`">许可证</a></nav>
      </div>
    </footer>
  </div>
</template>
