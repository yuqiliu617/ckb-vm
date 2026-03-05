// Performance microbenchmark for CPOP/CPOPW in asm backend.

#[cfg(has_asm)]
use ckb_vm::cost_model::constant_cycles;
#[cfg(has_asm)]
use ckb_vm::machine::VERSION1;
#[cfg(has_asm)]
use ckb_vm::machine::asm::{AsmCoreMachine, AsmDefaultMachineBuilder, AsmMachine};
#[cfg(has_asm)]
use ckb_vm::{DefaultMachineRunner, ISA_B, ISA_IMC, SupportMachine};
#[cfg(has_asm)]
use std::fs;
#[cfg(has_asm)]
use std::path::Path;
#[cfg(has_asm)]
use std::process::Command;

#[cfg(has_asm)]
fn load_cpop_microbench_binary() -> Option<Vec<u8>> {
    let prebuilt = Path::new("tests/programs/cpop_microbench");
    if prebuilt.exists() {
        return fs::read(prebuilt).ok();
    }

    let source = Path::new("tests/programs/cpop_microbench.S");
    if !source.exists() {
        return None;
    }

    let output =
        std::env::temp_dir().join(format!("ckb_vm_cpop_microbench_{}", std::process::id()));
    let compilers = ["riscv64-unknown-elf-gcc", "riscv64-linux-gnu-gcc"];

    for compiler in compilers {
        let status = Command::new(compiler)
            .arg("-nostdlib")
            .arg("-static")
            .arg("-march=rv64imac_zbb")
            .arg("-mabi=lp64")
            .arg("-Wl,-e,_start")
            .arg("-o")
            .arg(&output)
            .arg(source)
            .status();

        if let Ok(status) = status {
            if status.success() {
                return fs::read(&output).ok();
            }
        }
    }

    None
}

#[test]
#[cfg(has_asm)]
fn test_cpop_microbench() {
    let Some(buffer) = load_cpop_microbench_binary() else {
        eprintln!(
            "Skipping cpop microbench: prebuilt tests/programs/cpop_microbench is missing and no supported RISC-V gcc toolchain was found."
        );
        return;
    };
    let buffer = buffer.into();

    // Warm-up run to populate caches.
    {
        let asm_core = <AsmCoreMachine as SupportMachine>::new(ISA_IMC | ISA_B, VERSION1, u64::MAX);
        let core = AsmDefaultMachineBuilder::new(asm_core)
            .instruction_cycle_func(Box::new(constant_cycles))
            .build();
        let mut machine = AsmMachine::new(core);
        machine.load_program(&buffer, [].into_iter()).unwrap();
        let _ = machine.run();
    }

    // Timed runs.
    let num_runs = 100;
    let mut durations = Vec::with_capacity(num_runs);

    for _ in 0..num_runs {
        let asm_core = <AsmCoreMachine as SupportMachine>::new(ISA_IMC | ISA_B, VERSION1, u64::MAX);
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
        "CPOP microbench: average={:?}, median={:?}, min={:?}, max={:?} ({} runs, 1M iterations x 8 cpop/cpopw ops)",
        average, median, min, max, num_runs
    );
}
