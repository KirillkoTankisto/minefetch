PKGVER        := 1.6.5
TARGET_DIR    := target
TARGETS       := x86_64-unknown-linux-musl aarch64-unknown-linux-musl riscv64gc-unknown-linux-musl
CARGO         := cargo +nightly
CARGO_FLAGS   := -rqt --config package.version=$(PKGVER)
BUILD_STD     := -Z build-std
COMMAND       := $(CARGO) build $(BUILD_STD) -rqt --config package.version=$(PKGVER)

# default
all: $(TARGETS) package

# x86_64
x86_64-unknown-linux-musl:
	@echo ":out: Building $@"
	@$(COMMAND) $@
	@echo ":out: Finished building for $@"

# aarch64
aarch64-unknown-linux-musl:
	@echo ":out: Building $@"
	@$(COMMAND) $@
	@echo ":out: Finished building for $@"

# riscv64gc
riscv64gc-unknown-linux-musl:
	@echo ":out: Building $@"
	@RUSTFLAGS="-C target-feature=+crt-static" \
	@$(COMMAND) $@
	@echo ":out: Finished building for $@"

# clean workspace
clean:
	@echo ":out: Cleaning"
	@cargo clean -q
	@echo ":out: Done"

# tar up each release
package:
	@for t in $(TARGETS); do \
	  echo ":out: Packaging $$t"; \
	  cd $(TARGET_DIR)/$$t/release && \
	  tar czf minefetch-${PKGVER}-$$t.tar.gz minefetch && \
	  mv minefetch-${PKGVER}-$$t.tar.gz ../../../build-cross/; \
	  cd ../../../; \
	done

.PHONY: all clean package $(TARGETS)
