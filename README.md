# SME: Starbound Microdungeon Editor

A dedicated editor for Starbound (micro)dungeons, built to replace the generic
[Tiled](https://www.mapeditor.org/) workflow. Desktop app: Rust ([Tauri 2](https://tauri.app)) backend, Svelte 5 UI.

> Work in progress.

## Why not Tiled?

Tiled is a generic tile-map editor, so making a Starbound dungeon in it means hunting through
huge tileset sheets, remembering what each magenta marker tile does, hand-writing biome
`.patch` files, `.treasurepools` and chest `parameters` JSON, and finding mistakes only in
`starbound.log`. SME knows Starbound's formats and does that work for you, while still
reading and writing the exact files the game (and Tiled) understands.

## Game data

SME reads blocks, objects, tilesets and loot pools **at runtime from your own Starbound
install** (`assets/packed.pak` and the `tiled/` folder). Nothing from the game is copied into
this repository. The install is found in the usual Steam library locations; you can point SME
at it in Settings.

## Development

```sh
npm install
npm run tauri dev           # run the app
cd src-tauri && cargo test  # tests (the vanilla round-trip tests need Starbound installed)
```

Release build: `npx tauri build`.

## License

MIT
