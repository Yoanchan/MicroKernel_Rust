use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::{interrupts::without_interrupts, port::Port},
    registers::control::{Cr2, Cr3},
    registers::rflags::{self, RFlags},
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

use crate::device::pic_8259::{MAIN, WORKER};

pub mod gdt;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[inline(always)]
pub fn disable() {
    unsafe {
        llvm_asm!("cli" : : : : "intel", "volatile");
    }
}

#[inline(always)]
pub fn enable() {
    unsafe {
        llvm_asm!("sti; nop" : : : : "intel", "volatile");
    }
}

#[inline(always)]
pub fn enable_and_hlt() {
    x86_64::instructions::interrupts::enable_and_hlt();
}

pub fn enabled() -> bool {
    rflags::read().contains(RFlags::INTERRUPT_FLAG)
}

pub fn disable_then_execute<F, T>(uninterrupted_fn: F) -> T
where
    F: FnOnce() -> T,
{
    let interrupts_enabled = enabled();
    if interrupts_enabled == true {
        disable();
    }

    let result: T = uninterrupted_fn();

    if interrupts_enabled == true {
        enable();
    }

    result
}

pub fn mask_then_restore<F, T>(uninterrupted_fn: F) -> T
where
    F: FnOnce() -> T,
{
    let saved_masks: (u8, u8) = mask();
    let result: T = uninterrupted_fn();
    restore_mask(saved_masks);
    result
}

pub fn mask() -> (u8, u8) {
    disable();

    unsafe {
        let saved_mask1 = MAIN.lock().data.read();
        let saved_mask2 = WORKER.lock().data.read();
        MAIN.lock().data.write(0xff);
        WORKER.lock().data.write(0xff);
        (saved_mask1, saved_mask2)
    }
}

pub fn clear_mask() {
    disable();
    unsafe {
        MAIN.lock().data.write(0);
        WORKER.lock().data.write(0);
    }

    enable();
}

pub fn restore_mask(saved_masks: (u8, u8)) {
    disable();

    let (saved_mask1, saved_mask2) = saved_masks;

    unsafe {
        MAIN.lock().data.write(saved_mask1);
        WORKER.lock().data.write(saved_mask2);
    }

    enable();
}

#[inline(always)]
pub unsafe fn halt() {
    llvm_asm!("hlt");
}

#[inline(always)]
pub fn pause() {
    unsafe {
        llvm_asm!("pause");
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Cascade,
    SerialPort2,
    SerialPort1,
    ParallelPort2,
    FloppyDisk,
    ParallelPort1,
    RealTimeClock,
    Acpi,
    Available1,
    Available2,
    Mouse,
    CoProcessor,
    PrimaryAta,
    SecondaryAta,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub fn init() {
    IDT.load();
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
            idt.divide_error.set_handler_fn(divide_error_handler);
            idt.debug.set_handler_fn(debug_handler);
            idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
            idt.breakpoint.set_handler_fn(breakpoint_handler);
            idt.overflow.set_handler_fn(overflow_handler);
            idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
            idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
            idt.device_not_available.set_handler_fn(device_not_available_handler);
            unsafe {
                idt.double_fault
                    .set_handler_fn(double_fault_handler)
                    .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            }
            idt.invalid_tss.set_handler_fn(invalid_tss_handler);
            idt.segment_not_present.set_handler_fn(segment_not_present_handler);
            idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
            // idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
            idt.page_fault.set_handler_fn(page_fault_handler);
            // idt.reserved_1.set_handler_fn();
            idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
            idt.alignment_check.set_handler_fn(alignment_check_handler);
            // idt.machine_check.set_handler_fn(machine_check_handler);
            idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
            idt.virtualization.set_handler_fn(virtualization_handler);
            // idt.reserved_2.set_handler_fn();
            idt.security_exception.set_handler_fn(security_exception_handler);
            // idt.reserved_3.set_handler_fn();
            idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
            idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
            /*
            idt[Cascade.as_usize()].set_handler_fn(_interrupt_handler);
            idt[SerialPort2.as_usize()].set_handler_fn(_interrupt_handler);
            idt[SerialPort1.as_usize()].set_handler_fn(_interrupt_handler);
            idt[ParallelPort2_3.as_usize()].set_handler_fn(_interrupt_handler);
            idt[FloppyDisk.as_usize()].set_handler_fn(_interrupt_handler);
            idt[ParallelPort1.as_usize()].set_handler_fn(_interrupt_handler);
            */
            // idt[InterruptIndex::RealTimeClock.as_usize()].set_handler_fn(real_time_clock_interrupt_handler);
            /*
            idt[Acpi.as_usize()].set_handler_fn(_interrupt_handler);
            idt[Available1.as_usize()].set_handler_fn(_interrupt_handler);
            idt[Available2.as_usize()].set_handler_fn(_interrupt_handler);
            idt[Mouse.as_usize()].set_handler_fn(_interrupt_handler);
            idt[CoProcessor.as_usize()].set_handler_fn(_interrupt_handler);
            idt[PrimaryAta.as_usize()].set_handler_fn(_interrupt_handler);
            idt[SecondaryAta.as_usize()].set_handler_fn(_interrupt_handler);
            */

        idt
    };
}

// CPU exceptions

extern "x86-interrupt" fn divide_error_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("div 0");
    panic!("DIVISION BY ZERO {:#?}", _stack_frame);
}

