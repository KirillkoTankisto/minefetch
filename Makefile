PKGVER        := 1.6.6
TARGET_DIR    := target
TARGETS       := x86_64-unknown-linux-musl
CARGO         := cargo build
CARGO_FLAGS   := -rq --target
COMMAND       := $(CARGO) $(CARGO_FLAGS)

# default
all: set-version $(TARGETS) package

# set current version
set-version:
	@cargo set-version $(PKGVER)

# x86_64
x86_64-unknown-linux-musl:
	@echo ":out: Building $@"
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
