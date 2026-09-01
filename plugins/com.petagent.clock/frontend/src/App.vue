<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

interface ClockPayload {
  text?: string;
  date?: string | null;
  timezone?: string;
  source?: string;
  visible?: boolean;
}

const time = ref("暂不可用");
const date = ref("");
const timezone = ref("");
const available = ref(false);
let stop: (() => void) | null = null;

async function rpc<T>(method: string, params: unknown = {}): Promise<T> {
  const response = await fetch(new URL("../__petagent/rpc", window.location.href), {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ protocolVersion: 1, id: crypto.randomUUID(), method, params }),
  });
  const result = await response.json();
  if (!result.ok) throw new Error(result.error?.message ?? "宿主连接失败");
  return result.result as T;
}

function applyClock(payload: ClockPayload) {
  if (!payload.text || payload.source !== "windows-system-clock") return;
  time.value = payload.text;
  date.value = payload.date ?? "";
  timezone.value = payload.timezone ?? "";
  available.value = true;
}

onMounted(async () => {
  const handler = (event: Event) => {
    const detail = (event as CustomEvent<{ name?: string; payload?: ClockPayload }>).detail;
    if (detail?.name === "clock.updated" && detail.payload) applyClock(detail.payload);
    if (detail?.name === "clock.visibility" && detail.payload && detail.payload.visible === false) {
      void rpc("window.close");
    }
  };
  window.addEventListener("pet-app:event", handler);
  stop = () => window.removeEventListener("pet-app:event", handler);
  try {
    await rpc("app.events.subscribe");
    const current = await rpc<ClockPayload>("app.invoke", {
      capability: "clock.now",
      input: {},
    });
    applyClock(current);
  } catch {
    available.value = false;
  }
});

onBeforeUnmount(() => stop?.());
</script>

<template>
  <main class="clock" :class="{ unavailable: !available }">
    <div class="time">{{ time }}</div>
    <div class="meta">
      <span v-if="date">{{ date }}</span>
      <span v-if="timezone">UTC{{ timezone }}</span>
    </div>
  </main>
</template>
