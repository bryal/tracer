cargo build --release
perf record -g --call-graph=dwarf -F 1600 timeout 10 target/release/tracer
