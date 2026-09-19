<script setup lang="ts">
import { reactive } from "vue";
import { parseQzoneText } from "../utils/qzoneText";

defineProps<{ value?: string }>();

const emojiFormats = reactive<Record<string, "png" | "missing">>({});
const emojiUrl = (code: string) => `https://qzonestyle.gtimg.cn/qzone/em/${code}.${emojiFormats[code] === "png" ? "png" : "gif"}`;

function handleEmojiError(code: string) {
  if (!emojiFormats[code]) emojiFormats[code] = "png";
  else emojiFormats[code] = "missing";
}
</script>

<template>
  <template v-for="(part, index) in parseQzoneText(value)" :key="`${index}-${part.type}-${part.value}`">
    <span v-if="part.type === 'emoji' && emojiFormats[part.value] === 'missing'" class="qzone-emoji-missing" :title="`QQ 表情 ${part.value}`">[表情]</span>
    <img v-else-if="part.type === 'emoji'" class="qzone-emoji" :src="emojiUrl(part.value)" :alt="`[QQ 表情 ${part.value}]`" :title="`QQ 表情 ${part.value}`" loading="lazy" referrerpolicy="no-referrer" @error="handleEmojiError(part.value)" />
    <span v-else :class="{ 'qzone-mention': part.type === 'mention' }" :title="part.uin ? `QQ ${part.uin}` : undefined">{{ part.value }}</span>
  </template>
</template>
