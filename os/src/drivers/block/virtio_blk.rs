use super::BlockDevice;
use crate::mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use crate::sync::{UPSafeCell, Condvar};
use lazy_static::*;
use virtio_drivers::{VirtIOBlk, VirtIOHeader, BlkResp, RespStatus};
use crate::DEV_NON_BLOCKING_ACCESS;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock {
    virtio_blk: UPSafeCell<VirtIOBlk<'static>>,
    condvars: BTreeMap<u16, Condvar>,
}

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = unsafe { UPSafeCell::new(Vec::new()) };
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut blk = self.virtio_blk.exclusive_access();
            let mut resp = BlkResp::default();
            let token = unsafe {
                blk.read_block_nb(block_id, buf, &mut resp).unwrap()
            };
            drop(blk);
            self.condvars.get(&token).unwrap().wait();
            assert_eq!(resp.status(), RespStatus::Ok, "Error when reading VirtIOBlk");
        } else {
            self.virtio_blk
                .exclusive_access()
                .read_block(block_id, buf)
                .expect("Error when reading VirtIOBlk");
        }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let nb = *DEV_NON_BLOCKING_ACCESS.exclusive_access();
        if nb {
            let mut blk = self.virtio_blk.exclusive_access();
            let mut resp = BlkResp::default();
            let token = unsafe {
                blk.write_block_nb(block_id, buf, &mut resp).unwrap()
            };
            drop(blk);
            self.condvars.get(&token).unwrap().wait();
            assert_eq!(resp.status(), RespStatus::Ok, "Error when writing VirtIOBlk");
        } else {
            self.virtio_blk
                .exclusive_access()
                .write_block(block_id, buf)
                .expect("Error when writing VirtIOBlk");
        }
    }
    fn handle_irq(&self) {
        let mut blk = self.virtio_blk.exclusive_access(); 
        while let Ok(token) = blk.pop_used() {
            self.condvars.get(&token).unwrap().signal();
        }
    }
}

impl VirtIOBlock {
    pub fn new() -> Self {
        unsafe {
            let virtio_blk = UPSafeCell::new(
                VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            );
            let mut condvars = BTreeMap::new();
            let channels = virtio_blk.exclusive_access().virt_queue_size();
            for i in 0..channels {
                let condvar = Condvar::new(); 
                condvars.insert(i, condvar);
            }
            Self {
                virtio_blk,
                condvars,
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.exclusive_access().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(vaddr)
        .unwrap()
}
