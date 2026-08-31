#![allow(unused_imports)]

use verus_state_machines_macros::*;
use vstd::prelude::*;
use vstd::raw_ptr::*;
use vstd::*;
use vstd::layout::*;

use crate::types::{SegmentHeader, Page, PagePtr, SegmentPtr, todo, Heap, Tld};
use crate::tokens::{PageId, SegmentId, BlockId, HeapId, TldId};
use crate::config::*;

// Relationship between pointers and IDs

verus!{

pub uninterp spec fn is_page_ptr(ptr: *mut Page, page_id: PageId) -> bool;


pub uninterp spec fn is_segment_ptr(ptr: *mut SegmentHeader, segment_id: SegmentId) -> bool;


pub uninterp spec fn is_heap_ptr(ptr: *mut Heap, heap_id: HeapId) -> bool;


pub uninterp spec fn is_tld_ptr(ptr: *mut Tld, tld_id: TldId) -> bool;


pub uninterp spec fn segment_start(segment_id: SegmentId) -> int;

pub open spec fn page_header_start(page_id: PageId) -> int {
    segment_start(page_id.segment_id) + SIZEOF_SEGMENT_HEADER + page_id.idx * SIZEOF_PAGE_HEADER
}

pub open spec fn page_start(page_id: PageId) -> int {
    segment_start(page_id.segment_id) + SLICE_SIZE * page_id.idx
}

pub uninterp spec fn start_offset(block_size: int) -> int;

pub open spec fn block_start_at(page_id: PageId, block_size: int, block_idx: int) -> int {
    page_start(page_id)
         + start_offset(block_size)
         + block_idx * block_size
}

pub uninterp spec fn block_start(block_id: BlockId) -> int;

pub uninterp spec fn is_block_ptr(ptr: *mut u8, block_id: BlockId) -> bool;


pub uninterp spec fn is_block_ptr1(ptr: int, block_id: BlockId) -> bool;


pub uninterp spec fn is_page_ptr_opt(pptr: *mut Page, opt_page_id: Option<PageId>) -> bool;


#[verifier::external_body]
pub proof fn block_size_ge_word() { unimplemented!() }


#[verifier::external_body]
pub proof fn block_ptr_aligned_to_word() { unimplemented!() }


// Bit lemmas

/*proof fn bitmask_is_mod(t: usize)
    ensures (t & (((1usize << 26usize) - 1) as usize)) == (t % (1usize << 26usize)),
{
    //assert((t & (sub(1usize << 26usize, 1) as usize)) == (t % (1usize << 26usize)))
    //    by(bit_vector);
}*/

/*proof fn bitmask_is_rounded_down(t: usize)
    ensures (t & !(((1usize << 26usize) - 1) as usize)) == t - (t % (1usize << 26usize))
{
    assert((t & !(sub((1usize << 26usize), 1) as usize)) == sub(t, (t % (1usize << 26usize))))
        by(bit_vector);
    assert((1usize << 26usize) >= 1usize) by(bit_vector);
    assert(t >= (t % (1usize << 26usize))) by(bit_vector);
}*/

/*proof fn mod_removes_remainder(s: int, t: int, r: int)
    requires
        0 <= r < t,
        0 <= s,
    ensures (s*t + r) - ((s*t + r) % t) == s*t
{
    /*
    if s == 0 {
        assert(r % t == r) by(nonlinear_arith)
            requires 0 <= r < t;
    } else {
        let x = ((s-1)*t + r);
        assert((x % t) == (x + t) % t) by(nonlinear_arith);
    }
    */
    //assert(((s*t + r) % t) == r) by(nonlinear_arith)
    //  requires 0 <= r < t, 0 < t;

    //let x = s*t + r;
    //assert((x / t) * t + x % t == x) by(nonlinear_arith);
}*/

// Executable calculations

#[verifier::external_body]
pub fn calculate_segment_ptr_from_block(ptr: *mut u8, Ghost(block_id): Ghost<BlockId>) -> (res: *mut SegmentHeader)
{
    let block_p = ptr.addr();

    // Based on _mi_ptr_segment
    let segment_p = (block_p - 1) & (!((SEGMENT_SIZE - 1) as usize));

    /*proof {
        let s = block_id.page_id.segment_id.id;
        let t = SEGMENT_SIZE as int;
        let r = block_p - 1 - segment_start(block_id.page_id.segment_id);

        assert(block_p as int - 1 == s*t + r);
        assert(segment_p as int ==
            (block_p - 1) as int - ((block_p - 1) as int % SEGMENT_SIZE as int));
        assert(segment_p as int == (s*t + r) - ((s*t + r) % t));
    }*/

    ptr.with_addr(segment_p) as *mut SegmentHeader
}

