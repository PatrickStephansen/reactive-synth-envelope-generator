const bytesPerMemorySlot = Float32Array.BYTES_PER_ELEMENT;
const renderQuantumSampleCount = 128;

const stageNames = {
	0: "rest",
	1: "attack",
	2: "hold",
	3: "decay",
	4: "sustain",
	5: "release",
};

registerProcessor(
	"reactive-synth-envelope-generator",
	class EnvelopeGenerator extends AudioWorkletProcessor {
		static get parameterDescriptors() {
			return [
				{
					name: "trigger",
					defaultValue: 0,
					automationRate: "a-rate",
				},
				{
					name: "attackValue",
					defaultValue: 1,
					minValue: 0,
					maxValue: 1,
					automationRate: "a-rate",
				},
				{
					name: "attackTime",
					defaultValue: 0.001,
					minValue: 0,
					maxValue: 10,
					automationRate: "a-rate",
				},
				{
					name: "holdTime",
					minValue: 0,
					defaultValue: 0.0625,
					maxValue: 10,
					automationRate: "a-rate",
				},
				{
					name: "decayTime",
					defaultValue: 0.125,
					minValue: 0,
					maxValue: 10,
					automationRate: "a-rate",
				},
				{
					name: "sustainValue",
					defaultValue: 0.25,
					minValue: 0,
					maxValue: 1,
					automationRate: "a-rate",
				},
				{
					name: "releaseTime",
					defaultValue: 0.25,
					minValue: 0,
					maxValue: 10,
				},
			];
		}
		constructor(options) {
			super(options);
			this.port.onmessage = this.handleMessage.bind(this);
			this.sampleRate = options.sampleRate || 44100;
			this.triggerChangeMessage = { type: "trigger-change", value: false };
			this.stateMessage = {
				type: "state",
				state: {
					stage: "rest",
					stageProgress: 0,
					outputValue: 0,
				},
			};
			this.manualTriggerOn = false;
			this.manualTriggerOnParameter = [1];
		}

		handleMessage(event) {
			if (event.data && event.data.type === "getState") {
				if (this.wasmModule){
					this.wasmModule.exports.publish_state(this.internalProcessorPtr);

				}
			}
			if (event.data && event.data.type === "manual-trigger") {
				this.manualTriggerOn = event.data.value;
			}
			if (event.data && event.data.type === "wasm") {
				this.initWasmModule(event.data.wasmModule).then(() =>
					this.port.postMessage({ type: "module-ready", value: true })
				);
			}
		}

		async initWasmModule(wasmModule) {
			this.wasmModule = await WebAssembly.instantiate(wasmModule, {
				imports: {
					triggerChange: (t) => {
						this.triggerChangeMessage.value = t;
						this.port.postMessage(this.triggerChangeMessage);
					},
					shareState: (
						stage,
						stageProgress,
						outputValue,
						attackValue,
						attackTime,
						holdTime,
						decayTime,
						sustainValue,
						releaseTime
					) => {
						this.stateMessage.state.stage = stageNames[stage];
						this.stateMessage.state.stageProgress = stageProgress;
						this.stateMessage.state.outputValue = outputValue;
						this.stateMessage.state.parameters = {
							attackValue,
							attackTime,
							holdTime,
							decayTime,
							sustainValue,
							releaseTime,
						};
						this.port.postMessage(this.stateMessage);
					},
				},
			});
			this.internalProcessorPtr = this.wasmModule.exports.init(
				renderQuantumSampleCount,
				this.sampleRate
			);
			this.float32WasmMemory = new Float32Array(
				this.wasmModule.exports.memory.buffer
			);
		}

		process(_inputs, outputs, parameters) {
			if (this.wasmModule) {
				this.float32WasmMemory.set(
					this.manualTriggerOn ? this.manualTriggerOnParameter : parameters.trigger,
					this.wasmModule.exports.get_input_gate_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.attackValue,
					this.wasmModule.exports.get_attack_value_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.attackTime,
					this.wasmModule.exports.get_attack_time_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.holdTime,
					this.wasmModule.exports.get_hold_time_ptr(this.internalProcessorPtr) /
						bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.decayTime,
					this.wasmModule.exports.get_decay_time_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.sustainValue,
					this.wasmModule.exports.get_sustain_value_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				this.float32WasmMemory.set(
					parameters.releaseTime,
					this.wasmModule.exports.get_release_time_ptr(
						this.internalProcessorPtr
					) / bytesPerMemorySlot
				);
				const outputPointer =
					this.wasmModule.exports.process_quantum(
						this.internalProcessorPtr,
						this.manualTriggerOn
							? this.manualTriggerOnParameter.length
							: parameters.trigger.length,
						parameters.attackValue.length,
						parameters.attackTime.length,
						parameters.holdTime.length,
						parameters.decayTime.length,
						parameters.sustainValue.length,
						parameters.releaseTime.length
					) / bytesPerMemorySlot;

				for (
					let channelIndex = 0;
					channelIndex < outputs[0].length;
					channelIndex++
				) {
					for (
						let sample = 0;
						sample < outputs[0][channelIndex].length;
						sample++
					) {
						outputs[0][channelIndex][sample] = this.float32WasmMemory[
							outputPointer + sample
						];
					}
				}
			}
			return true;
		}
	}
);
