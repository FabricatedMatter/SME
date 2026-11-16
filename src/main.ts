import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { startSky } from "./lib/sky";
import { loadGameFont } from "./lib/theme";

async function boot() {
  startSky(document.getElementById("sky")!);
  await loadGameFont();
  mount(App, { target: document.getElementById("app")! });
}

void boot();
