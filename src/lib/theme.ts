import { invoke } from "@tauri-apps/api/core";
import "@fontsource/pixelify-sans/400.css";

/** Load the Starbound UI font from the user's own install; falls back to Pixelify Sans. */
export async function loadGameFont(): Promise<boolean> {
  try {
    const buf = await invoke<ArrayBuffer>("ui_font");
    const face = new FontFace("Hobo", buf);
    await face.load();
    document.fonts.add(face);
    document.documentElement.classList.add("game-font");
    return true;
  } catch {
    return false;
  }
}
