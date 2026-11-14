PROFILE ?= release
APPDIR ?= /Applications
CARGOFLAGS = $(if $(filter release,$(PROFILE)),--release)
PRODUCTS = target/$(if $(filter release,$(PROFILE)),release,debug)

.PHONY: lib build bundle install check performance ios-build ios-run

target/sdl/lib/libSDL3.a:
	packaging/build-sdl-macos.sh

lib:
	@test -e pxui/Cargo.toml || git submodule update --init pxui
	cargo build --locked $(CARGOFLAGS) --manifest-path pxui/Cargo.toml -p pxui_dylib --target-dir target
	ln -sfn libpxui.dylib $(PRODUCTS)/libpxui.1.dylib

build: target/sdl/lib/libSDL3.a lib
	cargo build --locked $(CARGOFLAGS) -p lw_bin --bin localwave

bundle: build
	packaging/bundle-macos.sh '$(PRODUCTS)'

install: bundle
	@test ! -e '$(APPDIR)/LocalWave.app' || { echo 'Refusing to overwrite an installed LocalWave.app; choose APPDIR or move it first.' >&2; exit 1; }
	mkdir -p '$(APPDIR)'
	ditto target/bundle/LocalWave.app '$(APPDIR)/LocalWave.app'

check: target/sdl/lib/libSDL3.a
	python3 packaging/test_ios.py
	cargo t -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin
	tools/check-pxui-remote.sh

performance: PROFILE = release
performance: bundle
	tools/check-macos-bundle.sh
	@performance_result=0; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-find || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-queue || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-queue-drag || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-queue-sheet || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-artwork || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-waveform || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-settings || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-files || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-drop || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-artists || performance_result=1; \
	  target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check --performance-song-indicators || performance_result=1; \
	  exit "$$performance_result"

# iOS development builds use the shared target/ and keep the shipping Swift app installed.
ios-build ios-run: PROFILE = debug

ios-build:
	python3 packaging/ios.py build --profile '$(PROFILE)'

ios-run:
	python3 packaging/ios.py run --profile '$(PROFILE)'
