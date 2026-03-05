// Performance regression test for MULHSU instruction optimization

#[cfg(has_asm)]
use ckb_vm::cost_model::constant_cycles;
#[cfg(has_asm)]
use ckb_vm::machine::VERSION2;
#[cfg(has_asm)]
use ckb_vm::machine::asm::{AsmCoreMachine, AsmDefaultMachineBuilder, AsmMachine};
#[cfg(has_asm)]
use ckb_vm::{DefaultMachineRunner, ISA_IMC, SupportMachine};
#[cfg(has_asm)]
use std::fs;

#[test]
#[cfg(has_asm)]
fn test_mulhsu_microbench() {
    let buffer = fs::read("tests/programs/mulhsu_microbench").unwrap().into();

    // Warm-up run to populate caches
    {
        let asm_core = <AsmCoreMachine as SupportMachine>::new(ISA_IMC, VERSION2, u64::MAX);
        let core = AsmDefaultMachineBuilder::new(asm_core)
            .instruction_cycle_func(Box::new(constant_cycles))
            .build();
        let mut machine = AsmMachine::new(core);
        machine.load_program(&buffer, [].into_iter()).unwrap();
        let _ = machine.run();
    }

    // Timed runs
    let num_runs = 1000;
    let mut durations = Vec::with_capacity(num_runs);

    for _ in 0..num_runs {
        let asm_core = <AsmCoreMachine as SupportMachine>::new(ISA_IMC, VERSION2, u64::MAX);
        let core = AsmDefaultMachineBuilder::new(asm_core)
            .instruction_cycle_func(Box::new(constant_cycles))
            .build();
        let mut machine = AsmMachine::new(core);
        machine.load_program(&buffer, [].into_iter()).unwrap();

        let start = std::time::Instant::now();
        let result = machine.run();
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        durations.push(elapsed);
    }

    durations.sort();
    let average = durations.iter().sum::<std::time::Duration>() / (durations.len() as u32);
    let median = durations[num_runs / 2];
    let min = durations[0];
    let max = durations[num_runs - 1];

    println!(
        "MULHSU microbench: average={:?}, median={:?}, min={:?}, max={:?} ({} runs, 125k iterations x 8 mulhsu, negative RS1)",
        average, median, min, max, num_runs
    );
}
