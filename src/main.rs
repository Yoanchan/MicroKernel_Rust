#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(default_alloc_error_handler)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(wake_trait)]
#![feature(naked_functions)]
#![feature(get_mut_unchecked)]

#[macro_use]
extern crate log;
extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use device::keyboard::print_keypresses;
use log::info;
use memory::BootInfoFrameAllocator;
use task::{
    scheduler::{priority::PriorityScheduler, Scheduler},
    PriorityTask,
};
use x86_64::VirtAddr;

mod logs;
#[macro_use]
mod serial;
#[macro_use]
mod vga_buffer;
mod allocators;
mod device;
mod interrupts;
mod memory;
mod task;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    info!("KERNEL STARTING...");
    log_init();
    memory_init(boot_info);
    interrupt_init();
    interrupts::clear_mask();
    let mut executor = PriorityScheduler::new();
    executor.spawn(PriorityTask::new(task::Priority::High, print_keypresses()));
    executor.spawn(PriorityTask::new(task::Priority::Low, task_1()));
    executor.spawn(PriorityTask::new(task::Priority::High, task_2()));
    executor.spawn(PriorityTask::new(task::Priority::High, task_3()));
    executor.run();
    hlt_loop()
}

fn log_init() {
    vga_buffer::WRITER.lock().clear_screen();
    logs::init().expect("LOGGER FAILED TO LAUNCH!");
    info!("Log Initialized!")
}

fn interrupt_init() {
    interrupts::gdt::init();
    interrupts::init();
    device::pic_8259::init();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    info!("Interrupt Initialized!")
}

fn memory_init(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocators::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");
    info!("Memory Manager Initialized!");
    // memory::print_l4_table(phys_mem_offset, mapper)
}

async fn task_1() {
    println!("Task 1")
}

async fn task_2() {
    println!("Task 2");
}

async fn task_3() {
    println!("Task 3")
}

fn breakpoint() {
    x86_64::instructions::interrupts::int3();
}

unsafe fn page_fault() {
    *(0xdeadbeef as *mut u64) = 42
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:#?}", info);
    loop {}
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
