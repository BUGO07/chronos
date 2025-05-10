# Nuke built-in rules and variables.
MAKEFLAGS += -rR
.SUFFIXES:

# This is the name that our final executable will have.
# Change as needed.
override OUTPUT := chronos

# Convenience macro to reliably declare user overridable variables.
override USER_VARIABLE = $(if $(filter $(origin $(1)),default undefined),$(eval override $(1) := $(2)))

$(call USER_VARIABLE,KARCH,x86_64)

ifeq ($(RUST_TARGET),)
    override RUST_TARGET := $(KARCH)-unknown-none
endif

ifeq ($(RUST_PROFILE),)
    override RUST_PROFILE := dev
endif

ifeq ($(KARCH),aarch64)
	ifeq ($(RUST_PROFILE),release)
		override RUST_PROFILE := armrel
	endif
endif

override RUST_PROFILE_SUBDIR := $(RUST_PROFILE)
override RUST_FLAGS := "-C relocation-model=static"
ifeq ($(RUST_PROFILE),dev)
    override RUST_PROFILE_SUBDIR := debug
    override RUST_FLAGS := "-C relocation-model=static -C force-frame-pointers=yes"
endif

# Default target.
.PHONY: $(KARCH)
$(KARCH):
	RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE)
	cp target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$$(cd target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR) && find -maxdepth 1 -perm -111 -type f) chronos-$(KARCH)

.PHONY: test
test:
	RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE) --features tests
	cp target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$$(cd target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR) && find -maxdepth 1 -perm -111 -type f) chronos-test

# add --features uacpi_test (crashes for some reason)
.PHONY: uacpi-test
uacpi-test:
	RUSTFLAGS="-C relocation-model=static" cargo build --target $(RUST_TARGET) --profile release --features uacpi_test
	cp target/$(RUST_TARGET)/release/$$(cd target/$(RUST_TARGET)/release && find -maxdepth 1 -perm -111 -type f) chronos-uacpi-test

# Remove object files and the final executable.
.PHONY: clean
clean:
	cargo clean
	rm -rf chronos*

.PHONY: distclean
distclean: clean
