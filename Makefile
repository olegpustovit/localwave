# make dev      debug build, incremental, runs in this terminal
# make run      optimized release build, runs in this terminal
# make install  release build signed with your Apple Development identity -> /Applications
# All three share one bundle id, signature and sandbox database.
.PHONY: dev run install
dev:
	./scripts/run-mac.sh
run:
	CONFIG=Release ./scripts/run-mac.sh
install:
	CONFIG=Release INSTALL=1 ./scripts/run-mac.sh
