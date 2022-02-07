pub const CLOCK_FREQ: usize = 12500000;

pub const MMIO: &[(usize, usize)] = &[
    (0x10001000, 0x1000),
    (0xC00_0000, 0x40_0000),
];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;

pub const VIRT_PLIC: usize = 0xC00_0000;

use crate::drivers::plic::{PLIC, IntrTargetPriority}; 

pub fn device_init() {
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine; 
    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);
    for intr_src_id in [1usize, 10] {
        plic.enable(hart_id, supervisor, intr_src_id); 
        plic.set_priority(intr_src_id, 1);
    }
    crate::println!("Hart0M threshold = {}", plic.get_threshold(hart_id, IntrTargetPriority::Machine));
    crate::println!("Hart0S threshold = {}", plic.get_threshold(hart_id, IntrTargetPriority::Supervisor));
    crate::println!("1 prio = {}", plic.get_priority(1));
    crate::println!("10 prio = {}", plic.get_priority(10));
    unsafe { sie::set_sext(); }
}

use crate::drivers::block::BLOCK_DEVICE;

pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match intr_src_id {
        1 => BLOCK_DEVICE.handle_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}
