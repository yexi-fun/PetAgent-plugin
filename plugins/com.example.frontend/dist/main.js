const status = document.querySelector("#status");
fetch("../__petagent/host-info.json")
  .then((response) => response.json())
  .then((info) => { status.textContent = `Connected to PetAgent ${info.hostVersion} (${info.pluginVersion})`; })
  .catch(() => { status.textContent = "PetAgent handshake failed"; });
