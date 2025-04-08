
@echo off

pushd user_programs

cargo build --message-format=short && cp target/tinyos_x64_user_target/debug/shell ../tiny_os/tinyos_kernel/src/shell

popd