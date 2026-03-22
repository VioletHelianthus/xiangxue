<template>
  <div class="device-frame-wrapper" ref="wrapperRef">
    <div class="device-frame" :style="frameStyle">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'

const props = withDefaults(defineProps<{
  width?: number
  height?: number
}>(), {
  width: 640,
  height: 960,
})

const wrapperRef = ref<HTMLElement | null>(null)
const availW = ref(800)
const availH = ref(600)

function measure() {
  if (wrapperRef.value) {
    availW.value = wrapperRef.value.clientWidth
    availH.value = wrapperRef.value.clientHeight
  }
}

let ro: ResizeObserver | null = null
onMounted(() => {
  measure()
  ro = new ResizeObserver(measure)
  if (wrapperRef.value) ro.observe(wrapperRef.value)
})
onUnmounted(() => ro?.disconnect())

const scale = computed(() => {
  const sx = availW.value / props.width
  const sy = availH.value / props.height
  return Math.min(sx, sy, 1)
})

const frameStyle = computed(() => ({
  width: `${props.width}px`,
  height: `${props.height}px`,
  transform: `scale(${scale.value})`,
}))
</script>

<style scoped>
.device-frame-wrapper {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.device-frame {
  position: relative;
  overflow: hidden;
  background: #2a2a3e;
  border: 2px solid #444;
  border-radius: 4px;
  transform-origin: center center;
  flex-shrink: 0;
}
</style>
