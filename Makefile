.PHONY: all compile

all: compile

compile:
	cd e-rewriter && cargo build --release
	cd flussab && cargo build --release
	cd extract_or_replace && cargo build --release
	cd extraction-gym && cargo build --release
	cd process_json && cargo build --release
	cd abc && make -j$(nproc)
	cd abc_stable/abc && make -j$(nproc)