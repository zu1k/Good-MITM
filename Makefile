NAME=good-mitm
BINDIR=bin
VERSION=$(shell git describe --tags || echo "unknown version")
UPX=upx --best
STRIP=llvm-strip -s
CROSS_BUILD=cross build --release --target

all: fmt clippy build

build:
	cargo build

clean:
	cargo clean

deps:
	cargo install cargo-strip xargo cross

a: fmt clippy

fmt:
	cargo fmt --all

fix:
	cargo fix

check:
	cargo check

clippy:
	cargo clippy

prepare: fmt check clippy fix

CROSS_TARGET_LIST = \
	x86_64-unknown-linux-musl \
	i686-unknown-linux-musl \
	aarch64-unknown-linux-musl \
	armv7-unknown-linux-musleabihf \

$(CROSS_TARGET_LIST):
	$(CROSS_BUILD) $@
	cp "target/$@/release/$(NAME)" "$(BINDIR)/$(NAME)-$@"
	$(STRIP) "$(BINDIR)/$(NAME)-$@"
	$(UPX) "$(BINDIR)/$(NAME)-$@"

windows:
	cargo build --target x86_64-pc-windows-gnu --release
	cp "target/x86_64-pc-windows-gnu/release/$(NAME).exe" "$(BINDIR)/$(NAME)-x86_64-pc-windows-gnu-$(VERSION).exe"
	$(STRIP) "$(BINDIR)/$(NAME)-x86_64-pc-windows-gnu-$(VERSION).exe"
	zip -q -m $(BINDIR)/$(NAME)-x86_64-pc-windows-gnu-$(VERSION).zip "$(BINDIR)/$(NAME)-x86_64-pc-windows-gnu-$(VERSION).exe"

bindir:
	rm -rf $(BINDIR)
	mkdir $(BINDIR)

bin_gz=$(addsuffix .gz, $(CROSS_TARGET_LIST))

$(bin_gz): %.gz : %
	chmod +x $(BINDIR)/$(NAME)-$(basename $@)
	gzip -f -S -$(VERSION).gz $(BINDIR)/$(NAME)-$(basename $@)

gz_release: $(bin_gz)

release: bindir gz_release windows
