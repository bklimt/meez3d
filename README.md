
# meez3d

meez3d is ...

## meez3d_wasm

This is an implementation of the meez3d game for use as a WASM web app.

To build meez3d for WASM, you need to have `wasm-pack` installed:
```
cargo install wasm-pack
```

Build meez3d for WASM:

```
wasm-pack build meez3d_wasm --target web
```

Run a testing server with meez3d:
```
cd meez3d_wasm
python3 -m http.server
```

To update the hosted version (from the repo root):
```
make release
```

