const zh = navigator.language?.startsWith("zh");

const t = (en, zhCn) => zh ? zhCn : en;

const loader = () => document.getElementById("web-loader");
const progress = () => document.getElementById("wasm-progress");
const status = () => document.getElementById("wasm-status");
const fallbackWasmUrl = () => new URL("icu_tool_bg.wasm", window.location.href).toString();

function setStatus(text) {
  const el = status();
  if (el) {
    el.textContent = text;
  }
}

function setDeterminate(current, total) {
  const el = progress();
  if (!el) {
    return;
  }
  el.max = total;
  el.value = current;
  el.removeAttribute("aria-busy");
}

function setIndeterminate() {
  const el = progress();
  if (!el) {
    return;
  }
  el.removeAttribute("value");
  el.removeAttribute("max");
  el.setAttribute("aria-busy", "true");
}

function setError(error) {
  loader()?.classList.add("is-error");
  setStatus(`${t("Failed", "失败")}: ${error}`);
}

export default function initializer() {
  let wasmUrl = null;
  let complete = false;
  let progressed = false;

  return {
    onStart: (source) => {
      complete = false;
      progressed = false;
      wasmUrl = (typeof source === "string" ? source : source?.url) ?? fallbackWasmUrl();
      loader()?.classList.remove("is-error", "is-complete");
      setDeterminate(0, 100);
      setStatus(t("Downloading…", "下载中…"));

      if (wasmUrl && "caches" in window) {
        caches.match(wasmUrl).then((response) => {
          if (!response || progressed || complete) {
            return;
          }
          setIndeterminate();
          setStatus(t("Loading from cache…", "从缓存加载…"));
        }).catch(() => {});
      }
    },
    onProgress: ({ current, total }) => {
      progressed = true;
      if (total > 0) {
        setDeterminate(current, total);
        setStatus(`${Math.floor(current / total * 100)}%`);
      } else {
        setIndeterminate();
        setStatus(`${Math.floor(current / 1024)} KiB`);
      }
    },
    onSuccess: () => {
      setDeterminate(100, 100);
      setStatus(t("Starting…", "启动中…"));
    },
    onFailure: (error) => {
      setError(error);
    },
    onComplete: () => {
      complete = true;
      loader()?.classList.add("is-complete");
    },
  };
}
