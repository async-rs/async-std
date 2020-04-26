#!/bin/sh

wasm-pack test --chrome --headless -- --features unstable --test buf_writer
wasm-pack test --chrome --headless -- --features unstable --test channel
wasm-pack test --chrome --headless -- --features unstable --test condvar
wasm-pack test --chrome --headless -- --features unstable --test mutex
wasm-pack test --chrome --headless -- --features unstable --test rwlock
wasm-pack test --chrome --headless -- --features unstable --test stream
wasm-pack test --chrome --headless -- --features unstable --test task_local
wasm-pack test --chrome --headless -- --features unstable --test timeout
