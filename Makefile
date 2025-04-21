
run_meez3d_wpgu:
	cargo run --bin=meez3d_wgpu

run_meez3d_winit:
	cargo run --bin=meez3d_winit --no-default-features --features wgpu

wasm_pack:
	wasm-pack build meez3d_wasm --target web

test_server: wasm_pack
	cd meez3d_wasm && python3 -m http.server

release: wasm_pack
	rm -rf docs/pkg
	cp -R meez3d_wasm/index.html docs/
	cp -R meez3d_wasm/pkg docs/
	rm docs/pkg/.gitignore

release_test_server: release
	cd docs && python3 -m http.server

release_itch:
	mkdir -p itch/pkg
	rm -rf itch/pkg
	cp meez3d_wasm/itch.html itch/index.html
	cp -R meez3d_wasm/pkg itch/
	rm itch/pkg/.gitignore
	cd itch && zip -r itch *
	mv itch/itch.zip ./

