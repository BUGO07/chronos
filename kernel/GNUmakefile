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
	ifeq ($(KARCH),riscv64)
    	override RUST_TARGET := riscv64gc-unknown-none-elf
	endif
endif

ifeq ($(RUST_PROFILE),)
    override RUST_PROFILE := dev
endif

override RUST_PROFILE_SUBDIR := $(RUST_PROFILE)
override RUST_FLAGS := "-C relocation-model=static"
ifeq ($(RUST_PROFILE),dev)
    override RUST_PROFILE_SUBDIR := debug
    override RUST_FLAGS := "-C relocation-model=static -C force-frame-pointers=yes"
endif

.PHONY: $(KARCH)
$(KARCH):
	RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE)
	cp ../target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$(OUTPUT) $(OUTPUT)-$(KARCH)

.PHONY: test
test:
	RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE) --features tests
	cp ../target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$(OUTPUT) $(OUTPUT)-test

.PHONY: clean
clean:
	cargo clean
	rm -rf $(OUTPUT)*

.PHONY: distclean
distclean: clean
