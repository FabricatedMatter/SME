import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { loadGameFont, paintSky } from "./lib/theme";

async function boot() {
  const sky = document.getElementById("sky") as HTMLCanvasElement;
  paintSky(sky);
  let t: number | undefined;
  window.addEventListener("resize", () => {
    clearTimeout(t);
    t = window.setTimeout(() => paintSky(sky), 150);
  });
  await loadGameFont();
  mount(App, { target: document.getElementById("app")! });
}

void boot();
