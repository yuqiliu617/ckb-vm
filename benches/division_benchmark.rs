use criterion::{criterion_group, criterion_main, Criterion};

#[path = "vm_bench_utils.rs"]
mod vm_bench_utils;

#[cfg(has_asm)]
use ckb_vm::machine::VERSION2;
#[cfg(has_asm)]
use ckb_vm::ISA_IMC;

#[cfg(has_asm)]
fn bench_div(c: &mut Criterion) {
    let buffer = std::fs::read("tests/programs/div_microbench")
        .unwrap()
        .into();
    c.bench_function("div_microbench", |b| {
        b.iter(|| vm_bench_utils::run_program(&buffer, ISA_IMC, VERSION2))
    });
}

#[cfg(has_asm)]
fn bench_divw(c: &mut Criterion) {
    let buffer = std::fs::read("tests/programs/divw_microbench")
        .unwrap()
        .into();
    c.bench_function("divw_microbench", |b| {
        b.iter(|| vm_bench_utils::run_program(&buffer, ISA_IMC, VERSION2))
    });
}

#[cfg(has_asm)]
criterion_group!(benches, bench_div, bench_divw);
#[cfg(not(has_asm))]
fn noop(_: &mut Criterion) {}
#[cfg(not(has_asm))]
criterion_group!(benches, noop);
criterion_main!(benches);