/*
pub fn calculate_slice_idx_from_block(block_ptr: PPtr<u8>, segment_ptr: PPtr<SegmentHeader>, Ghost(block_id): Ghost<BlockId>) -> (slice_idx: usize)
    requires
        is_block_ptr(block_ptr.id(), block_id),
        is_segment_ptr(segment_ptr.id(), block_id.page_id.segment_id)
    ensures slice_idx as int == block_id.slice_idx,
{
    let block_p = block_ptr.addr();
    let segment_p = segment_ptr.addr();

    // Based on _mi_segment_page_of
    let diff = segment_p - block_p;
    diff >> (SLICE_SHIFT as usize)
}
*/

#[verifier::external_body]
pub fn calculate_slice_page_ptr_from_block(block_ptr: *mut u8, segment_ptr: *mut SegmentHeader, Ghost(block_id): Ghost<BlockId>) -> (page_ptr: *mut Page)
{
    let b = block_ptr.addr();
    let s = segment_ptr.addr();
    let q = (b - s) / SLICE_SIZE as usize;
    let h = s + SIZEOF_SEGMENT_HEADER + q * SIZEOF_PAGE_HEADER;
    block_ptr.with_addr(h) as *mut Page
}

#[inline(always)]
#[verifier::external_body]
pub fn calculate_page_ptr_subtract_offset(
    page_ptr: *mut Page, offset: u32, Ghost(page_id): Ghost<PageId>, Ghost(target_page_id): Ghost<PageId>) -> (result: *mut Page)
{

    let p = page_ptr.addr();
    let q = p - offset as usize;
    page_ptr.with_addr(q)
}

/*
pub fn calculate_page_ptr_add_offset(
    page_ptr: *mut Page, offset: u32, Ghost(page_id): Ghost<PageId>) -> (result: *mut Page)
    requires
        is_page_ptr(page_ptr as int, page_id),
        offset <= 0x1_0000,
    ensures
        is_page_ptr(result as int, PageId { idx: (page_id.idx + offset) as nat, ..page_id }),
{
    todo(); loop { }
}
*/

/*
pub fn calculate_segment_page_start(
    segment_ptr: SegmentPtr,
    page_ptr: PagePtr)
) -> (p: PPtr<u8>)
    ensures
        p as int == page_start(page_ptr.page_id)
{
}
*/

#[verifier::external_body]
pub fn calculate_page_start(page_ptr: PagePtr, block_size: usize) -> (addr: usize)
{
    let segment_ptr = SegmentPtr::ptr_segment(page_ptr);
    segment_page_start_from_slice(segment_ptr, page_ptr, block_size)
}

#[verifier::external_body]
pub fn calculate_page_block_at(
    page_start: usize,
    block_size: usize,
    idx: usize,
    Ghost(page_id): Ghost<PageId>
) -> (p: usize)
{
    let p = page_start + block_size * idx;
    return p;
}

#[verifier::external_body]
pub proof fn mk_segment_id(p: *mut SegmentHeader) -> (id: SegmentId) { unimplemented!() }


#[verifier::external_body]
pub fn segment_page_start_from_slice(
    segment_ptr: SegmentPtr,
    slice: PagePtr,
    xblock_size: usize)
  -> (res: usize) // start_offset
{

    let idxx = slice.page_ptr.addr() - (segment_ptr.segment_ptr.addr() + SIZEOF_SEGMENT_HEADER);
    let idx = idxx / SIZEOF_PAGE_HEADER;

    let start_offset = if xblock_size >= INTPTR_SIZE as usize && xblock_size <= 1024 {
        3 * MAX_ALIGN_GUARANTEE
    } else {
        0
    };

    segment_ptr.segment_ptr.addr() + (idx * SLICE_SIZE as usize) + start_offset
}

#[verifier::spinoff_prover]
#[inline]
#[verifier::external_body]
pub fn align_down(x: usize, y: usize) -> (res: usize)
{
    let mask = y - 1;

    if ((y & mask) == 0) { // power of two?
        x & !mask
    } else {
        (x / y) * y
    }
}

#[inline]
#[verifier::external_body]
pub fn align_up(x: usize, y: usize) -> (res: usize)
{
    let mask = y - 1;

    if ((y & mask) == 0) { // power of two?
        (x + mask) & !mask
    } else {
        ((x + mask) / y) * y
    }
}

impl SegmentPtr {
    #[inline]
#[verifier::external_body]
    pub fn ptr_segment(page_ptr: PagePtr) -> (segment_ptr: SegmentPtr)
    {

        let p = page_ptr.page_ptr.addr();
        let s = (p / SEGMENT_SIZE as usize) * SEGMENT_SIZE as usize;
        SegmentPtr {
            segment_ptr: page_ptr.page_ptr.with_addr(s) as *mut SegmentHeader,
            segment_id: Ghost(page_ptr.page_id@.segment_id),
        }
    }
}

#[verifier::external_body]
pub proof fn is_block_ptr_mult4(ptr: *mut u8, block_id: BlockId) { unimplemented!() }


#[verifier::external_body]
pub fn calculate_start_offset(block_size: usize) -> (res: u32)
{
    if block_size >= 8 && block_size <= 1024 {
        3 * MAX_ALIGN_GUARANTEE as u32
    } else {
        0
    }
}

}
