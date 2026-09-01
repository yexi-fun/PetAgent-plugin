const status = document.querySelector("#status");
const configView = document.querySelector("#config");
const rpc = async (method, params = {}) => {
  const response = await fetch(new URL("../__petagent/rpc", window.location.href), {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ protocolVersion: 1, id: crypto.randomUUID(), method, params })
  });
  const payload = await response.json();
  if (!payload.ok) throw new Error(payload.error?.message || "RPC failed");
  return payload.result;
};

async function load() {
  const info = await fetch(new URL("../__petagent/host-info.json", window.location.href)).then((response) => response.json());
  status.textContent = `Connected to PetAgent ${info.hostVersion} (${info.pluginVersion})`;
  const current = await rpc("config.get");
  configView.textContent = JSON.stringify(current, null, 2);
  document.querySelector("#save").addEventListener("click", async () => {
    const saved = await rpc("config.set", { config: { theme: "dark", updatedBy: "frontend-sample" }, expectedRevision: current.revision });
    configView.textContent = JSON.stringify(saved, null, 2);
  });
  document.querySelector("#close").addEventListener("click", () => void rpc("window.close"));
  window.addEventListener("pet-plugin:event", (event) => {
    if (event.detail?.name === "pet-plugin-frontend-dispose") status.textContent = "PetAgent is closing this plugin";
  });
}

load().catch((error) => { status.textContent = `PetAgent handshake/RPC failed: ${error.message}`; });
