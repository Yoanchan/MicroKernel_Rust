use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

pub mod page;

pub unsafe fn init(physical_memory_offset: x86_64::VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(
    physical_memory_offset: x86_64::VirtAddr,
) -> &'static mut x86_64::structures::paging::PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(x86_64::PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub fn print_l4_table(physical_memory_offset: x86_64::VirtAddr, mapper: OffsetPageTable) {
    let l4_table = unsafe { active_level_4_table(physical_memory_offset) };
    for (i, entry) in l4_table.iter().enumerate() {
        if !entry.is_unused() {
            info!("L4 Entry {} : {:?}", i, entry);
            // let phys = entry.frame().unwrap().start_address();
            // let virt = translate_physical_to_virtual(phys, physical_memory_offset);
            // let l3_table: &PageTable = unsafe { &*virt.as_mut_ptr() };

            // for (i, entry) in l3_table.iter().enumerate() {
            //     if !entry.is_unused() {
            //         info!("L3 Entry {}: {:?}", i, entry);
            //     }
            // }
        }
    }

    // let addresses = [
    //     0xb8000,
    //     0x201008,
    //     0x0100_0020_1a10,
    //     physical_memory_offset.as_u64(),
    // ];
    // for &address in &addresses {
    //     let virt = VirtAddr::new(address);
    //     let phys = mapper.translate_addr(virt);
    //     info!("{:?} -> {:?}", virt, phys);
    // }
}

fn translate_physical_to_virtual(
    physical_address: PhysAddr,
    physical_memory_offset: x86_64::VirtAddr,
) -> VirtAddr {
    return VirtAddr::new(physical_address.as_u64() + physical_memory_offset.as_u64());
}