extern "x86-interrupt" fn debug_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("debug");
    panic!("DEBUG");
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("non maskable");
    panic!("Non maskable Stack Frame");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("overflow");
    panic!("OVERFLOW");
}

extern "x86-interrupt" fn bound_range_exceeded_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("bound range");
    panic!("BOUND RANGE EXCEEDED");
}

extern "x86-interrupt" fn invalid_opcode_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("opcode");
    panic!("INVALID OPCODE");
}

extern "x86-interrupt" fn device_not_available_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("device");
    panic!("DEVICE NOT AVAILABLE");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_tss_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("tss");
    panic!("INVALID TSS");
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("segment {:#?}", stack_frame);
    error!("error : {}", _error_code);
    panic!("SEGMENT NOT PRESENT {}", _error_code);
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("stack");
    panic!("STACK SEGMENT FAULT");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("Protection {}", _error_code);
    unsafe {
        let stack = _stack_frame.as_mut();
        println!("cs {} ss {}", stack.code_segment, stack.stack_segment);
        println!("ip : {}", stack.instruction_pointer.as_u64());
        println!("sp : {}", stack.stack_pointer.as_u64());
        println!("GENERAL PROTECTION FAULT! {:#?}", stack);
    }
    println!("TRIED TO READ : {:#?}", Cr2::read());
    println!("CR3 : {:#?}", Cr3::read());
    println!("ERROR : {:#?}", _error_code);
    shutdown();
}

extern "x86-interrupt" fn x87_floating_point_handler(_stack_frame: &mut InterruptStackFrame) {
    panic!("x87 FLOATING POINT ERROR");
}

extern "x86-interrupt" fn alignment_check_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("alignement");
    panic!("ALIGNMENT CHECK ERROR");
}

// extern "x86-interrupt" fn machine_check_handler(
//     _stack_frame: &mut InterruptStackFrame,
//     _error_code: u64,
// ) {
//     error!("machine");
//     panic!("MACHINE CHECK ERROR");
// }

extern "x86-interrupt" fn simd_floating_point_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("simd");
    panic!("SIMD FLOATING POINT ERROR");
}

extern "x86-interrupt" fn virtualization_handler(_stack_frame: &mut InterruptStackFrame) {
    error!("virtualization");
    panic!("VIRTUALIZATION ERROR");
}

extern "x86-interrupt" fn security_exception_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    error!("security");
    panic!("SECURITY EXCEPTION");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);

    x86_64::instructions::hlt()
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    // print!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8())
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    crate::device::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// extern "x86-interrupt" fn real_time_clock_interrupt_handler(
//     _stack_frame: &mut InterruptStackFrame,
// ) {
//     without_interrupts(|| unsafe {
//         Port::<u8>::new(0x70).write(0x0C);
//         Port::<u8>::new(0x71).read();
//     });

//     unsafe {
//         PICS.lock()
//             .notify_end_of_interrupt(InterruptIndex::RealTimeClock.as_u8())
//     }
// }

// pub fn change_rtc_interrupt_rate(mut rate: u8) -> u16 {
//     rate &= 0x0F;
//     without_interrupts(|| {
//         let mut address_port = Port::<u8>::new(0x70);
//         let mut data_port = Port::<u8>::new(0x71);

//         unsafe {
//             address_port.write(0x8A);
//             let prev: u8 = data_port.read();
//             address_port.write(0x8A);
//             data_port.write((prev & 0xF0) | rate);
//         }
//     });

//     32768 >> (rate - 1)
// }

// pub fn enable_rtc_interrupt() {
//     without_interrupts(|| {
//         let mut address_port = Port::<u8>::new(0x70);
//         let mut data_port = Port::<u8>::new(0x71);

//         unsafe {
//             address_port.write(0x8B);
//             let prev: u8 = data_port.read();
//             address_port.write(0x8B);
//             data_port.write(prev | 0x40);
//         }
//     });
// }

pub fn shutdown() -> ! {
    unsafe {
        warn!("Sending shutdown signal to QEMU.");
        let mut shutdown = Port::new(0x604);
        shutdown.write(0x2000_u16);
    }
    unsafe {
        asm!(
            "push rax",
            "push rbx",
            "push rcx",
            "push rsp",
            "mov ax, 0x1000",
            "mov ax, ss",
            "mov sp, 0xf000",
            "mov ax, 0x5307",
            "mov bx, 0x0001",
            "mov cx, 0x0003",
            "int 0x15",
            "pop rsp",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "call failed_shutdown",
            "ret",
            options(noreturn,),
        )
    }
}
