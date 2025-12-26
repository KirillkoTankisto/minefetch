PKGVER        := 1.6.9
TARGET_DIR    := target
TARGETS       := x86_64-unknown-linux-musl aarch64-unknown-linux-musl riscv64gc-unknown-linux-musl
CARGO         := cargo zigbuild
CARGO_FLAGS   := -rq --target
COMMAND       := $(CARGO) $(CARGO_FLAGS)

all: build package

# Set current version to PKGVER
set-version:
	@cargo set-version $(PKGVER)

# Builds all targets in TARGETS
build: set-version
	@set -e; \
    for t in $(TARGETS); do \
	    printf ":out: Building $$t\n"; \
		$(COMMAND) $$t || { printf ":err: Build failed for $$t\n\n"; exit 1; }; \
		printf ":out: Finished building $$t\n\n"; \
	done

# Native target
native: set-version
	@set -e; \
	echo ":out: Building $@"; \
	cargo build -rq || { echo ":err: Build failed for $@"; exit 1; }; \
	echo ":out: Finished building $@";

# Stub
Makefile: ;

# Custom target triple
%: set-version
	@set -e; \
	printf ":out: Building $@\n"; \
	$(COMMAND) $@ || { printf ":err: Build failed for $@\n"; exit 1; }; \
	printf ":out: Finished building $@\n";

# tar up each release
package:
	@set -e; \
    for t in $(TARGETS); do \
        echo ":out: Packaging $$t"; \
        cd $(TARGET_DIR)/$$t/release && \
        tar czf minefetch-${PKGVER}-$$t.tar.gz minefetch && \
        mv minefetch-${PKGVER}-$$t.tar.gz ../../../build-cross/; \
        cd ../../../; \
	done

# clean workspace
clean:
	@set -e; \
	printf ":out: Cleaning\n"; \
	cargo clean -q; \
	echo ":out: Done";

.PHONY: all set-version build native package clean
