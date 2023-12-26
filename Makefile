install:
	cargo install --path daemon

check:
	# rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
	cargo check --all --all-targets --target x86_64-unknown-linux-gnu
	cargo check --all --all-targets --target x86_64-pc-windows-gnu
