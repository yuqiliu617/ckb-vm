use criterion::{criterion_group, criterion_main, Criterion};

#[path = "vm_bench_utils.rs"]
mod vm_bench_utils;

#[cfg(has_asm)]
use ckb_vm::machine::VERSION1;
#[cfg(has_asm)]
use ckb_vm::{ISA_B, ISA_IMC};

#[cfg(has_asm)]
fn bench_shadd(c: &mut Criterion) {
    let buffer = std::fs::read("tests/programs/shadd_microbench")
        .unwrap()
        .into();
    c.bench_function("shadd_microbench", |b| {
        b.iter(|| vm_bench_utils::run_program(&buffer, ISA_IMC | ISA_B, VERSION1))
    });
}

#[cfg(has_asm)]
criterion_group!(benches, bench_shadd);
#[cfg(not(has_asm))]
fn noop(_: &mut Criterion) {}
#[cfg(not(has_asm))]
criterion_group!(benches, noop);
criterion_main!(benches);
