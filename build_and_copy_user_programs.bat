
@echo off

pushd user_programs

cargo build && cp target/tinyos_x64_user_target/debug/simple_test ../tiny_os/tinyos_kernel/src/simple_test

popd