#[cfg(has_asm)]
use bytes::Bytes;
#[cfg(has_asm)]
use ckb_vm::cost_model::constant_cycles;
#[cfg(has_asm)]
use ckb_vm::machine::asm::{AsmCoreMachine, AsmDefaultMachineBuilder, AsmMachine};
#[cfg(has_asm)]
use ckb_vm::{DefaultMachineRunner, SupportMachine};

/// Creates a fresh ASM machine, loads `buffer`, runs it, and returns the exit code.
#[cfg(has_asm)]
pub fn run_program(buffer: &Bytes, isa: u8, version: u32) -> i8 {
    let asm_core = <AsmCoreMachine as SupportMachine>::new(isa, version, u64::MAX);
    let core = AsmDefaultMachineBuilder::new(asm_core)
        .instruction_cycle_func(Box::new(constant_cycles))
        .build();
    let mut machine = AsmMachine::new(core);
    machine.load_program(buffer, [].into_iter()).unwrap();
    machine.run().unwrap()
}
