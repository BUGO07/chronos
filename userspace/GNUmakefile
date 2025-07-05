MAKEFLAGS += -rR
.SUFFIXES:

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

PROGRAM_DIRS := $(filter-out elfs,$(shell find . -mindepth 1 -maxdepth 1 -type d -printf "%f\n"))

.PHONY: $(KARCH)
$(KARCH): distclean
	@for prog in $(PROGRAM_DIRS); do \
		echo "Building $$prog..."; \
		cd $$prog && \
		RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE) && \
		cp ../../target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$$prog ../elfs/$$prog; \
		cd ..; \
	done

.PHONY: test
test: distclean
	@for prog in $(PROGRAM_DIRS); do \
		echo "Building test $$prog..."; \
		cd $$prog && \
		RUSTFLAGS=$(RUST_FLAGS) cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE) --features tests && \
		cp ../../target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/$$prog ../elfs/$$prog-test; \
		cd ..; \
	done

.PHONY: clean
clean: distclean
	@for prog in $(PROGRAM_DIRS); do \
		echo "Cleaning $$prog..."; \
		cd $$prog && cargo clean && cd ..; \
	done

.PHONY: distclean
distclean:
	rm -f elfs/*

