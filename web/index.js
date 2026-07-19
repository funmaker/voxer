
const params = new URLSearchParams(window.location.search);
const backend = params.get("backend");

const webgpuRadio = document.getElementById("webgpuRadio");
const webgl2Radio = document.getElementById("webgl2Radio");

function redirect(newBackend) {
  if(newBackend === backend) return;
  
  window.location.assign(`${window.location.origin}${window.location.pathname}?backend=${newBackend}`);
}

if(backend === "webgl2") {
  import(`./webgl2.js`).then((module) => module.default());
} else if(backend === "webgpu") {
  import(`./webgpu.js`).then((module) => module.default());
} else if(navigator.gpu) {
  redirect("webgpu");
} else {
  redirect("webgl2");
}

webgpuRadio.checked = backend === "webgpu";
webgl2Radio.checked = backend === "webgl2";

webgpuRadio.addEventListener("change", (ev) => {
  webgl2Radio.checked = false;
  redirect("webgpu")
});

webgl2Radio.addEventListener("change", (ev) => {
  webgpuRadio.checked = false;
  redirect("webgl2")
});
