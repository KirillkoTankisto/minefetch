CARGO         := cargo +nightly
BUILD_STD     := -Z build-std
PKGVER        := 1.6.4
TARGET_DIR    := target
TARGETS       := x86_64-unknown-linux-musl aarch64-unknown-linux-musl riscv64gc-unknown-linux-musl

# default
all: $(TARGETS) package

# x86_64
x86_64-unknown-linux-musl:
	@echo ":out: Building $@"
	@$(CARGO) build $(BUILD_STD) --release --target $@ -q \

# aarch64
aarch64-unknown-linux-musl:
	@echo ":out: Building $@"
	@$(CARGO) build $(BUILD_STD) --release --target $@ -q \

# riscv64gc
riscv64gc-unknown-linux-musl:
	@echo ":out: Building $@"
	@RUSTFLAGS="-C target-feature=+crt-static" \
	$(CARGO) build $(BUILD_STD) --release --target $@ -q \

# clean workspace
clean:
	@echo ":out: Cleaning"
	@cargo clean -q

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
