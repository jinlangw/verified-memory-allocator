#![allow(unused_imports)]

use vstd::prelude::*;
use vstd::raw_ptr::*;
use vstd::modes::*;
use vstd::*;
use vstd::cell::pcell::*;
use vstd::cell::pcell;
use vstd::atomic_ghost::*;
use vstd::shared::Shared;
use verus_state_machines_macros::*;

use crate::config::*;
use crate::tokens::{Mim, PageId, ThreadId, SegmentId, HeapId, PageState, HeapState, SegmentState, TldId};
use crate::linked_list::{LL, ThreadLLSimple, ThreadLLWithDelayBits};
use crate::layout::{is_page_ptr, is_page_ptr_opt, is_heap_ptr, is_tld_ptr, block_start_at, is_segment_ptr};
use crate::page_organization::*;
use crate::os_mem::MemChunk;
use crate::commit_mask::CommitMask;
use crate::bin_sizes::{valid_bin_idx, size_of_bin, smallest_bin_fitting_size};
use crate::arena::{ArenaId, MemId};

verus!{

//// Page header data

#[repr(C)]
pub struct PageInner {
    pub flags0: u8,   // is_reset, is_committed, is_zero_init,

    pub capacity: u16,
    pub reserved: u16,

    pub flags1: u8,       // in_full, has_aligned
    pub flags2: u8,       // is_zero, retire_expire

    pub free: LL,

    // number of blocks that are allocated, or in `xthread_free`
    // In other words, this is the "complement" of the number
    // of blocks in `free` and `local_free`.
    pub used: u32,

    pub xblock_size: u32,
    pub local_free: LL,
}

impl PageInner {
    pub uninterp spec fn wf(&self, page_id: PageId, page_state: PageState, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn zeroed(&self) -> bool;


    pub uninterp spec fn zeroed_except_block_size(&self) -> bool;

}

tokenized_state_machine!{ BoolAgree {
    fields {
        #[sharding(variable)] pub x: bool,
        #[sharding(variable)] pub y: bool,
    }
    init!{
        initialize(b: bool) {
            init x = b;
            init y = b;
        }
    }
    transition!{
        set(b: bool) {
            assert(pre.x == pre.y);
            update x = b;
            update y = b;
        }
    }
    property!{
        agree() {
            assert(pre.x == pre.y);
        }
    }
    #[invariant]
    pub spec fn inv_eq(&self) -> bool {
        self.x == self.y
    }
    #[inductive(initialize)] fn initialize_inductive(post: Self, b: bool) { }
    #[inductive(set)] fn set_inductive(pre: Self, post: Self, b: bool) { }
}}

struct_with_invariants!{
    pub struct AtomicHeapPtr {
        pub atomic: AtomicPtr<Heap, _, (BoolAgree::y, Option<Mim::heap_of_page>), _>,

        pub instance: Ghost<Mim::Instance>,
        pub page_id: Ghost<PageId>,
        pub emp: Tracked<BoolAgree::x>,
        pub emp_inst: Tracked<BoolAgree::Instance>,
    }

    pub open spec fn wf(&self, instance: Mim::Instance, page_id: PageId) -> bool {
        predicate {
            self.instance == instance
            && self.page_id == page_id
            && self.emp@.instance_id() == self.emp_inst@.id()
        }
        invariant
            on atomic
            with (instance, page_id, emp, emp_inst)
            is (v: *mut Heap, all_g: (BoolAgree::y, Option<Mim::heap_of_page>))
        {
            let (is_emp, g_opt) = all_g;
            is_emp.instance_id() == emp_inst@.id()
            && (match g_opt {
                None => is_emp.value(),
                Some(g) => {
                    &&& !is_emp.value()
                    &&& g.instance_id() == instance@.id()
                    &&& g.key() == page_id
                    &&& is_heap_ptr(v, g.value())
                }
            })
        }
    }
}

impl AtomicHeapPtr {
    pub uninterp spec fn is_empty(&self) -> bool;


#[verifier::external_body]
    pub fn empty() -> (ahp: AtomicHeapPtr)
    {
        let tracked (Tracked(emp_inst), Tracked(emp_x), Tracked(emp_y)) = BoolAgree::Instance::initialize(true);
        let ghost g = (Ghost(arbitrary()), Ghost(arbitrary()), Tracked(emp_x), Tracked(emp_inst));
        AtomicHeapPtr {
            page_id: Ghost(arbitrary()),
            instance: Ghost(arbitrary()),
            emp: Tracked(emp_x),
            emp_inst: Tracked(emp_inst),
            atomic: AtomicPtr::new(Ghost(g), core::ptr::null_mut(), Tracked((emp_y, None))),
        }
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn disable(&mut self) -> (hop: Tracked<Mim::heap_of_page>)
    {
        let tracked mut heap_of_page;
        atomic_with_ghost!(
            &self.atomic => no_op();
            ghost g => {
                let tracked (mut y, heap_of_page_opt) = g;
                self.emp_inst.borrow().set(true, self.emp.borrow_mut(), &mut y);
                heap_of_page = heap_of_page_opt.tracked_unwrap();
                g = (y, None);
            }
        );
        Tracked(heap_of_page)
    }
}

#[repr(C)]
pub struct Page {
    pub count: PCell<u32>,
    pub offset: u32, // this value is read-only while the Page is shared

    pub inner: PCell<PageInner>,
    pub xthread_free: ThreadLLWithDelayBits,
    pub xheap: AtomicHeapPtr,
    pub prev: PCell<*mut Page>,
    pub next: PCell<*mut Page>,

    pub padding: usize,
}

pub tracked struct PageSharedAccess {
    pub tracked points_to: raw_ptr::PointsTo<Page>,
    pub tracked exposed: raw_ptr::IsExposed,
}

pub tracked struct PageLocalAccess {
    pub tracked count: pcell::PointsTo<u32>,
    pub tracked inner: pcell::PointsTo<PageInner>,
    pub tracked prev: pcell::PointsTo<*mut Page>,
    pub tracked next: pcell::PointsTo<*mut Page>,
}

pub tracked struct PageFullAccess {
    pub tracked s: PageSharedAccess,
    pub tracked l: PageLocalAccess,
}

impl Page {
    pub uninterp spec fn wf(&self, page_id: PageId, block_size: nat, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn wf_secondary(&self, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn wf_unused(&self, mim_instance: Mim::Instance) -> bool;

}

pub uninterp spec fn page_differ_only_in_offset(page1: Page, page2: Page) -> bool;


pub uninterp spec fn psa_differ_only_in_offset(psa1: PageSharedAccess, psa2: PageSharedAccess) -> bool;


impl PageSharedAccess {
    pub uninterp spec fn wf(&self, page_id: PageId, block_size: nat, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn wf_secondary(&self, page_id: PageId, block_size: nat, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn wf_unused(&self, page_id: PageId, mim_instance: Mim::Instance) -> bool;

}

pub uninterp spec fn wf_reserved(block_size: int, reserved: int, count: int) -> bool;


impl PageLocalAccess {
    pub uninterp spec fn wf(&self, page_id: PageId, page_state: PageState, mim_instance: Mim::Instance) -> bool;


    pub uninterp spec fn wf_unused(&self, page_id: PageId, shared_access: PageSharedAccess, popped: Popped, mim_instance: Mim::Instance) -> bool;

}

impl PageFullAccess {
    pub uninterp spec fn wf_empty_page_global(&self) -> bool;

}

/////////////////////////////////////////////
///////////////////////////////////////////// Segments
/////////////////////////////////////////////

#[derive(Clone, Copy)]
pub enum SegmentKind {
    Normal,
    Huge,
}

#[repr(C)]
pub struct SegmentHeaderMain {
    pub memid: usize,
    pub mem_is_pinned: bool,
    pub mem_is_large: bool,
    pub mem_is_committed: bool,
    pub mem_alignment: usize,
    pub mem_align_offset: usize,
    pub allow_decommit: bool,
    pub decommit_expire: i64,
    pub decommit_mask: CommitMask,
    pub commit_mask: CommitMask,
}

#[repr(C)]
pub struct SegmentHeaderMain2 {
    pub next: *mut SegmentHeader,
    pub abandoned: usize,
    pub abandoned_visits: usize,
    pub used: usize,
    pub cookie: usize,
    pub segment_slices: usize,
    pub segment_info_slices: usize,
    pub kind: SegmentKind,
    pub slice_entries: usize,
}

struct_with_invariants!{
    #[repr(C)]
    pub struct SegmentHeader {
        pub main: PCell<SegmentHeaderMain>,
        pub abandoned_next: usize, // TODO should be atomic
        pub main2: PCell<SegmentHeaderMain2>,

        // Note: thread_id is 0 if the segment is abandoned
        pub thread_id: AtomicU64<_, Mim::thread_of_segment, _>,

        pub instance: Ghost<Mim::Instance>,
        pub segment_id: Ghost<SegmentId>,
    }

    pub open spec fn wf(&self, instance: Mim::Instance, segment_id: SegmentId) -> bool {
        predicate {
            self.instance == instance
            && self.segment_id == segment_id
        }
        invariant
            on thread_id
            with (instance, segment_id)
            is (v: u64, g: Mim::thread_of_segment)
        {
            &&& g.instance_id() == instance@.id()
            &&& g.key() == segment_id
            &&& g.value() == ThreadId { thread_id: v }
        }
    }
}

pub tracked struct SegmentSharedAccess {
    pub points_to: raw_ptr::PointsTo<SegmentHeader>,
}

impl SegmentSharedAccess {
    pub uninterp spec fn wf(&self, segment_id: SegmentId, mim_instance: Mim::Instance) -> bool;

}

pub tracked struct SegmentLocalAccess {
    pub mem: MemChunk,
    pub main: pcell::PointsTo<SegmentHeaderMain>,
    pub main2: pcell::PointsTo<SegmentHeaderMain2>,
}

impl SegmentLocalAccess {
    pub uninterp spec fn wf(&self, segment_id: SegmentId, segment_state: SegmentState, mim_instance: Mim::Instance) -> bool;

}

/////////////////////////////////////////////
///////////////////////////////////////////// Heaps
/////////////////////////////////////////////

pub struct PageQueue {
    pub first: *mut Page,
    pub last: *mut Page,
    pub block_size: usize,
}

impl Clone for PageQueue {
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        PageQueue { first: self.first, last: self.last, block_size: self.block_size }
    }
}
impl Copy for PageQueue { }

#[repr(C)]
pub struct Heap {
    pub tld_ptr: TldPtr,

    pub pages_free_direct: PCell<[*mut Page; 129]>, // length PAGES_DIRECT
    pub pages: PCell<[PageQueue; 75]>, // length BIN_FULL + 1

    pub thread_delayed_free: ThreadLLSimple,
    pub thread_id: ThreadId,
    pub arena_id: ArenaId,
    //pub cookie: usize,
    //pub keys: usize,
    //pub random:
    pub page_count: PCell<usize>,
    pub page_retired_min: PCell<usize>,
    pub page_retired_max: PCell<usize>,
    //pub next: HeapPtr,
    pub no_reclaim: bool,

    // TODO should be a global, but right now we don't support pointers to globals
    pub page_empty_ptr: *mut Page,
}

pub struct HeapSharedAccess {
    pub points_to: raw_ptr::PointsTo<Heap>,
}

pub struct HeapLocalAccess {
    pub pages_free_direct: pcell::PointsTo<[*mut Page; 129]>,
    pub pages: pcell::PointsTo<[PageQueue; 75]>,
    pub page_count: pcell::PointsTo<usize>,
    pub page_retired_min: pcell::PointsTo<usize>,
    pub page_retired_max: pcell::PointsTo<usize>,
}

impl Heap {
    pub uninterp spec fn wf(&self, heap_id: HeapId, tld_id: TldId, mim_instance: InstanceId) -> bool;

}

impl HeapSharedAccess {
    pub uninterp spec fn wf(&self, heap_id: HeapId, tld_id: TldId, mim_instance: InstanceId) -> bool;


    pub uninterp spec fn wf2(&self, heap_id: HeapId, mim_instance: InstanceId) -> bool;

}

pub uninterp spec fn pages_free_direct_match(pfd_val: *mut Page, p_val: *mut Page, emp: *mut Page) -> bool;


pub uninterp spec fn pages_free_direct_is_correct(pfd: Seq<*mut Page>, pages: Seq<PageQueue>, emp: *mut Page) -> bool;


impl HeapLocalAccess {
    pub uninterp spec fn wf(&self, heap_id: HeapId, heap_state: HeapState, tld_id: TldId, mim_instance: InstanceId, emp: *mut Page) -> bool;


    pub uninterp spec fn wf_basic(&self, heap_id: HeapId, heap_state: HeapState, tld_id: TldId, mim_instance: InstanceId) -> bool;

}

/////////////////////////////////////////////
///////////////////////////////////////////// Thread local data
/////////////////////////////////////////////

//pub struct OsTld {
//    pub region_idx: usize,
//}

pub struct SegmentsTld {
    pub span_queue_headers: [SpanQueueHeader; 32], // len = SEGMENT_BIN_MAX + 1
    pub count: usize,
    pub peak_count: usize,
    pub current_size: usize,
    pub peak_size: usize,
}

pub struct SpanQueueHeader {
    pub first: *mut Page,
    pub last: *mut Page,
}

impl Clone for SpanQueueHeader {
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        SpanQueueHeader { first: self.first, last: self.last }
    }
}
impl Copy for SpanQueueHeader { }

pub struct Tld {
    // TODO mimalloc allows multiple heaps per thread
    pub heap_backing: *mut Heap,

    pub segments: SegmentsTld,
}

pub tracked struct Local {
    pub ghost thread_id: ThreadId,

    pub tracked my_inst: Mim::my_inst,
    pub tracked instance: Mim::Instance,
    pub tracked thread_token: Mim::thread_local_state,
    pub tracked checked_token: Mim::thread_checked_state,
    pub tracked is_thread: crate::thread::IsThread,

    pub ghost heap_id: HeapId,
    pub tracked heap: HeapLocalAccess,

    pub ghost tld_id: TldId,
    pub tracked tld: raw_ptr::PointsTo<Tld>,

    pub tracked segments: Map<SegmentId, SegmentLocalAccess>,

    // All pages, used and unused
    pub tracked pages: Map<PageId, PageLocalAccess>,
    pub ghost psa: Map<PageId, PageSharedAccess>,

    // All unused pages
    // (used pages are in the token system)
    pub tracked unused_pages: Map<PageId, PageSharedAccess>,

    pub ghost page_organization: PageOrg::State,

    pub tracked page_empty_global: Shared<PageFullAccess>,
}

pub uninterp spec fn common_preserves(l1: Local, l2: Local) -> bool;


impl Local {
    pub open(crate) spec fn inst(&self) -> Mim::Instance {
        self.instance
    }

    pub uninterp spec fn wf(&self) -> bool;


    pub uninterp spec fn wf_basic(&self) -> bool;


    pub uninterp spec fn wf_main(&self) -> bool;


    pub uninterp spec fn page_organization_valid(&self) -> bool;


    pub open spec fn page_state(&self, page_id: PageId) -> PageState
        recommends self.thread_token.value().pages.dom().contains(page_id)
    {
        self.thread_token.value().pages.index(page_id)
    }

    pub open spec fn page_inner(&self, page_id: PageId) -> PageInner
        recommends
            self.pages.dom().contains(page_id),
    {
        *self.pages.index(page_id).inner.value()
    }


    // This is for when we need to obtain ownership of the ThreadToken
    // but when we have a &mut reference to the Local

    #[verifier::external_body]
    pub proof fn take_thread_token(tracked &mut self) -> (tracked tt: Mim::thread_local_state)
    {
        unimplemented!();
    }

    #[verifier::external_body]
    pub proof fn take_checked_token(tracked &mut self) -> (tracked tt: Mim::thread_checked_state)
    {
        unimplemented!();
    }

    pub open spec fn commit_mask(&self, segment_id: SegmentId) -> CommitMask {
        self.segments[segment_id].main.value().commit_mask
    }

    pub open spec fn decommit_mask(&self, segment_id: SegmentId) -> CommitMask {
        self.segments[segment_id].main.value().decommit_mask
    }

    pub uninterp spec fn is_used_primary(&self, page_id: PageId) -> bool;


    pub open spec fn page_reserved(&self, page_id: PageId) -> int {
        self.pages[page_id].inner.value().reserved as int
    }

    pub open spec fn page_count(&self, page_id: PageId) -> int {
        self.pages[page_id].count.value() as int
    }

    pub open spec fn page_capacity(&self, page_id: PageId) -> int {
        self.pages[page_id].inner.value().capacity as int
    }

    pub open spec fn block_size(&self, page_id: PageId) -> int {
        self.pages[page_id].inner.value().xblock_size as int
    }
}

pub uninterp spec fn page_organization_queues_match(
    org_queues: Seq<DlistHeader>,
    queues: Seq<SpanQueueHeader>,
) -> bool;


pub uninterp spec fn page_organization_used_queues_match(
    org_queues: Seq<DlistHeader>,
    queues: Seq<PageQueue>,
) -> bool;



pub uninterp spec fn page_organization_pages_match(
    org_pages: Map<PageId, PageData>,
    pages: Map<PageId, PageLocalAccess>,
    psa: Map<PageId, PageSharedAccess>,
    popped: Popped,
) -> bool;


pub uninterp spec fn page_organization_pages_match_data(
    page_data: PageData,
    pla: PageLocalAccess,
    psa: PageSharedAccess,
    page_id: PageId,
    popped: Popped) -> bool;


pub uninterp spec fn page_organization_segments_match(
    org_segments: Map<SegmentId, SegmentData>,
    segments: Map<SegmentId, SegmentLocalAccess>,
) -> bool;


pub uninterp spec fn page_organization_matches_token_page(
    page_data: PageData,
    page_state: PageState) -> bool;



/////////////////////////////////////////////
/////////////////////////////////////////////
/////////////////////////////////////////////
/////////////////////////////////////////////
/////////////////////////////////////////////
/////////////////////////////////////////////
/////////////////////////////////////////////
////// Utilities for local access

pub struct HeapPtr {
    pub heap_ptr: *mut Heap,
    pub heap_id: Ghost<HeapId>,
}

impl Clone for HeapPtr {
    #[inline(always)]
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        HeapPtr { heap_ptr: self.heap_ptr, heap_id: Ghost(self.heap_id@) }
    }
}
impl Copy for HeapPtr { }

impl HeapPtr {
    pub uninterp spec fn wf(&self) -> bool;


    pub uninterp spec fn is_in(&self, local: Local) -> bool;


    #[inline(always)]
#[verifier::external_body]
    pub fn get_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (heap: &'a Heap)
    {
        let tracked perm = &local.instance.thread_local_state_guards_heap(
            local.thread_id, &local.thread_token).points_to;
        ptr_ref(self.heap_ptr, Tracked(perm))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_pages<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (pages: &'a [PageQueue; 75])
    {
        self.get_ref(Tracked(local)).pages.borrow(Tracked(&local.heap.pages))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_page_count<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page_count: usize)
    {
        *self.get_ref(Tracked(local)).page_count.borrow(Tracked(&local.heap.page_count))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn set_page_count<'a>(&self, Tracked(local): Tracked<&mut Local>, page_count: usize)
    {
        let tracked perm = &local.instance.thread_local_state_guards_heap(
            local.thread_id, &local.thread_token).points_to;
        let heap = ptr_ref(self.heap_ptr, Tracked(perm));
        *heap.page_count.borrow_mut(Tracked(&mut local.heap.page_count)) = page_count;
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_page_retired_min<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page_retired_min: usize)
    {
        *self.get_ref(Tracked(local)).page_retired_min.borrow(Tracked(&local.heap.page_retired_min))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn set_page_retired_min<'a>(&self, Tracked(local): Tracked<&mut Local>, page_retired_min: usize)
    {
        let tracked perm = &local.instance.thread_local_state_guards_heap(
            local.thread_id, &local.thread_token).points_to;
        let heap = ptr_ref(self.heap_ptr, Tracked(perm));
        *heap.page_retired_min.borrow_mut(Tracked(&mut local.heap.page_retired_min)) = page_retired_min;
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_page_retired_max<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page_retired_max: usize)
    {
        *self.get_ref(Tracked(local)).page_retired_max.borrow(Tracked(&local.heap.page_retired_max))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn set_page_retired_max<'a>(&self, Tracked(local): Tracked<&mut Local>, page_retired_max: usize)
    {
        let tracked perm = &local.instance.thread_local_state_guards_heap(
            local.thread_id, &local.thread_token).points_to;
        let heap = ptr_ref(self.heap_ptr, Tracked(perm));
        *heap.page_retired_max.borrow_mut(Tracked(&mut local.heap.page_retired_max)) = page_retired_max;
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_pages_free_direct<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (pages: &'a [*mut Page; 129])
    {
        self.get_ref(Tracked(local)).pages_free_direct.borrow(Tracked(&local.heap.pages_free_direct))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_arena_id<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (arena_id: ArenaId)
    {
        self.get_ref(Tracked(local)).arena_id
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_page_empty(&self, Tracked(local): Tracked<&Local>)
        -> (res: (*mut Page, Tracked<Shared<PageFullAccess>>))
    {
        let page_ptr = self.get_ref(Tracked(local)).page_empty_ptr;
        let tracked pfa = local.page_empty_global.clone();
        (page_ptr, Tracked(pfa))
    }
}

pub uninterp spec fn local_page_count_update(loc1: Local, loc2: Local) -> bool;


pub uninterp spec fn local_page_retired_min_update(loc1: Local, loc2: Local) -> bool;


pub uninterp spec fn local_page_retired_max_update(loc1: Local, loc2: Local) -> bool;




pub struct TldPtr {
    pub tld_ptr: *mut Tld,
    pub tld_id: Ghost<TldId>,
}

impl Clone for TldPtr {
    #[inline(always)]
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        TldPtr { tld_ptr: self.tld_ptr, tld_id: Ghost(self.tld_id@) }
    }
}
impl Copy for TldPtr { }


impl TldPtr {
    pub uninterp spec fn wf(&self) -> bool;


    pub uninterp spec fn is_in(&self, local: Local) -> bool;


    #[inline(always)]
#[verifier::external_body]
    pub fn get_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (tld: &'a Tld)
    {
        ptr_ref(self.tld_ptr, Tracked(&local.tld))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_mut<'a>(&self, Tracked(local): Tracked<&'a mut Local>) -> (tld: &'a mut Tld)
    {
        ptr_mut_ref(self.tld_ptr, Tracked(&mut local.tld))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_segments_count(&self, Tracked(local): Tracked<&Local>) -> (count: usize)
    {
        self.get_ref(Tracked(local)).segments.count
    }
}

pub struct SegmentPtr {
    pub segment_ptr: *mut SegmentHeader,
    pub segment_id: Ghost<SegmentId>,
}

impl Clone for SegmentPtr {
    #[inline(always)]
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        SegmentPtr { segment_ptr: self.segment_ptr, segment_id: Ghost(self.segment_id@) }
    }
}
impl Copy for SegmentPtr { }

impl SegmentPtr {
    pub uninterp spec fn wf(&self) -> bool;


    pub uninterp spec fn is_in(&self, local: Local) -> bool;


    #[inline(always)]
#[verifier::external_body]
    pub fn is_null(&self) -> (b: bool)
    {
        self.segment_ptr.addr() == 0
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn null() -> (s: Self)
    {
        SegmentPtr { segment_ptr: core::ptr::null_mut(),
            segment_id: Ghost(arbitrary())
        }
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_page_header_ptr(&self, idx: usize) -> (page_ptr: PagePtr)
    {
        let j = self.segment_ptr.addr() + SIZEOF_SEGMENT_HEADER + idx * SIZEOF_PAGE_HEADER;
        return PagePtr {
            page_ptr: self.segment_ptr.with_addr(j) as *mut Page,
            page_id: Ghost(PageId { segment_id: self.segment_id@, idx: idx as nat }),
        };
    }

    #[inline]
#[verifier::external_body]
    pub fn get_page_after_end(&self) -> (page_ptr: *mut Page)
    {
        let j = self.segment_ptr.addr()
          + SIZEOF_SEGMENT_HEADER
          + SLICES_PER_SEGMENT as usize * SIZEOF_PAGE_HEADER;
        self.segment_ptr.with_addr(j) as *mut Page
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (segment: &'a SegmentHeader)
    {
        let tracked perm =
            &local.instance.thread_local_state_guards_segment(
                local.thread_id, self.segment_id@, &local.thread_token).points_to;
        ptr_ref(self.segment_ptr, Tracked(perm))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_main_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (segment_header_main: &'a SegmentHeaderMain)
    {
        let segment = self.get_ref(Tracked(local));
        segment.main.borrow(Tracked(&local.segments.tracked_borrow(self.segment_id@).main))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_main2_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (segment_header_main2: &'a SegmentHeaderMain2)
    {
        let segment = self.get_ref(Tracked(local));
        segment.main2.borrow(Tracked(&local.segments.tracked_borrow(self.segment_id@).main2))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_commit_mask<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (cm: &'a CommitMask)
    {
        &self.get_main_ref(Tracked(local)).commit_mask
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_decommit_mask<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (cm: &'a CommitMask)
    {
        &self.get_main_ref(Tracked(local)).decommit_mask
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_decommit_expire(&self, Tracked(local): Tracked<&Local>) -> (i: i64)
    {
        self.get_main_ref(Tracked(local)).decommit_expire
    }


    #[inline(always)]
#[verifier::external_body]
    pub fn get_allow_decommit(&self, Tracked(local): Tracked<&Local>) -> (b: bool)
    {
        self.get_main_ref(Tracked(local)).allow_decommit
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_used(&self, Tracked(local): Tracked<&Local>) -> (used: usize)
    {
        self.get_main2_ref(Tracked(local)).used
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_abandoned(&self, Tracked(local): Tracked<&Local>) -> (abandoned: usize)
    {
        self.get_main2_ref(Tracked(local)).abandoned
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_mem_is_pinned(&self, Tracked(local): Tracked<&Local>) -> (mem_is_pinned: bool)
    {
        self.get_main_ref(Tracked(local)).mem_is_pinned
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn is_abandoned(&self, Tracked(local): Tracked<&Local>) -> (is_ab: bool)
    {
        self.get_ref(Tracked(local)).thread_id.load() == 0
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_segment_kind(&self, Tracked(local): Tracked<&Local>) -> (kind: SegmentKind)
    {
        self.get_main2_ref(Tracked(local)).kind
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn is_kind_huge(&self, Tracked(local): Tracked<&Local>) -> (b: bool)
    {
        let kind = self.get_main2_ref(Tracked(local)).kind;
        matches!(kind, SegmentKind::Huge)
    }
}

pub struct PagePtr {
    pub page_ptr: *mut Page,
    pub page_id: Ghost<PageId>,
}

impl Clone for PagePtr {
    #[inline(always)]
#[verifier::external_body]
    fn clone(&self) -> (s: Self)
    {
        PagePtr { page_ptr: self.page_ptr, page_id: Ghost(self.page_id@) }
    }
}
impl Copy for PagePtr { }

impl PagePtr {
    pub uninterp spec fn wf(&self) -> bool;


    pub uninterp spec fn is_in(&self, local: Local) -> bool;


    pub uninterp spec fn is_empty_global(&self, local: Local) -> bool;


    pub uninterp spec fn is_used_and_primary(&self, local: Local) -> bool;


    pub uninterp spec fn is_in_unused(&self, local: Local) -> bool;


    pub uninterp spec fn is_used(&self, local: Local) -> bool;


    #[inline(always)]
#[verifier::external_body]
    pub fn null() -> (s: Self)
    {
        PagePtr { page_ptr: core::ptr::null_mut(),
            page_id: Ghost(arbitrary())
        }
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn is_null(&self) -> (b: bool)
    {
        self.page_ptr.addr() == 0
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page: &'a Page)
    {
        let tracked perm = if self.is_in_unused(*local) {
            &local.unused_pages.tracked_borrow(self.page_id@).points_to
        } else {
            &local.instance.thread_local_state_guards_page(
                local.thread_id, self.page_id@, &local.thread_token).points_to
        };

        ptr_ref(self.page_ptr, Tracked(perm))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_inner_ref<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page_inner: &'a PageInner)
    {
        let page = self.get_ref(Tracked(local));
        page.inner.borrow(Tracked(
            &local.pages.tracked_borrow(self.page_id@).inner
            ))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_inner_ref_maybe_empty<'a>(&self, Tracked(local): Tracked<&'a Local>) -> (page_inner: &'a PageInner)
    {
        let tracked perm = if self.is_empty_global(*local) {
            &local.page_empty_global.borrow().s.points_to
        } else if self.is_in_unused(*local) {
            &local.unused_pages.tracked_borrow(self.page_id@).points_to
        } else {
            &local.instance.thread_local_state_guards_page(
                local.thread_id, self.page_id@, &local.thread_token).points_to
        };
        let page = ptr_ref(self.page_ptr, Tracked(perm));
        page.inner.borrow(Tracked(
            if self.is_empty_global(*local) {
                &local.page_empty_global.borrow().l.inner
            } else {
                &local.pages.tracked_borrow(self.page_id@).inner
            }
            ))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_count<'a>(&self, Tracked(local): Tracked<&Local>) -> (count: u32)
    {
        let page = self.get_ref(Tracked(local));
        *page.count.borrow(Tracked(
            &local.pages.tracked_borrow(self.page_id@).count
            ))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_next<'a>(&self, Tracked(local): Tracked<&Local>) -> (next: *mut Page)
    {
        let page = self.get_ref(Tracked(local));
        *page.next.borrow(Tracked(
            &local.pages.tracked_borrow(self.page_id@).next
            ))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_prev<'a>(&self, Tracked(local): Tracked<&Local>) -> (prev: *mut Page)
    {
        let page = self.get_ref(Tracked(local));
        *page.prev.borrow(Tracked(
            &local.pages.tracked_borrow(self.page_id@).prev
            ))
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn add_offset(&self, count: usize) -> (p: Self)
    {
        let p = self.page_ptr.addr();
        let q = p + count * SIZEOF_PAGE_HEADER;
        PagePtr {
            page_ptr: self.page_ptr.with_addr(q),
            page_id: Ghost(PageId {
                segment_id: self.page_id@.segment_id,
                idx: (self.page_id@.idx + count) as nat,
            })
        }
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn sub_offset(&self, count: usize) -> (p: Self)
    {
        let p = self.page_ptr.addr();
        let q = p - count * SIZEOF_PAGE_HEADER;
        let ghost page_id = PageId {
                segment_id: self.page_id@.segment_id,
                idx: (self.page_id@.idx - count) as nat,
            };
        let q = self.page_ptr.with_addr(q);
        PagePtr {
            page_ptr: q,
            page_id: Ghost(page_id)
        }
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn is_gt_0th_slice(&self, segment: SegmentPtr) -> (res: bool)
    {
        self.page_ptr.addr() > segment.get_page_header_ptr(0).page_ptr.addr()
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_index(&self) -> (idx: usize)
    {
        let segment = SegmentPtr::ptr_segment(*self);
        (self.page_ptr.addr() - segment.segment_ptr.addr() - SIZEOF_SEGMENT_HEADER)
            / SIZEOF_PAGE_HEADER
    }

#[verifier::external_body]
    pub fn slice_start(&self) -> (p: usize)
    {
        let segment = SegmentPtr::ptr_segment(*self);
        let s = segment.segment_ptr.addr();
        s +
          ((self.page_ptr.addr() - s - SIZEOF_SEGMENT_HEADER) / SIZEOF_PAGE_HEADER)
            * SLICE_SIZE as usize
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn add_offset_and_check(&self, count: usize, segment: SegmentPtr) -> (res: (Self, bool))
    {
        let p = self.page_ptr.addr();
        let q = p + count * SIZEOF_PAGE_HEADER;
        let page_ptr = PagePtr {
            page_ptr: self.page_ptr.with_addr(q),
            page_id: Ghost(PageId {
                segment_id: self.page_id@.segment_id,
                idx: (self.page_id@.idx + count) as nat,
            })
        };
        let last = segment.get_page_after_end();
        (page_ptr, page_ptr.page_ptr.addr() < last.addr())
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_block_size(&self, Tracked(local): Tracked<&Local>) -> (bsize: u32)
    {
        self.get_inner_ref(Tracked(local)).xblock_size
    }

    #[inline(always)]
#[verifier::external_body]
    pub fn get_heap(&self, Tracked(local): Tracked<&Local>) -> (heap: HeapPtr)
    {
        let page_ref = self.get_ref(Tracked(&*local));
        let h = atomic_with_ghost!(
            &page_ref.xheap.atomic => load();
            ghost g => {
                page_ref.xheap.emp_inst.borrow().agree(page_ref.xheap.emp.borrow(), &g.0);
                let tracked heap_of_page = g.1.tracked_borrow();
                local.instance.heap_of_page_agree_with_thread_state(
                    self.page_id@,
                    local.thread_id,
                    &local.thread_token,
                    heap_of_page);
            }
        );
        HeapPtr {
            heap_ptr: h,
            heap_id: Ghost(local.heap_id),
        }
    }
}

// Use macro as a work-arounds for not supporting functions that return &mut
// (note: not necessary anymore now that &mut is supported)

#[macro_export]
macro_rules! page_get_mut_inner {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::page_get_mut_inner_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! page_get_mut_inner_internal {
    ($ptr:expr, $local:ident, $page_inner:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = $ptr;

            let tracked perm = &$local.instance.thread_local_state_guards_page(
                    $local.thread_id, page_ptr.page_id@, &$local.thread_token).points_to;
            let page = vstd::raw_ptr::ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: mut inner_0, prev: prev_0, next: next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_inner = page.inner.borrow_mut(Tracked(&mut inner_0));

            { $body }

        } }
    }
}

pub use page_get_mut_inner;
pub use page_get_mut_inner_internal;

#[macro_export]
macro_rules! unused_page_get_mut_prev {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::unused_page_get_mut_prev_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! unused_page_get_mut_prev_internal {
    ($ptr:expr, $local:ident, $page_prev:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);


            let tracked perm = &$local.unused_pages.tracked_borrow(page_ptr.page_id@).points_to;
            let page = ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: inner_0, prev: mut prev_0, next: next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_prev = page.prev.read(Tracked(&mut prev_0));

            { $body }

            page.prev.write(Tracked(&mut prev_0), $page_prev);
        } }
    }
}

pub use unused_page_get_mut_prev;
pub use unused_page_get_mut_prev_internal;

#[macro_export]
macro_rules! unused_page_get_mut_inner {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::unused_page_get_mut_inner_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! unused_page_get_mut_inner_internal {
    ($ptr:expr, $local:ident, $page_inner:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);

            let tracked perm = &$local.unused_pages.tracked_borrow(page_ptr.page_id@).points_to;
            let page = vstd::raw_ptr::ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: mut inner_0, prev: prev_0, next: next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_inner = page.inner.borrow_mut(Tracked(&mut inner_0));

            { $body }

        } }
    }
}

pub use unused_page_get_mut_inner;
pub use unused_page_get_mut_inner_internal;


#[macro_export]
macro_rules! unused_page_get_mut_next {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::unused_page_get_mut_next_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! unused_page_get_mut_next_internal {
    ($ptr:expr, $local:ident, $page_next:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);

            let tracked perm = &$local.unused_pages.tracked_borrow(page_ptr.page_id@).points_to;
            let page = ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: inner_0, prev: prev_0, next: mut next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_next = page.next.read(Tracked(&mut next_0));

            { $body }

            page.next.write(Tracked(&mut next_0), $page_next);
        } }
    }
}

pub use unused_page_get_mut_next;
pub use unused_page_get_mut_next_internal;

#[macro_export]
macro_rules! unused_page_get_mut_count {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::unused_page_get_mut_count_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! unused_page_get_mut_count_internal {
    ($ptr:expr, $local:ident, $page_count:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);

            let tracked perm = &$local.unused_pages.tracked_borrow(page_ptr.page_id@).points_to;
            let page = ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: inner_0, prev: prev_0, next: next_0, count: mut count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_count = page.count.read(Tracked(&mut count_0));

            { $body }

            page.count.write(Tracked(&mut count_0), $page_count);
        } }
    }
}

pub use unused_page_get_mut_count;
pub use unused_page_get_mut_count_internal;


#[macro_export]
macro_rules! unused_page_get_mut {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::unused_page_get_mut_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! unused_page_get_mut_internal {
    ($ptr:expr, $local:ident, $page:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);

            let tracked psa = $local.unused_pages.tracked_remove(page_ptr.page_id@);
            let tracked PageSharedAccess { points_to: mut points_to, exposed } = psa;
            let mut $page = vstd::raw_ptr::ptr_mut_read(page_ptr.page_ptr, Tracked(&mut points_to));

            { $body }

            vstd::raw_ptr::ptr_mut_write(page_ptr.page_ptr, Tracked(&mut points_to), $page);
        } }
    }
}

pub use unused_page_get_mut;
pub use unused_page_get_mut_internal;


#[macro_export]
macro_rules! used_page_get_mut_prev {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::used_page_get_mut_prev_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! used_page_get_mut_prev_internal {
    ($ptr:expr, $local:ident, $page_prev:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);


            let tracked perm = &$local.instance.thread_local_state_guards_page(
                $local.thread_id, page_ptr.page_id@, &$local.thread_token).points_to;
            let page = vstd::raw_ptr::ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: inner_0, prev: mut prev_0, next: next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_prev = page.prev.read(Tracked(&mut prev_0));

            { $body }

            page.prev.write(Tracked(&mut prev_0), $page_prev);
        } }
    }
}

pub use used_page_get_mut_prev;
pub use used_page_get_mut_prev_internal;

#[macro_export]
macro_rules! heap_get_pages {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::heap_get_pages_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! heap_get_pages_internal {
    ($ptr:expr, $local:ident, $pages:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let heap_ptr = ($ptr);

            let tracked perm = &$local.instance.thread_local_state_guards_heap(
                $local.thread_id, &$local.thread_token).points_to;
            let heap = vstd::raw_ptr::ptr_ref(heap_ptr.heap_ptr, Tracked(perm));
            let mut $pages = heap.pages.read(Tracked(&mut $local.heap.pages));

            { $body }

            heap.pages.write(Tracked(&mut $local.heap.pages), $pages);
        } }
    }
}

pub use heap_get_pages;
pub use heap_get_pages_internal;

#[macro_export]
macro_rules! heap_get_pages_free_direct {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::heap_get_pages_free_direct_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! heap_get_pages_free_direct_internal {
    ($ptr:expr, $local:ident, $pages_free_direct:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let heap_ptr = ($ptr);

            let tracked perm = &$local.instance.thread_local_state_guards_heap(
                $local.thread_id, &$local.thread_token).points_to;
            let heap = vstd::raw_ptr::ptr_ref(heap_ptr.heap_ptr, Tracked(perm));
            let mut $pages_free_direct = heap.pages_free_direct.read(Tracked(&mut $local.heap.pages_free_direct));

            { $body }

            let mut $pages_free_direct = heap.pages_free_direct.write(Tracked(&mut $local.heap.pages_free_direct), $pages_free_direct);
        } }
    }
}

pub use heap_get_pages_free_direct;
pub use heap_get_pages_free_direct_internal;



#[macro_export]
macro_rules! used_page_get_mut_next {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::used_page_get_mut_next_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! used_page_get_mut_next_internal {
    ($ptr:expr, $local:ident, $page_next:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let page_ptr = ($ptr);


            let tracked perm = &$local.instance.thread_local_state_guards_page(
                $local.thread_id, page_ptr.page_id@, &$local.thread_token).points_to;
            let page = vstd::raw_ptr::ptr_ref(page_ptr.page_ptr, Tracked(perm));

            let tracked PageLocalAccess { inner: inner_0, prev: prev_0, next: mut next_0, count: count_0 } =
                $local.pages.tracked_remove(page_ptr.page_id@);
            let mut $page_next = page.next.read(Tracked(&mut next_0));

            { $body }

            page.next.write(Tracked(&mut next_0), $page_next);
        } }
    }
}

pub use used_page_get_mut_next;
pub use used_page_get_mut_next_internal;

#[verus::trusted]
#[verifier::external_body]
pub fn print_hex(s: &'static str, u: usize)
{
    println!("{:} {:x}", s, u);
}

#[verus::trusted]
#[cfg(feature = "override_system_allocator")]
#[verifier::external_body]
pub fn todo()
{
    std::process::abort();
}

#[verus::trusted]
#[cfg(not(feature = "override_system_allocator"))]
#[verifier::external_body]
pub fn todo()
{
    panic!("todo");
}

#[macro_export]
macro_rules! segment_get_mut_main {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::segment_get_mut_main_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! segment_get_mut_main_internal {
    ($ptr:expr, $local:ident, $segment_main:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let segment_ptr = $ptr;

            let tracked perm = &$local.instance.thread_local_state_guards_segment(
                    $local.thread_id, segment_ptr.segment_id@, &$local.thread_token).points_to;
            let segment = vstd::raw_ptr::ptr_ref(segment_ptr.segment_ptr, Tracked(perm));

            let tracked SegmentLocalAccess { main: mut main_0, mem: mem_0, main2: main2_0 } =
                $local.segments.tracked_remove(segment_ptr.segment_id@);
            let mut $segment_main = segment.main.borrow_mut(Tracked(&mut main_0));

            { $body }

        } }
    }
}

pub use segment_get_mut_main;
pub use segment_get_mut_main_internal;

#[macro_export]
macro_rules! segment_get_mut_main2 {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::segment_get_mut_main2_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! segment_get_mut_main2_internal {
    ($ptr:expr, $local:ident, $segment_main2:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let segment_ptr = $ptr;

            let tracked perm = &$local.instance.thread_local_state_guards_segment(
                    $local.thread_id, segment_ptr.segment_id@, &$local.thread_token).points_to;
            let segment = vstd::raw_ptr::ptr_ref(segment_ptr.segment_ptr, Tracked(perm));

            let tracked SegmentLocalAccess { main: main_0, mem: mem_0, main2: mut main2_0 } =
                $local.segments.tracked_remove(segment_ptr.segment_id@);
            let mut $segment_main2 = segment.main2.borrow_mut(Tracked(&mut main2_0));

            { $body }

        } }
    }
}

pub use segment_get_mut_main2;
pub use segment_get_mut_main2_internal;

#[macro_export]
macro_rules! segment_get_mut_local {
    [$($tail:tt)*] => {
        ::vstd::prelude::verus_exec_macro_exprs!(
            $crate::types::segment_get_mut_local_internal!($($tail)*))
    };
}

#[macro_export]
macro_rules! segment_get_mut_local_internal {
    ($ptr:expr, $local:ident, $segment_local:ident => $body:expr) => {
        ::vstd::prelude::verus_exec_expr!{ {
            let segment_ptr = $ptr;

            let tracked perm = &$local.instance.thread_local_state_guards_segment(
                    $local.thread_id, segment_ptr.segment_id@, &$local.thread_token).points_to;
            let segment = vstd::raw_ptr::ptr_ref(segment_ptr.segment_ptr, Tracked(perm));

            let tracked mut $segment_local =
                $local.segments.tracked_remove(segment_ptr.segment_id@);

            { $body }

        } }
    }
}

pub use segment_get_mut_local;
pub use segment_get_mut_local_internal;


}
