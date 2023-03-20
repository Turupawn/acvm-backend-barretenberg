use barretenberg_wasm;
use common::acvm::{
    acir::circuit::Circuit, acir::native_types::Witness, FieldElement, PartialWitnessGenerator,
};
use js_sys::JsString;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn js_map_to_witness_map(js_map: js_sys::Map) -> BTreeMap<Witness, FieldElement> {
    let mut witness_skeleton: BTreeMap<Witness, FieldElement> = BTreeMap::new();
    for key_result in js_map.keys() {
        let key = key_result.expect("bad key");
        let idx;
        unsafe {
            idx = key
                .as_f64()
                .expect("not a number")
                .to_int_unchecked::<u32>();
        }
        let hex_str = js_map.get(&key).as_string().expect("not a string");
        let field_element = FieldElement::from_hex(&hex_str).expect("bad hex str");
        witness_skeleton.insert(Witness(idx), field_element);
    }
    witness_skeleton
}

fn witness_map_to_js_map(witness_map: BTreeMap<Witness, FieldElement>) -> js_sys::Map {
    let js_map = js_sys::Map::new();
    for (witness, field_value) in witness_map.iter() {
        let js_idx = js_sys::Number::from(witness.0);
        let mut hex_str = "0x".to_owned();
        hex_str.push_str(&field_value.to_hex());
        let js_hex_str = js_sys::JsString::from(hex_str);
        js_map.set(&js_idx, &js_hex_str);
    }
    js_map
}

fn read_circuit(circuit: js_sys::Uint8Array) -> Circuit {
    let circuit: Vec<u8> = circuit.to_vec();
    match Circuit::read(&*circuit) {
        Ok(circuit) => circuit,
        Err(err) => panic!("Circuit read err: {}", err),
    }
}

#[wasm_bindgen]
pub async fn solve_intermediate_witness(
    circuit: js_sys::Uint8Array,
    initial_witness: js_sys::Map,
    witness_loader: js_sys::Function,
) -> js_sys::Map {
    console_error_panic_hook::set_once();

    let circuit = read_circuit(circuit);
    let mut witness_skeleton = js_map_to_witness_map(initial_witness);

    use barretenberg_wasm::Plonk;
    let plonk = Plonk;
    // TODO: switch to `plonk.progress_solution` and then dispatch any async witness loads.
    match plonk.solve(&mut witness_skeleton, circuit.opcodes) {
        Ok(_) => {}
        Err(opcode) => panic!("solver came across an error with opcode {}", opcode),
    };

    // Example dumby call to witness_loader
    let this = JsValue::null();
    let descriptor = JsValue::from("some data please");
    let arb_load_future: wasm_bindgen_futures::JsFuture = witness_loader
        .call1(&this, &descriptor)
        .map(|js_value| js_sys::Promise::from(js_value))
        .expect("Not a promise")
        .into();
    match arb_load_future.await {
        Ok(_) => {
            // Don't care for now, just testing the await
        }
        Err(err) => {
            panic!("failed call of witness_loader: {}", JsString::from(err));
        }
    };

    witness_map_to_js_map(witness_skeleton)
}

#[wasm_bindgen]
pub fn intermediate_witness_to_assignment_bytes(
    intermediate_witness: js_sys::Map,
) -> js_sys::Uint8Array {
    console_error_panic_hook::set_once();

    let intermediate_witness = js_map_to_witness_map(intermediate_witness);

    // Add witnesses in the correct order
    // Note: The witnesses are sorted via their witness index
    // witness_values may not have all the witness indexes, e.g for unused witness which are not solved by the solver
    let num_witnesses = intermediate_witness.len();
    let mut sorted_witness = common::barretenberg_structures::Assignments::new();
    for i in 1..num_witnesses {
        let value = match intermediate_witness.get(&Witness(i as u32)) {
            Some(value) => *value,
            None => panic!("Missing witness element at idx {}", i),
        };

        sorted_witness.push(value);
    }

    let bytes = sorted_witness.to_bytes();
    js_sys::Uint8Array::from(&bytes[..])
}

#[wasm_bindgen]
pub fn acir_to_constraints_system(circuit: js_sys::Uint8Array) -> js_sys::Uint8Array {
    console_error_panic_hook::set_once();

    let circuit = read_circuit(circuit);
    let bytes = common::serializer::serialize_circuit(&circuit).to_bytes();
    js_sys::Uint8Array::from(&bytes[..])
}

#[wasm_bindgen]
pub fn public_input_length(circuit: js_sys::Uint8Array) -> js_sys::Number {
    console_error_panic_hook::set_once();

    let circuit = read_circuit(circuit);
    let length = circuit.public_inputs.0.len() as u32;
    js_sys::Number::from(length)
}

#[wasm_bindgen]
pub fn public_input_as_bytes(public_witness: js_sys::Map) -> js_sys::Uint8Array {
    console_error_panic_hook::set_once();

    let public_witness = js_map_to_witness_map(public_witness);
    let mut buffer = Vec::new();
    // Implicitly ordered by index
    for assignment in public_witness.values() {
        buffer.extend_from_slice(&assignment.to_be_bytes());
    }
    js_sys::Uint8Array::from(&buffer[..])
}
