# reactive-synth-envelope-generator
WASM implementation of an envelope generator audio processing node compatible with the web audio API. Created for [reactive-synth](https://github.com/PatrickStephansen/reactive-synth), but usable elsewhere if I ever document how.

The input is treated as a gate that opens when it rises above 0. The envelope has 5 active stages: attack, hold, decay, sustain and release. It moves through the first 3, which have fixed times, and settles on the 4th as the gate opens and is held open. It skips to the release stage when the gate closes. All curves are currently linear, but there are plans to parameterize each curve. Output is between 0 and 1.

## build

build command:

```bash
cargo build --features wee_alloc --release --target=wasm32-unknown-unknown && \
wasm-opt -Oz --strip-debug -o worklet/reactive_synth_envelope_generator.wasm \
target/wasm32-unknown-unknown/release/reactive_synth_envelope_generator.wasm
```
Inspect size with:

```bash
twiggy top -n 20 worklet/reactive_synth_envelope_generator.wasm
```

Run `npm link` from the worklet directory before trying to build the reactive-synth app (the dependent app not in this repo)

## test

`cargo test`
