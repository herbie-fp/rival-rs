PKG_DIR := rival3-racket
PKG_NAME := rival3
FFI_MANIFEST := rival3-ffi/Cargo.toml

PLATFORM := $(shell racket -e "(display (path->string (system-library-subpath (not 1))))" | tr '\\\\' '/')
LIB_NAME := $(shell racket -e "(display (string-append (if (eq? (system-type) 'windows) \"rival3_ffi\" \"librival3_ffi\") (bytes->string/utf-8 (system-type 'so-suffix))))")

FFI_BUILD := rival3-ffi/target/release/$(LIB_NAME)
PKG_NATIVE_DIR := $(PKG_DIR)/private/native/$(PLATFORM)
PKG_NATIVE_LIB := $(PKG_NATIVE_DIR)/$(LIB_NAME)

.PHONY: build package install update uninstall clean nightly

build:
	cargo build --release --manifest-path=$(FFI_MANIFEST)
	mkdir -p $(PKG_NATIVE_DIR)
	cp $(FFI_BUILD) $(PKG_NATIVE_LIB)

package: build
	raco pkg create --format zip $(PKG_DIR)
	mv -f $(PKG_DIR).zip $(PKG_NAME).zip
	@if [ -f "$(PKG_DIR).zip.CHECKSUM" ]; then mv -f $(PKG_DIR).zip.CHECKSUM $(PKG_NAME).zip.CHECKSUM; fi

install: build
	@if raco pkg show --user $(PKG_NAME) | awk 'NR == 2 { print $$1 }' | grep -qx $(PKG_NAME); then \
		raco pkg update --user --batch --auto -D --type dir --link --name $(PKG_NAME) $(PKG_DIR); \
	else \
		raco pkg install --user --batch --auto -D --type dir --link --name $(PKG_NAME) $(PKG_DIR); \
	fi

update: build
	raco pkg update --user --batch --auto -D --type dir --link --name $(PKG_NAME) $(PKG_DIR)

uninstall:
	raco pkg remove --user --batch --auto --force --no-docs $(PKG_NAME)

clean:
	cargo clean
	cargo clean --manifest-path=$(FFI_MANIFEST)
	rm -rf $(PKG_DIR)/private/native
	rm -f $(PKG_NAME).zip $(PKG_NAME).zip.CHECKSUM
	rm -f $(PKG_DIR).zip $(PKG_DIR).zip.CHECKSUM

nightly:
	bash infra/nightly.sh
