all: x86_64 aarch64 riscv64gc clean package

x86_64:
	cargo build -Z build-std --target x86_64-unknown-linux-musl --target-dir . --release
aarch64:
	cargo build -Z build-std --target aarch64-unknown-linux-musl --target-dir . --release
riscv64gc:
	cargo build -Z build-std --target riscv64gc-unknown-linux-musl --target-dir . --release

clean:
	cargo clean

package:
	
	cd x86_64-unknown-linux-musl/release/ && tar czf minefetch-x86_64-unknown-linux-musl.tar.gz minefetch && mv minefetch-x86_64-unknown-linux-musl.tar.gz ../../build-cross/

	cd aarch64-unknown-linux-musl/release/ && tar czf minefetch-aarch64-unknown-linux-musl.tar.gz minefetch && mv minefetch-aarch64-unknown-linux-musl.tar.gz ../../build-cross/

	cd riscv64gc-unknown-linux-musl/release/ && tar czf minefetch-riscv64gc-unknown-linux-musl.tar.gz minefetch && mv minefetch-riscv64gc-unknown-linux-musl.tar.gz ../../build-cross/