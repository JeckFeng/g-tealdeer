<script setup lang="ts">
import { ref, watch, computed } from 'vue';

interface Props {
  message: string;
  visible: boolean;
  duration?: number;
  type?: 'success' | 'error';
}

const props = withDefaults(defineProps<Props>(), {
  duration: 2000,
  type: 'success'
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const show = ref(props.visible);

const icon = computed(() => {
  return props.type === 'error' ? '✕' : '✓';
});

watch(() => props.visible, (newVal) => {
  show.value = newVal;
  if (newVal) {
    setTimeout(() => {
      show.value = false;
      emit('update:visible', false);
    }, props.duration);
  }
});
</script>

<template>
  <Transition name="toast">
    <div v-if="show" class="toast" :class="type">
      <span class="toast-icon">{{ icon }}</span>
      <span class="toast-message">{{ message }}</span>
    </div>
  </Transition>
</template>

<style scoped>
.toast {
  position: fixed;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  color: #ffffff;
  padding: 12px 20px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  display: flex;
  align-items: center;
  gap: 8px;
  z-index: 1000;
  font-size: 14px;
  font-weight: 500;
}

.toast.success {
  background: #4f87ff;
}

.toast.error {
  background: #e74c3c;
}

.toast-icon {
  font-size: 16px;
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(-50%) translateY(-20px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-20px);
}
</style>
