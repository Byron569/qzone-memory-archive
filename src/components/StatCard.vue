<script setup lang="ts">
defineProps<{ label: string; value: string; hint: string; icon: string; tone?: string }>();
</script>

<template>
  <article class="stat-card">
    <span class="stat-icon" :class="`tone-${tone ?? 'blue'}`"><i :class="icon" /></span>
    <div><p>{{ label }}</p><strong>{{ value }}</strong><small>{{ hint }}</small></div>
  </article>
</template>
