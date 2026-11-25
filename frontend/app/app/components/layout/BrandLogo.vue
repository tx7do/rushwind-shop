<script setup lang="ts">
// RushWind R monogram（深蓝圆角方块 + 渐变 R）。此前是 gowind 风格的
// 三道弧线内联 SVG；改引 public/logo.svg 的官方图形，避免内联渐变 id
// 在同页多实例下的碰撞退化问题。
</script>

<template>
  <img
    src="/logo.svg"
    alt="RushWind"
    class="block size-full object-contain"
    role="img"
  />
</template>
