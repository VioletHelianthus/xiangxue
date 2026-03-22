<template>
  <div class="preview-shell">
    <!-- Top toolbar -->
    <div class="toolbar">
      <!-- Resolution input -->
      <div class="toolbar-group">
        <label class="toolbar-label">Resolution</label>
        <input
          class="res-input"
          type="number"
          :value="width"
          @change="onWidthChange"
          min="1"
        />
        <span class="res-sep">&times;</span>
        <input
          class="res-input"
          type="number"
          :value="height"
          @change="onHeightChange"
          min="1"
        />
        <button class="btn-swap" @click="swapResolution" title="Swap width/height">&#8646;</button>
      </div>

      <!-- Presets -->
      <div class="toolbar-group presets">
        <label class="toolbar-label">Presets</label>
        <div class="preset-list">
          <button
            v-for="p in config.presets"
            :key="p.label"
            class="preset-btn"
            :class="{ active: width === p.width && height === p.height }"
            @click="applyPreset(p)"
            :title="p.label"
          >
            {{ p.width }}&times;{{ p.height }}
          </button>
        </div>
      </div>

      <!-- Info -->
      <div class="toolbar-group toolbar-info">
        <span class="scale-info">{{ scalePercent }}%</span>
      </div>
    </div>

    <!-- Device frame area -->
    <div class="frame-area">
      <DeviceFrame
        :width="width"
        :height="height"
        :design-width="config.designResolution.width"
        :design-height="config.designResolution.height"
      >
        <slot />
      </DeviceFrame>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import DeviceFrame from './DeviceFrame.vue'
import type { PreviewConfig, Resolution } from './previewConfig'

const props = defineProps<{
  config: PreviewConfig
}>()

const width = ref(props.config.designResolution.width)
const height = ref(props.config.designResolution.height)

function onWidthChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  if (v > 0) width.value = v
}

function onHeightChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  if (v > 0) height.value = v
}

function swapResolution() {
  const tmp = width.value
  width.value = height.value
  height.value = tmp
}

function applyPreset(p: Resolution) {
  width.value = p.width
  height.value = p.height
}

const scalePercent = computed(() => {
  // Approximate: actual scale is computed inside DeviceFrame
  const maxW = window.innerWidth - 40
  const maxH = window.innerHeight - 120
  const s = Math.min(maxW / width.value, maxH / height.value, 1)
  return Math.round(s * 100)
})
</script>

<style scoped>
.preview-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1a1a1a;
  color: #ccc;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 13px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  background: #252525;
  border-bottom: 1px solid #333;
  flex-shrink: 0;
  flex-wrap: wrap;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.toolbar-label {
  color: #888;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-right: 4px;
}

.res-input {
  width: 64px;
  padding: 4px 6px;
  background: #333;
  border: 1px solid #444;
  border-radius: 3px;
  color: #eee;
  font-size: 13px;
  text-align: center;
}
.res-input:focus {
  outline: none;
  border-color: #6af;
}
/* Hide number input spinners */
.res-input::-webkit-inner-spin-button,
.res-input::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.res-input { -moz-appearance: textfield; }

.res-sep {
  color: #666;
  font-size: 14px;
}

.btn-swap {
  padding: 3px 8px;
  background: #333;
  border: 1px solid #444;
  border-radius: 3px;
  color: #aaa;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
}
.btn-swap:hover {
  background: #444;
  color: #fff;
}

.presets {
  flex: 1;
  min-width: 0;
}

.preset-list {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.preset-btn {
  padding: 3px 8px;
  background: #333;
  border: 1px solid #444;
  border-radius: 3px;
  color: #aaa;
  cursor: pointer;
  font-size: 11px;
  white-space: nowrap;
}
.preset-btn:hover {
  background: #444;
  color: #fff;
}
.preset-btn.active {
  background: #2a4a6a;
  border-color: #6af;
  color: #6af;
}

.toolbar-info {
  margin-left: auto;
}

.scale-info {
  color: #666;
  font-size: 12px;
}

.frame-area {
  flex: 1;
  display: flex;
  padding: 16px;
  min-height: 0;
  overflow: hidden;
}
</style>
