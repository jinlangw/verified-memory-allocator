use core::intrinsics::{unlikely, likely};
use vstd::prelude::*;
use vstd::set_lib::set_int_range;
use crate::config::*;
use crate::os_mem::*;
use crate::layout::*;
use crate::types::todo;


verus!{

pub fn os_alloc_aligned_offset(
    size: usize,
    alignment: usize,
    offset: usize,
    request_commit: bool,
    allow_large: bool,
) -> (res: (*mut u8, bool, Tracked<MemChunk>))
    requires alignment + page_size() <= usize::MAX,
        size as int % page_size() == 0,
        size == SEGMENT_SIZE,
        alignment as int % page_size() == 0,
        offset == 0,
    ensures ({ let (addr, is_large, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.os_has_range(addr as int, size as int)
            && mem@.points_to.provenance() == addr@.provenance
            && addr as int + size <= usize::MAX
            && (request_commit ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (request_commit ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!request_commit ==> mem@.os_has_range_no_read_write(addr as int, size as int))
            && (alignment != 0 ==> (addr as int + offset) % alignment as int == 0)
        )
    })
{
    if offset > SEGMENT_SIZE as usize {
        return (core::ptr::null_mut(), allow_large, Tracked(MemChunk::empty()));
    }

    if offset == 0 {
        return os_alloc_aligned(size, alignment, request_commit, allow_large);
    } else {
        assert(false); loop{}
        /*
        let extra = align_up(offset, alignment) - offset;
        let oversize = size + extra;

        let (start, commited, is_large) = os_alloc_aligned(oversize, alignment, request_commit, allow_large);
        if start == 0 {
            return 0;
        }

        let p = start + extra;
        if commited && extra > get_page_size() {
        }
        */
    }
}

pub fn os_good_alloc_size(size: usize) -> (res: usize)
    requires size as int % page_size() == 0,
    ensures res as int % page_size() == 0,
      res >= size,
      size == SEGMENT_SIZE ==> res == SEGMENT_SIZE,
{
    let kib = 1024;
    let mib = 1024*1024;

    let align_size = if size < 512 * kib {
        get_page_size()
    } else if size < 2 * mib {
        64 * kib
    } else if size < 8 * mib {
        256 * kib
    } else if size < 32 * mib {
        mib
    } else {
        4 * mib
    };

    if unlikely(size >= usize::MAX - align_size) {
        size
    } else {
        let x = align_up(size, align_size);
        proof {
            const_facts();
            mod_trans(x as int, align_size as int, page_size());
            if size <= SEGMENT_SIZE {
                assert((size + page_size() - 1) / page_size() <= 8192);
                assert((size + page_size() - 1) / page_size() * page_size() <= SEGMENT_SIZE);
                assert((SEGMENT_SIZE + 0x400000 - 1) / 0x400000 as int == 8) by(compute); // needed on x64-docker
            }
        }
        return x;
    }
}

pub fn os_alloc_aligned(
    size: usize,
    alignment: usize,
    request_commit: bool,
    allow_large: bool
) -> (res: (*mut u8, bool, Tracked<MemChunk>))
    requires
        alignment + page_size() <= usize::MAX,
        size == SEGMENT_SIZE,
        size as int % page_size() == 0,
        alignment as int % page_size() == 0,
    ensures ({ let (addr, is_large, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.os_has_range(addr as int, size as int)
            && mem@.points_to.provenance() == addr@.provenance
            && addr as int + size <= usize::MAX
            && (request_commit ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (request_commit ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!request_commit ==> mem@.os_has_range_no_read_write(addr as int, size as int))
            && (alignment != 0 ==> addr as int % alignment as int == 0)
        )
    })
{
    if size == 0 {
        return (core::ptr::null_mut(), allow_large, Tracked(MemChunk::empty()));
    }
    let size1 = os_good_alloc_size(size);
    let alignment1 = align_up(alignment, get_page_size());
    proof {
        assert(alignment1 == alignment);
        assert(size1 >= size);
        const_facts();
    }
    os_mem_alloc_aligned(size1, alignment1, request_commit, allow_large)
}

pub fn os_mem_alloc_aligned(
    size: usize,
    alignment: usize,
    request_commit: bool,
    allow_large: bool,
) -> (res: (*mut u8, bool, Tracked<MemChunk>))
    requires
        size as int % page_size() == 0,
        size <= SEGMENT_SIZE,
        alignment as int % page_size() == 0,
    ensures ({ let (addr, is_large, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.os_exact_range(addr as int, size as int)
            && mem@.points_to.provenance() == addr@.provenance
            && addr as int + size <= usize::MAX
            && (request_commit ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (request_commit ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!request_commit ==> mem@.os_has_range_no_read_write(addr as int, size as int))
            && (alignment != 0 ==> addr as int % alignment as int == 0)
        )
    })
{
    let mut allow_large = allow_large;
    if !request_commit {
        allow_large = false;
    }

    if (!(alignment >= get_page_size() && ((alignment & (alignment - 1)) == 0))) {
        return (core::ptr::null_mut(), allow_large, Tracked(MemChunk::empty()));
    }
    if size == 0 {
        return (core::ptr::null_mut(), allow_large, Tracked(MemChunk::empty()));
    }

    let (p, is_large, Tracked(mem)) = os_mem_alloc(size, alignment, request_commit, allow_large);
    if p.addr() == 0 {
        return (p, is_large, Tracked(mem));
    }

    if p.addr() % alignment != 0 {
        if p.addr() % get_page_size() != 0 {
            return (core::ptr::null_mut(), is_large, Tracked(mem));
        }
        proof {
            if request_commit {
                assert forall |addr: int| mem.range_os_rw().contains(addr)
                    implies mem.range_points_to().contains(addr)
                by {
                    assert(mem.range_os().contains(addr));
                    assert(set_int_range(p as int, p as int + size as int).contains(addr));
                    assert(mem.range_points_to().contains(addr));
                }
            } else {
                assert forall |addr: int| mem.range_os_rw().contains(addr)
                    implies mem.range_points_to().contains(addr)
                by {
                    assert(mem.range_os().contains(addr));
                    assert(set_int_range(p as int, p as int + size as int).contains(addr));
                    assert(mem.range_os_none().contains(addr));
                    assert(mem.os[addr]@.mem_protect == MemProtect { read: true, write: true });
                    assert(mem.os[addr]@.mem_protect == MemProtect { read: false, write: false });
                }
            }
            assert(mem.has_pointsto_for_all_read_write());
        }
        munmap_release(p, size, Tracked(mem));

        if size > usize::MAX - alignment {
            return (core::ptr::null_mut(), is_large, Tracked(MemChunk::empty()));
        }
        let over_size = size + alignment;
        proof {
            assert(page_size() > 0) by(compute);
            vstd::arithmetic::div_mod::lemma_add_mod_noop(
                size as int, alignment as int, page_size());
            assert(over_size as int == size as int + alignment as int);
            assert(over_size as int % page_size() == 0);
        }
        let (q, Tracked(mut over_mem)) = mmap_prot_read_write(core::ptr::null_mut(), over_size);
        if q.addr() == MAP_FAILED {
            return (core::ptr::null_mut(), is_large, Tracked(over_mem));
        }

        let aligned_addr = align_up(q.addr(), alignment);
        let aligned = q.with_addr(aligned_addr);
        let head_size = aligned_addr - q.addr();
        proof {
            mod_trans(aligned as int, alignment as int, page_size());
            assert(aligned as int % page_size() == 0);
            assert(head_size as int == aligned as int - q as int);
            vstd::arithmetic::div_mod::lemma_sub_mod_noop(
                aligned as int, q as int, page_size());
            assert(head_size as int % page_size() == 0);
            assert(aligned as int <= q as int + alignment as int - 1);
            assert(head_size as int <= alignment as int - 1) by(nonlinear_arith)
                requires
                    head_size as int == aligned as int - q as int,
                    aligned as int <= q as int + alignment as int - 1;
            assert(head_size as int + size as int <= over_size as int) by(nonlinear_arith)
                requires
                    head_size as int <= alignment as int - 1,
                    over_size as int == size as int + alignment as int;
        }
        let tail_start = aligned_addr + size;
        let tail_ptr = q.with_addr(tail_start);
        let tail_size = over_size - head_size - size;
        proof {
            assert(tail_start as int == aligned as int + size as int);
            assert(tail_size as int == over_size as int - head_size as int - size as int);
            assert(tail_size as int == q as int + over_size as int - tail_start as int)
                by(nonlinear_arith)
                requires
                    head_size as int == aligned as int - q as int,
                    tail_start as int == aligned as int + size as int,
                    tail_size as int == over_size as int - head_size as int - size as int;
            vstd::arithmetic::div_mod::lemma_add_mod_noop(
                aligned as int, size as int, page_size());
            assert(tail_start as int % page_size() == 0);
            vstd::arithmetic::div_mod::lemma_add_mod_noop(
                q as int, over_size as int, page_size());
            assert((q as int + over_size as int) % page_size() == 0);
            vstd::arithmetic::div_mod::lemma_sub_mod_noop(
                q as int + over_size as int, tail_start as int, page_size());
            assert(tail_size as int % page_size() == 0);
        }

        if head_size > 0 {
            let tracked head_mem;
            proof {
                head_mem = over_mem.split(q as int, head_size as int);
                assert(head_mem.os.dom() =~= set_int_range(q as int, q as int + head_size as int));
                assert(head_mem.os_exact_range(q as int, head_size as int));
                assert(head_mem.wf());
                assert(head_mem.has_pointsto_for_all_read_write());
                assert(over_mem.os.dom() =~= set_int_range(aligned as int, q as int + over_size as int));
                assert(over_mem.os_exact_range(aligned as int, over_size as int - head_size as int));
                assert(over_mem.wf());
                assert(over_mem.has_pointsto_for_all_read_write());
            }
            munmap_release(q, head_size, Tracked(head_mem));
        }

        let tracked mut aligned_mem;
        proof {
            aligned_mem = over_mem.split(aligned as int, size as int);
            assert(aligned_mem.os.dom() =~= set_int_range(aligned as int, aligned as int + size as int));
            assert(aligned_mem.os_exact_range(aligned as int, size as int));
            assert(aligned_mem.wf());
            assert(over_mem.os.dom() =~= set_int_range(tail_start as int, tail_start as int + tail_size as int));
            assert(over_mem.os_exact_range(tail_start as int, tail_size as int));
            assert(over_mem.wf());
            if request_commit {
                assert(aligned_mem.os_has_range_read_write(aligned as int, size as int));
                assert(aligned_mem.pointsto_has_range(aligned as int, size as int));
            }
            assert(aligned_mem.has_pointsto_for_all_read_write());
            assert(over_mem.has_pointsto_for_all_read_write());
            assert(aligned_mem.points_to.provenance() == aligned@.provenance);
            assert(over_mem.points_to.provenance() == tail_ptr@.provenance);
        }
        if tail_size > 0 {
            munmap_release(tail_ptr, tail_size, Tracked(over_mem));
        }
        if !request_commit {
            mprotect_prot_none(aligned, size, Tracked(&mut aligned_mem));
        }
        return (aligned, is_large, Tracked(aligned_mem));
    }

    (p, is_large, Tracked(mem))
}

fn os_mem_alloc(
    size: usize,
    try_alignment: usize,
    request_commit: bool,
    allow_large: bool,
) -> (res: (*mut u8, bool, Tracked<MemChunk>))
    requires
        size as int % page_size() == 0,
        size <= SEGMENT_SIZE,
        try_alignment == 1 || try_alignment as int % page_size() == 0,
    ensures ({ let (addr, is_large, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.points_to.provenance() == addr@.provenance
            && addr as int + size <= usize::MAX
            && mem@.os_exact_range(addr as int, size as int)
            && (request_commit ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (request_commit ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!request_commit ==> mem@.os_has_range_no_read_write(addr as int, size as int))
        )
    })
{
    if size == 0 { 
        return (core::ptr::null_mut(), allow_large, Tracked(MemChunk::empty()));
    }

    let mut allow_large = allow_large;
    if !request_commit {
        allow_large = false;
    }

    let mut try_alignment = try_alignment;
    if try_alignment == 0 { try_alignment = 1; }

    unix_mmap(core::ptr::null_mut(), size, try_alignment, request_commit, false, allow_large)
}

fn use_large_os_page(size: usize, alignment: usize) -> bool {
    false
}

fn unix_mmap(
    addr: *mut u8,
    size: usize,
    try_alignment: usize,
    prot_rw: bool,
    large_only: bool,
    allow_large: bool,
) -> (res: (*mut u8, bool, Tracked<MemChunk>))
    requires
        addr as int % page_size() == 0,
        size as int % page_size() == 0,
        size <= SEGMENT_SIZE,
        try_alignment == 1 || try_alignment as int % page_size() == 0,
    ensures ({ let (addr, is_large, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.points_to.provenance() == addr@.provenance
            && mem@.os_exact_range(addr as int, size as int)
            && addr as int + size <= usize::MAX
            && (prot_rw ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (prot_rw ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!prot_rw ==> mem@.os_has_range_no_read_write(addr as int, size as int))
        )
    })
{
    let is_large = true;
    if (large_only || use_large_os_page(size, try_alignment)) && allow_large {
        return (core::ptr::null_mut(), is_large, Tracked(MemChunk::empty()));
    }

    let is_large = false;
    let (p, Tracked(mem)) = unix_mmapx(addr, size, try_alignment, prot_rw);
    (p, is_large, Tracked(mem))
}

exec static ALIGNED_BASE: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

#[inline]
fn aligned_base_add(s: usize) -> usize {
    ALIGNED_BASE.fetch_add(s, core::sync::atomic::Ordering::AcqRel)
}

#[inline]
fn aligned_base_cas(s: usize, t: usize) {
    let _ = ALIGNED_BASE.compare_exchange(s, t, core::sync::atomic::Ordering::AcqRel, core::sync::atomic::Ordering::Acquire);
}

const HINT_BASE: usize = (2 as usize) << (40 as usize);
const HINT_AREA: usize = (4 as usize) << (40 as usize);
const HINT_MAX: usize = (30 as usize) << (40 as usize);

fn os_get_aligned_hint(try_alignment: usize, size: usize) -> (hint: usize)
    requires size <= SEGMENT_SIZE,
    ensures try_alignment != 0 ==> hint % try_alignment == 0,
      try_alignment <= 1 ==> hint == 0,
{
    proof { const_facts(); }

    if try_alignment <= 1 || try_alignment > SEGMENT_SIZE as usize {
        return 0;
    }

    let size = align_up(size, SEGMENT_SIZE as usize);
    if size > 1024*1024*1024 {
        return 0;
    }

    let mut hint = aligned_base_add(size);
    if hint == 0 || hint > HINT_MAX {
        let iinit = HINT_BASE;
        
        //let r = heap_random_next();
        //let iinit = iinit + ((MI_SEGMENT_SIZE * ((r>>17) & 0xFFFFF)) % MI_HINT_AREA);

        let expected = hint.wrapping_add(size);
        aligned_base_cas(expected, iinit);
        hint = aligned_base_add(size);
    }

    if hint % try_alignment != 0 {
        return 0;
    }
    return hint;
}

fn unix_mmapx(
    hint: *mut u8,
    size: usize,
    try_alignment: usize,
    prot_rw: bool,
) -> (res: (*mut u8, Tracked<MemChunk>))
    requires
        hint as int % page_size() == 0,
        size as int % page_size() == 0,
        size <= SEGMENT_SIZE,
        try_alignment > 1 ==> try_alignment as int % page_size() == 0,
    ensures ({ let (addr, mem) = res;
        addr as int != 0 ==> (
            mem@.wf()
            && mem@.os_exact_range(addr as int, size as int)
            && mem@.points_to.provenance() == addr@.provenance
            && addr as int + size <= usize::MAX
            && (prot_rw ==> mem@.os_has_range_read_write(addr as int, size as int))
            && (prot_rw ==> mem@.pointsto_has_range(addr as int, size as int))
            && (!prot_rw ==> mem@.os_has_range_no_read_write(addr as int, size as int))
        )
    })
{
    if hint.addr()  == 0 && INTPTR_SIZE >= 8 {
        let hinti = os_get_aligned_hint(try_alignment, size);
        let hint = hint.with_addr(hinti);
        proof {
            const_facts();
            if try_alignment > 1 {
                mod_trans(hint as int, try_alignment as int, page_size());
            }
        }
        if hint.addr() != 0 {
            let (p, Tracked(mem)) = if prot_rw {
                mmap_prot_read_write(hint, size)
            } else {
                mmap_prot_none(hint, size)
            };
            if p.addr() != MAP_FAILED {
                return (p, Tracked(mem));
            }
        }
    }
    let (p, Tracked(mem)) = if prot_rw {
        mmap_prot_read_write(hint, size)
    } else {
        mmap_prot_none(hint, size)
    };
    if p.addr() != MAP_FAILED {
        return (p, Tracked(mem));
    }

    return (core::ptr::null_mut(), Tracked(mem));
}

}
