// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// rust has a built-in for this but behind a feature flag
// use the native one if they get their shit together
fn clamp(min_value: f32, max_value: f32, value: f32) -> f32 {
	if value < min_value {
		return min_value;
	} else {
		if value > max_value {
			return max_value;
		} else {
			return value;
		}
	};
}

fn get_parameter(param: &Vec<f32>, min_value: f32, max_value: f32, index: usize) -> f32 {
	if param.len() > 1 {
		clamp(min_value, max_value, param[index])
	} else {
		if param.len() == 0 {
			clamp(min_value, max_value, 0.0)
		} else {
			clamp(min_value, max_value, param[0])
		}
	}
}

fn linear_interp(
	start_value: f32,
	mut start_time: f32,
	end_value: f32,
	end_time: f32,
	current_time: f32,
) -> f32 {
	if start_time >= end_time {
		start_time = end_time;
	}
	if current_time <= start_time {
		return start_value;
	}
	if current_time >= end_time {
		return end_value;
	}
	let gradient = (end_value - start_value) / (end_time - start_time);
	return start_value + (current_time - start_time) * gradient;
}

fn get_envelope_value(
	sample_rate: f32,
	input_gate_open: bool,
	attack_time: f32,
	attack_value: f32,
	hold_time: f32,
	decay_time: f32,
	sustain_value: f32,
	release_time: f32,
	mut seconds_on_stage: f32,
	mut envelope_stage: EnvelopeStage,
	mut value_on_trigger_change: f32,
) -> (EnvelopeStage, f32, f32, f32, f32) {
	let sample_time = 1.0 / sample_rate;
	let mut stage_progress = 0.0;
	// if this is returned ever then I've fucked up
	let mut output_value = -1.0;

	if envelope_stage == EnvelopeStage::Rest {
		if !input_gate_open {
			envelope_stage = EnvelopeStage::Rest;
			seconds_on_stage = seconds_on_stage + sample_time;
			stage_progress = 0.0;
			output_value = 0.0;
		} else {
			if sample_time < attack_time {
				envelope_stage = EnvelopeStage::Attack;
				seconds_on_stage = sample_time;
				stage_progress = seconds_on_stage / attack_time;
				value_on_trigger_change = 0.0;
				output_value = linear_interp(0.0, 0.0, attack_value, attack_time, seconds_on_stage);
			} else if sample_time - attack_time < hold_time {
				envelope_stage = EnvelopeStage::Hold;
				seconds_on_stage = sample_time - attack_time;
				stage_progress = seconds_on_stage / hold_time;
				output_value = attack_value;
			} else if sample_time - attack_time - hold_time < decay_time {
				envelope_stage = EnvelopeStage::Decay;
				seconds_on_stage = sample_time - attack_time - hold_time;
				stage_progress = seconds_on_stage / decay_time;
				output_value = linear_interp(
					attack_value,
					0.0,
					sustain_value,
					decay_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Sustain;
				seconds_on_stage = sample_time - attack_time - hold_time - decay_time;
				// loop the progress each second since this stage can last forever
				stage_progress = seconds_on_stage - seconds_on_stage.floor();
				output_value = sustain_value;
			}
		}
	}
	if envelope_stage == EnvelopeStage::Attack {
		if !input_gate_open {
			if sample_time < release_time {
				envelope_stage = EnvelopeStage::Release;
				if seconds_on_stage < attack_time {
					value_on_trigger_change = linear_interp(
						value_on_trigger_change,
						0.0,
						attack_value,
						attack_time,
						seconds_on_stage,
					);
				} else if seconds_on_stage - attack_time < hold_time {
					value_on_trigger_change = attack_value
				} else if seconds_on_stage - attack_time - hold_time < decay_time {
					value_on_trigger_change = linear_interp(
						attack_value,
						0.0,
						sustain_value,
						decay_time,
						seconds_on_stage - attack_time - hold_time,
					);
				} else {
					value_on_trigger_change = sustain_value;
				}
				seconds_on_stage = sample_time;
				stage_progress = sample_time / release_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				)
			} else {
				envelope_stage = EnvelopeStage::Rest;
				seconds_on_stage = sample_time - release_time;
				output_value = 0.0;
				stage_progress = 0.0;
			}
		} else {
			if seconds_on_stage + sample_time < attack_time {
				envelope_stage = EnvelopeStage::Attack;
				seconds_on_stage = seconds_on_stage + sample_time;
				stage_progress = seconds_on_stage / attack_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					attack_value,
					attack_time,
					seconds_on_stage,
				);
			} else if seconds_on_stage + sample_time - attack_time < hold_time {
				envelope_stage = EnvelopeStage::Hold;
				seconds_on_stage = seconds_on_stage + sample_time - attack_time;
				stage_progress = seconds_on_stage / hold_time;
				output_value = attack_value;
			} else if seconds_on_stage + sample_time - attack_time - hold_time < decay_time {
				envelope_stage = EnvelopeStage::Decay;
				seconds_on_stage = seconds_on_stage + sample_time - attack_time - hold_time;
				stage_progress = seconds_on_stage / decay_time;
				output_value = linear_interp(
					attack_value,
					0.0,
					sustain_value,
					decay_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Sustain;
				seconds_on_stage =
					seconds_on_stage + sample_time - attack_time - hold_time - decay_time;
				stage_progress = seconds_on_stage - seconds_on_stage.floor();
				output_value = sustain_value
			}
		}
	}
	if envelope_stage == EnvelopeStage::Hold {
		if !input_gate_open {
			if sample_time < release_time {
				envelope_stage = EnvelopeStage::Release;
				if seconds_on_stage < hold_time {
					value_on_trigger_change = attack_value;
				} else if seconds_on_stage - hold_time < decay_time {
					value_on_trigger_change = linear_interp(
						attack_value,
						0.0,
						sustain_value,
						decay_time,
						seconds_on_stage - hold_time,
					);
				} else {
					value_on_trigger_change = sustain_value;
				}
				seconds_on_stage = sample_time;
				stage_progress = seconds_on_stage / release_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Rest;
				seconds_on_stage = sample_time - release_time;
				output_value = 0.0;
				stage_progress = 0.0;
			}
		} else {
			if seconds_on_stage + sample_time < hold_time {
				envelope_stage = EnvelopeStage::Hold;
				seconds_on_stage = seconds_on_stage + sample_time;
				stage_progress = seconds_on_stage / hold_time;
				output_value = attack_value;
			} else if seconds_on_stage + sample_time - hold_time < decay_time {
				envelope_stage = EnvelopeStage::Decay;
				seconds_on_stage = seconds_on_stage + sample_time - hold_time;
				stage_progress = seconds_on_stage / decay_time;
				output_value = linear_interp(
					attack_value,
					0.0,
					sustain_value,
					decay_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Sustain;
				seconds_on_stage = seconds_on_stage + sample_time - hold_time - decay_time;
				output_value = sustain_value;
				stage_progress = seconds_on_stage - seconds_on_stage.floor();
			}
		}
	}
	if envelope_stage == EnvelopeStage::Decay {
		if !input_gate_open {
			if sample_time < release_time {
				envelope_stage = EnvelopeStage::Release;
				if seconds_on_stage < decay_time {
					value_on_trigger_change = linear_interp(
						attack_value,
						0.0,
						sustain_value,
						decay_time,
						seconds_on_stage,
					);
				} else {
					value_on_trigger_change = sustain_value;
				}
				seconds_on_stage = sample_time;
				stage_progress = seconds_on_stage / release_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Rest;
				seconds_on_stage = sample_time - release_time;
				output_value = 0.0;
				stage_progress = 0.0;
			}
		} else {
			if seconds_on_stage + sample_time < decay_time {
				envelope_stage = EnvelopeStage::Decay;
				seconds_on_stage = seconds_on_stage + sample_time;
				stage_progress = seconds_on_stage / decay_time;
				output_value = linear_interp(
					attack_value,
					0.0,
					sustain_value,
					decay_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Sustain;
				seconds_on_stage = sample_time - decay_time;
				output_value = sustain_value;
				stage_progress = seconds_on_stage - seconds_on_stage.floor();
			}
		}
	}
	if envelope_stage == EnvelopeStage::Sustain {
		if !input_gate_open {
			if sample_time < release_time {
				envelope_stage = EnvelopeStage::Release;
				seconds_on_stage = sample_time;
				stage_progress = seconds_on_stage / release_time;
				value_on_trigger_change = sustain_value;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Rest;
				seconds_on_stage = seconds_on_stage + sample_time - release_time;
				output_value = 0.0;
				stage_progress = 0.0;
			}
		} else {
			envelope_stage = EnvelopeStage::Sustain;
			seconds_on_stage = seconds_on_stage + sample_time;
			output_value = sustain_value;
			stage_progress = seconds_on_stage - seconds_on_stage.floor();
		}
	}
	if envelope_stage == EnvelopeStage::Release {
		if !input_gate_open {
			if seconds_on_stage + sample_time < release_time {
				envelope_stage = EnvelopeStage::Release;
				seconds_on_stage = seconds_on_stage + sample_time;
				stage_progress = seconds_on_stage / release_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
			} else {
				envelope_stage = EnvelopeStage::Rest;
				seconds_on_stage = seconds_on_stage + sample_time - release_time;
				output_value = 0.0;
				stage_progress = 0.0;
			}
		} else {
			if sample_time < attack_time {
				envelope_stage = EnvelopeStage::Attack;
				value_on_trigger_change = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
				seconds_on_stage = sample_time;
				stage_progress = seconds_on_stage / attack_time;
				output_value = linear_interp(
					value_on_trigger_change,
					0.0,
					0.0,
					release_time,
					seconds_on_stage,
				);
			} else if sample_time - attack_time < hold_time {
				envelope_stage = EnvelopeStage::Hold;
				seconds_on_stage = sample_time - attack_time;
				stage_progress = seconds_on_stage/hold_time;
				output_value = attack_value;
			}else if sample_time - attack_time-hold_time < decay_time{
				envelope_stage = EnvelopeStage::Decay;
				seconds_on_stage = sample_time - attack_time - hold_time;
				stage_progress = seconds_on_stage/decay_time;
				output_value = linear_interp(attack_value, 0.0, sustain_value, decay_time, seconds_on_stage);
			}else{
				envelope_stage = EnvelopeStage::Sustain;
				seconds_on_stage = sample_time -attack_time - hold_time - decay_time;
				output_value = sustain_value;
				stage_progress = seconds_on_stage - seconds_on_stage.floor();
			}
		}
	}

	(
		envelope_stage,
		seconds_on_stage,
		stage_progress,
		value_on_trigger_change,
		output_value,
	)
}

#[derive(Copy, Clone, PartialEq)]
#[repr(i32)]
pub enum InputGateStage {
	Opening = 1,
	Open = 2,
	Closing = 3,
	Closed = 4,
}
#[derive(Copy, Clone, PartialEq)]
#[repr(i32)]
pub enum EnvelopeStage {
	Rest = 0,
	Attack = 1,
	Hold = 2,
	Decay = 3,
	Sustain = 4,
	Release = 5,
}

pub struct EnvelopeGenerator {
	input_gate: Vec<f32>,
	attack_value: Vec<f32>,
	attack_time: Vec<f32>,
	hold_time: Vec<f32>,
	decay_time: Vec<f32>,
	sustain_value: Vec<f32>,
	release_time: Vec<f32>,
	render_quantum_samples: usize,
	sample_rate: f32,
	output: Vec<f32>,
	seconds_on_stage: f32,
	stage_progress: f32,
	envelope_stage: EnvelopeStage,
	input_gate_stage: InputGateStage,
	output_value: f32,
	value_on_trigger_change: f32,
}

impl EnvelopeGenerator {
	pub fn new(render_quantum_samples: usize, sample_rate: f32) -> EnvelopeGenerator {
		let mut output = Vec::with_capacity(render_quantum_samples);
		output.resize(render_quantum_samples, 0.0);
		EnvelopeGenerator {
			input_gate: Vec::with_capacity(render_quantum_samples),
			attack_value: Vec::with_capacity(render_quantum_samples),
			attack_time: Vec::with_capacity(render_quantum_samples),
			hold_time: Vec::with_capacity(render_quantum_samples),
			decay_time: Vec::with_capacity(render_quantum_samples),
			sustain_value: Vec::with_capacity(render_quantum_samples),
			release_time: Vec::with_capacity(render_quantum_samples),
			render_quantum_samples,
			sample_rate,
			output,
			seconds_on_stage: 0.0,
			stage_progress: 0.0,
			envelope_stage: EnvelopeStage::Rest,
			input_gate_stage: InputGateStage::Closed,
			output_value: 0.0,
			value_on_trigger_change: 0.0,
		}
	}

	pub fn process(
		&mut self,
		input_gate_changed: unsafe extern "C" fn(bool),
	) {
		for sample_index in 0..self.render_quantum_samples {
			let attack_time = get_parameter(&self.attack_time, 0.0, 10.0, sample_index);
			let attack_value = get_parameter(&self.attack_value, 0.0, 1.0, sample_index);
			let hold_time = get_parameter(&self.hold_time, 0.0, 10.0, sample_index);
			let decay_time = get_parameter(&self.decay_time, 0.0, 10.0, sample_index);
			let sustain_value = get_parameter(&self.sustain_value, 0.0, 1.0, sample_index);
			let release_time = get_parameter(&self.release_time, 0.0, 10.0, sample_index);
			let input_value = get_parameter(&self.input_gate, -1e9, 1e9, sample_index);
			if input_value > 0.0 {
				if self.input_gate_stage == InputGateStage::Closed
					|| self.input_gate_stage == InputGateStage::Closing
				{
					unsafe {
						input_gate_changed(true);
					}
					self.input_gate_stage = InputGateStage::Opening;
				} else {
					self.input_gate_stage = InputGateStage::Open;
				}
			} else {
				if self.input_gate_stage == InputGateStage::Opening
					|| self.input_gate_stage == InputGateStage::Open
				{
					unsafe {
						input_gate_changed(false);
					}
					self.input_gate_stage = InputGateStage::Closing;
				} else {
					self.input_gate_stage = InputGateStage::Closed;
				}
			}
			let (
				envelope_stage,
				seconds_on_stage,
				stage_progress,
				value_on_trigger_change,
				output_value,
			) = get_envelope_value(
				self.sample_rate,
				self.input_gate_stage == InputGateStage::Open
					|| self.input_gate_stage == InputGateStage::Opening,
				attack_time,
				attack_value,
				hold_time,
				decay_time,
				sustain_value,
				release_time,
				self.seconds_on_stage,
				self.envelope_stage,
				self.value_on_trigger_change,
			);
			self.envelope_stage = envelope_stage;
			self.seconds_on_stage = seconds_on_stage;
			self.stage_progress = stage_progress;
			self.value_on_trigger_change = value_on_trigger_change;
			self.output_value = output_value;
			self.output[sample_index] = output_value;
		}
	}
}

#[link(wasm_import_module = "imports")]
extern "C" {
	fn triggerChange(active: bool);
	fn shareState(
		stage: i32,
		stage_progress: f32,
		output_value: f32,
		attack_value: f32,
		attack_time: f32,
		hold_time: f32,
		decay_time: f32,
		sustain_value: f32,
		release_time: f32,
	);
}

#[no_mangle]
pub unsafe extern "C" fn get_input_gate_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).input_gate.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_attack_value_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).attack_value.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_attack_time_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).attack_time.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_hold_time_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).hold_time.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_decay_time_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).decay_time.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_sustain_value_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).sustain_value.as_mut_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn get_release_time_ptr(me: *mut EnvelopeGenerator) -> *mut f32 {
	(*me).release_time.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn init(
	render_quantum_samples: i32,
	sample_rate: f32,
) -> *mut EnvelopeGenerator {
	Box::into_raw(Box::new(EnvelopeGenerator::new(
		render_quantum_samples as usize,
		sample_rate,
	)))
}

#[no_mangle]
pub unsafe extern "C" fn process_quantum(
	me: *mut EnvelopeGenerator,
	input_gate_length: usize,
	attack_value_length: usize,
	attack_time_length: usize,
	hold_time_length: usize,
	decay_time_length: usize,
	sustain_value_length: usize,
	release_time_length: usize,
) -> *const f32 {
	// the expectation is that the parameters are copied directly into memory before this is called
	// so fix the length if it changed
	(*me).input_gate.set_len(input_gate_length);
	(*me).attack_value.set_len(attack_value_length);
	(*me).attack_time.set_len(attack_time_length);
	(*me).hold_time.set_len(hold_time_length);
	(*me).decay_time.set_len(decay_time_length);
	(*me).sustain_value.set_len(sustain_value_length);
	(*me).release_time.set_len(release_time_length);
	(*me).process(triggerChange);
	(*me).output.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn publish_state(me: *mut EnvelopeGenerator) {
	shareState(
		(*me).envelope_stage as i32,
		(*me).stage_progress,
		(*me).output_value,
		get_parameter(&(*me).attack_value, 0.0, 1.0, 0),
		get_parameter(&(*me).attack_time, 0.0, 10.0, 0),
		get_parameter(&(*me).hold_time, 0.0, 10.0, 0),
		get_parameter(&(*me).decay_time, 0.0, 10.0, 0),
		get_parameter(&(*me).sustain_value, 0.0, 1.0, 0),
		get_parameter(&(*me).release_time, 0.0, 10.0, 0),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	// TODO translate tests from js implementation
}
