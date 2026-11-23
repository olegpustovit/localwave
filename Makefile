PROFILE ?= release
APPDIR ?= /Applications
CARGOFLAGS = $(if $(filter release,$(PROFILE)),--release)
# COUNTING=1 (set by `performance`) builds the counting libpxui and app: the zero-allocation checks count both sides of the ABI.
# Shipping builds leave it empty and keep the system allocator.
COUNTING ?=
PRODUCTS = target/$(if $(filter release,$(PROFILE)),release,debug)

.PHONY: lib build bundle sign notarize install check performance ios-build ios-run

target/sdl/lib/libSDL3.a:
	packaging/build-sdl-macos.sh

lib:
	@test -e pxui/Cargo.toml || git submodule update --init pxui
	cargo build --locked $(CARGOFLAGS) --manifest-path pxui/Cargo.toml -p pxui_dylib $(if $(COUNTING),--features counting) --target-dir target
	ln -sfn libpxui.dylib $(PRODUCTS)/libpxui.2.dylib

build: target/sdl/lib/libSDL3.a lib
	cargo build --locked $(CARGOFLAGS) -p lw_bin --bin localwave $(if $(COUNTING),--features counting)

bundle: build
	packaging/bundle-macos.sh '$(PRODUCTS)'

sign: build
	tools/sign-macos.sh '$(PRODUCTS)'

notarize:
	tools/notarize-macos.sh

install: bundle
	@test ! -e '$(APPDIR)/LocalWave.app' || { echo 'Refusing to overwrite an installed LocalWave.app; choose APPDIR or move it first.' >&2; exit 1; }
	mkdir -p '$(APPDIR)'
	ditto target/bundle/LocalWave.app '$(APPDIR)/LocalWave.app'

check: target/sdl/lib/libSDL3.a
	python3 packaging/test_ios.py
	python3 tools/test_split_rs.py
	cargo t -p lw_sqlite -p lw_core -p lw_apple_sys -p lw_ui -p lw_bin
	@# Shipping builds keep the system allocator: no normal dependency may turn on mem_sys/counting (only COUNTING=1 does).
	@! cargo tree --locked -q --workspace --target all -e normal,features -i mem_sys --prefix none | grep -qx 'mem_sys feature "counting"' \
	  || { echo 'check: a normal dependency enables mem_sys/counting in the shipping build' >&2; exit 1; }
	tools/check-pxui-remote.sh

performance: PROFILE = release
performance: COUNTING = 1
performance: PERFORMANCE_CHECK = target/bundle/LocalWave.app/Contents/MacOS/localwave --performance-check -ApplePersistenceIgnoreState YES
performance: bundle
	tools/check-macos-bundle.sh
	@performance_result=0; \
	  $(PERFORMANCE_CHECK) || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-find || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-queue || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-queue-drag || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-queue-sheet || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-artwork || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-waveform || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-settings || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-files || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-drop || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-artists || performance_result=1; \
	  PXUI_FRAME_REASONS=1 $(PERFORMANCE_CHECK) --performance-song-indicators || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-long-scroll || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-album-detail || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-artist-detail || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-list-dialogs || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-metadata-editor || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-song-drag || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-fling || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-reverse-scroll || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-jump || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-non-song-lists || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-search-typing || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-first-navigation || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-text-typing || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-first-menu || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-resize || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-theme-switch || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-idle-600 || performance_result=1; \
	  $(PERFORMANCE_CHECK) --performance-clock-120 || performance_result=1; \
	  exit "$$performance_result"

# iOS builds use the shared target/ and keep the shipping Swift app installed (release, like the Mac app).

ios-build:
	python3 packaging/ios.py build --profile '$(PROFILE)'

ios-run:
	python3 packaging/ios.py run --profile '$(PROFILE)'
