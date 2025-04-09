
@echo off

pushd user_programs

cargo build --message-format=short && cp target/tinyos_x64_user_target/debug/shell ../tiny_os/tinyos_kernel/src/shell && cp target/tinyos_x64_user_target/debug/rec_fib ../tiny_os/tinyos_kernel/src/rec_fib

popd