
check:
	cargo c
	cargo fmt -- --check
	cargo clean -p humio
	cargo clippy
	cargo t