# Reactive-Synth envelope generator

A WASM binary and a [AudioWorkletProcessor](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor) implementation for an envelope generator created for [Reactive-Synth](https://github.com/PatrickStephansen/reactive-synth).

It generates a linear 5-step [envelope](https://en.wikipedia.org/wiki/Envelope_(music)) output with values between 0 and 1.

## Initialization

Copy both files (`envelope-generator.js` and `reactive_synth_envelope_generator.wasm`) to your static assets folder (preferably at build-time) to use in a web project. Assuming they are both in a `worklets` folder ready to be served by your web server, the follow steps are needed to initialize and use an evelope generator instance in your audio graph.

1. Fetch the WASM binary and read it as an `ArrayBuffer`.

   ```js
   const envelopeGeneratorBinaryResponse = await fetch(
     "/worklets/reactive_synth_envelope_generator.wasm"
   );
   const envelopeGeneratorBinary =
     await envelopeGeneratorBinaryResponse.arrayBuffer();
   ```

1. Add the js module to your audio worklet context.

   ```js
   const audioContext = new AudioContext();
   // ...
   await audioContext.audioWorklet.addModule("/worklets/envelope-generator.js");
   ```

1. Create an `AudioWorkletNode` instance from the registered module. It's registered under the name: `reactive-synth-envelope-generator`.

   ```js
   const envelopeGeneratorNode = new AudioWorkletNode(
     audioContext,
     "reactive-synth-envelope-generator",
     {
       numberOfInputs: 0,
       numberOfOutputs: 1,
       channelCount: 1,
       channelCountMode: "explicit",
       outputChannelCount: [1],
       processorOptions: { sampleRate: audioContext.sampleRate },
     }
   );
   ```

1. Send the WASM binary to the envelope generator node through its message port. It will respond through the same port when it's ready to run.

   ```js
   envelopeGeneratorNode.port.postMessage({
     type: "wasm",
     wasmModule: envelopeGeneratorBinary,
   });
   envelopeGeneratorNode.port.start();
   ```

1. Wait for the envelope generator node to tell you it's ready through the message port.

   ```js
   await new Promise((resolve) =>
     envelopeGeneratorNode.port.addEventListener("message", function moduleReady(event) {
       if (
         event.data &&
         event.data.type === "module-ready" &&
         event.data.value
       ) {
         resolve();
         envelopeGeneratorNode.port.removeEventListener("message", moduleReady);
       }
     })
   );
   ```

The envelope generator is now ready to connect to your audio graph.

## Parameters

The `trigger` parameter is effectively the input for the module. It's a parameter rather than an actual input because it can be convenient to set the value to manually start and stop an envelope, or add a constant to the output of whatever other modules are connected. Any value above zero starts an envelope, and falling back to 0 or below starts the release phase then ends the envelope.

The envelope shape is controlled by the rest of the parameters: `attackValue`, `attackTime`, `holdTime`, `decayTime`, `sustainValue`, and `releaseTime`. Times are in seconds, and the values for attack and sustain must be between 0 and 1.

## Output

The envelope generator node only has one output value, which is between 0 and 1. Usually, you will want to plug it into a GainNode to scale it for whatever purpose you need. This can be duplicated over multiple channels if that somehow helps you by changing `outputChannelCount` on construction of the `AudioWorkletNode`.

## Message port events

Besides being used to send the WASM binary to the processor, the message port is also used to make visualizations and manual triggering possible.

The envelope generator can be manually triggered by sending messages with `type` "manual-trigger" and a `value` of `true` or `false`.

The port will send a message of `type` "trigger-change" with a `value` of `true` or `false` whenever the trigger state changes for any reason, be it an automated input or manual trigger. The value changes to `false` when the trigger is released (value falls to 0 or below), normally before the release step of the envelope is done. In [Reactive-Synth](https://github.com/PatrickStephansen/reactive-synth), this is used to blink a light on the trigger button.

Since the state changes at audio rates (usually 44.1kHz or 48kHz) and we want to visualize at video rates (usually 60Hz), the node doesn't just spam out state updates, you have to ask for each one (usually from a `requestAnimationFrame` call). Post a message with `type` "get-state", and it will post a message back with `type` "state" and a `state` field with this structure:

```ts
{
  stage: "rest" | "attack" | "hold" | "decay" | "sustain" | "release";
  stageProgress: number;
  outputValue: number;
  parameters: {
    attackValue: number;
    attackTime: number;
    holdTime: number;
    decayTime: number;
    sustainValue: number;
    releaseTime: number;
  }
}
```

