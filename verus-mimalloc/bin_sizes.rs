use vstd::prelude::*;
use vstd::assert_by_contradiction;
use vstd::calc;
use vstd::std_specs::bits::u64_leading_zeros;
use crate::config::*;

//fn main() {}

// BLOCK SIZE BINS
//
// For a given allocation size, what bin does it fit in?
// Based off of logic in mi_bin
//
// First  compute wsize = ceil(size / (word size))
//
// Now, each wsize up to 8 gets its own bin.
// After that, each number is rounded up to a number such that
// all its 1s in the binary representation are of the 3 most significant
//
//
// wsize      bin size                        bin #
//
// 0, 1       1                               1
// 2          2                               2
// 3          3                               3
// 4          4                               4
// 5          5                               5
// 6          6                               6
// 7          7                               7
// 8          8                               8
//
// 9, 10      10      (10 = 1010)             9
// 11, 12     12      (12 = 1100)             10
// 13, 14     14      (14 = 1010)             11
// 15, 16     16      (16 = 10000)            12
//
// 17-20      20      (20 = 10100)            13
// 21-24      24      (24 = 11000)            14
// 25-28      28      (28 = 11100)            15
// 29-32      32      (32 = 100000)           16
//
// ...
//
// This goes up to MEDIUM_OBJ_WSIZE_MAX, and after that, everything goes in the "huge bin"
// which has bin # BIN_HUGE.
//
// The bin # should fit in a u8.
//
// -----------------------------------------------------------------------------------
//
// SLICE BINS (SBINS)
//
// When we allocate a page spanning a given # of slices, the '# of slices' also goes
// into a bin. To keep things straight, I'm going to call this binning method
// "sbins", while the above is just normal "bins".
//
// The algorithm here is a similar, though for some reason size 8 is lumped in with
// the bin [9, 10], and everything from that point is shifted down an index.
//
// slices     bin size                        bin #
//
// 0          0                               0         (unused)
// 1          1                               1
// 2          2                               2
// 3          3                               3
// 4          4                               4
// 5          5                               5
// 6          6                               6
// 7          7                               7

// 8, 9, 10   10      (10 = 1010)             8

// 11, 12     12      (12 = 1100)             9
// 13, 14     14      (14 = 1010)             10
// 15, 16     16      (16 = 10000)            11
//
// 17-20      20      (20 = 10100)            11
// 21-24      24      (24 = 11000)            13
// 25-28      28      (28 = 11100)            14
// 29-32      32      (32 = 100000)           15
//
// ...
//
// 449-512    512                             31
//
// The max # of slices is SLICES_PER_SEGMENT (512) which goes in bin 31.

verus!{


// TODO: Pulled in constants to make this a standalone file
/*
global size_of usize == 8;

// Log of the (pointer-size in bytes) // TODO make configurable
pub const INTPTR_SHIFT: u64 = 3;
pub const INTPTR_SIZE: u64 = 8;

// Log of the size of a 'slice'
pub const SLICE_SHIFT: u64 = 13 + INTPTR_SHIFT;

// Size of a slice
pub const SLICE_SIZE: u64 = 65536; //(1 << SLICE_SHIFT);

// Log of the size of a 'segment'
pub const SEGMENT_SHIFT: u64 = 9 + SLICE_SHIFT;

// Log of the size of a 'segment'
pub const SEGMENT_SIZE: u64 = (1 << SEGMENT_SHIFT);

// Log of the size of a 'segment'
pub const SEGMENT_ALIGN: u64 = SEGMENT_SIZE;

// Size of a 'segment'
pub const SLICES_PER_SEGMENT: u64 = (SEGMENT_SIZE / SLICE_SIZE);

pub const BIN_HUGE: u64 = 73;

pub const PAGES_DIRECT: usize = SMALL_WSIZE_MAX + 1;
pub const SMALL_SIZE_MAX: usize = SMALL_WSIZE_MAX * INTPTR_SIZE as usize;
pub const SMALL_WSIZE_MAX: usize = 128;

pub const SEGMENT_BIN_MAX: usize = 31;

// maximum alloc size the user is allowed to request
// note: mimalloc use ptrdiff_t max here
pub const MAX_ALLOC_SIZE: usize = isize::MAX as usize;
*/


pub open spec fn valid_bin_idx(bin_idx: int) -> bool {
    1 <= bin_idx <= BIN_HUGE
}

pub open spec fn pow2(i: int) -> nat
    decreases i
{
    if i <= 0 {
        1
    } else {
        pow2(i - 1) * 2
    }
}

pub uninterp spec fn size_of_bin(bin_idx: int) -> nat
    recommends valid_bin_idx(bin_idx);

proof fn lemma_div_is_ordered(x: int, y: int, z: int) { }

proof fn pow2_subtracts(e1:nat, e2:nat) { }

proof fn pow2_properties() { }

proof fn shift_is_div(x:u64, shift:u64) { }

proof fn log2(i:u64) -> (e:nat) { 0 }

proof fn leading_zeros_between(lo:u64, mid:u64, hi:u64) { }

pub proof fn bin_size_result(size: usize) { }

pub proof fn smallest_bin_fitting_size_size_of_bin(bin_idx: int) { }

pub proof fn different_bin_size(bin_idx1: int, bin_idx2: int) { }





// spec equivalent of bin
pub open spec fn smallest_bin_fitting_size(size: int) -> int {
    let bytes_per_word = (usize::BITS / 8) as int;
    let wsize = (size + bytes_per_word - 1) / bytes_per_word;
    if wsize <= 1 {
        1
    } else if wsize <= 8 {
        wsize
    } else if wsize > 524288 {
        BIN_HUGE as int
    } else {
        let w = (wsize - 1) as u64;
        //let lz = w.leading_zeros();
        let lz = u64_leading_zeros(w);
        let b = (usize::BITS - 1 - lz) as u8;
        let shifted = (w >> (b - 2) as u64) as u8;
        let bin_idx = ((b * 4) + (shifted & 0x03)) - 3;
        bin_idx
    }
}





// TODO: The assertions in this lemma are duplicated in init.rs



/** Put our desired property into a proof-by-compute-friendly form **/














/********************************************************
 * TODO: All of these should be standard library proofs
 ********************************************************/





















/********************************************************
 * END: All of these should be standard library proofs
 ********************************************************/








/** Put our desired property into a proof-by-compute-friendly form **/



























/** Put our desired property into a proof-by-compute-friendly form **/








/** Put our desired property into a proof-by-compute-friendly form **/












/** Put our desired property into a proof-by-compute-friendly form **/










// The "proof" is below is broken into chunks,
// so (a) we don't exceed the interpreter's stack limit,
// and (b) because the interpreter time seems to scale
// non-linearly with recursion depth


// Used to compute a bin for a given size
pub fn bin(size: usize) -> (bin_idx: u8)
    {

    let bytes_per_word = usize::BITS as usize / 8;
    assert(usize::BITS / 8 == 8) by (nonlinear_arith);
    let wsize = (size + bytes_per_word - 1) / bytes_per_word;
    assert(((wsize * 8) + 8 - 1) / 8 == wsize) by (nonlinear_arith);

    if wsize <= 1 {
        1
    } else if wsize <= 8 {
        wsize as u8
    } else {
        assert(9 <= wsize < 131073);
        let w: u64 = (wsize - 1) as u64;
        assert(8 <= w < 131072);
        let lz: u32 = w.leading_zeros();
        assert(46 <= lz <= 60) by {
            assert(u64_leading_zeros(8) == 60) by (compute_only);
            assert(u64_leading_zeros(131072) == 46) by (compute_only);
            leading_zeros_between(8, w, 131072);
        }
        let ghost log2_w = log2(w);


        let b = (usize::BITS - 1 - lz) as u8;
        assert(b == log2_w);
        assert(3 <= b <= 17);

//        assert(w > 255 ==> u64_leading_zeros(w) <= 52) by {
//            if w > 255 {
//                assert(u64_leading_zeros(256) == 55) by (compute_only);
//                leading_zeros_between(256, w, 131072);
//            }
//        }
        // This isn't true with this limited context, b/c we need to know how w and b scale relative to each other
//        assert((w >> sub(b as u64, 2)) < 256) by (bit_vector)
//            requires 8 <= w < 131072 && 3 <= b <= 17;
        assert(w >> ((b as u64 - 2) as u64) <= 8) by {
            assert(w < pow2((log2_w + 1) as int));
            assert(pow2((log2_w - 2) as int) > 0) by { pow2_properties(); }
            assert(w as nat / pow2((log2_w - 2) as int) <=
                    pow2((log2_w + 1) as int) / pow2((log2_w - 2) as int)) by {
                lemma_div_is_ordered(w as int,
                                     pow2((log2_w + 1) as int) as int,
                                     pow2((log2_w - 2) as int) as int);
            }
            assert(pow2((log2_w + 1) as int) / pow2((log2_w - 2) as int) == pow2(3)) by {
                pow2_subtracts((log2_w - 2) as nat, log2_w + 1);
            }
            assert(pow2(3) == 8) by (compute_only);
            shift_is_div(w, ((b as u64 - 2) as u64));
        }
        assert((w >> sub(b as u64, 2)) < 256);

        let shifted = (w >> (b as u64 - 2)) as u8;

        assert((w >> sub(sub(63, lz as u64), 2)) & 0x03 < 4) by (bit_vector)
            requires 8 <= w < 131073 && 46 <= lz <= 60;
        //assert(((w >> sub(63 - lz as u64), 2)) & 0x03 < 4);
        //assert((w >> ((63 - lz as u64) - 2)) & 0x03 < 4);

        assert(shifted & 0x03 < 4) by (bit_vector);
        let bin_idx = ((b * 4) + (shifted & 0x03)) - 3;

        assert(valid_bin_idx(bin_idx as int));
        assert(bin_idx == smallest_bin_fitting_size(size as int));
        assert(size_of_bin(bin_idx as int) >= size) by { bin_size_result(size); };
        //assert(size_of_bin(bin_idx as int) >= size)
            // Can't call this because the precondition restricts it to small sizes
            // by { bounds_for_smallest_bin_fitting_size(size as int); }

        bin_idx
    }
}

//////// Segment bins

pub open spec fn valid_sbin_idx(sbin_idx: int) -> bool {
    0 <= sbin_idx <= SEGMENT_BIN_MAX
}

pub open spec fn smallest_sbin_fitting_size(i: int) -> int
{
    if i <= 8 {
        i
    } else {
        let w = (i - 1) as u64;
        let lz = u64_leading_zeros(w);
        let b = (usize::BITS - 1 - lz) as u8;
        let sbin_idx = ((b << 2u8) as u64 | ((w >> (b as u64 - 2) as u64) & 0x03)) - 4;
        sbin_idx
    }
}

pub proof fn valid_sbin_idx_smallest_sbin_fitting_size(i: int) { }







/** Put our desired property into a proof-by-compute-friendly form **/








/** Put our desired property into a proof-by-compute-friendly form **/






/** Put our desired property into a proof-by-compute-friendly form **/






pub fn slice_bin(slice_count: usize) -> (sbin_idx: usize)
    {
    // Based on mi_slice_bin8
    if slice_count <= 8 {
        slice_count
    } else {
        let w = (slice_count - 1) as u64;
        assert(SLICES_PER_SEGMENT == 512) by (compute_only);
        assert(9 <= slice_count <= 512);
        assert(8 <= w <= 511);
        let lz = w.leading_zeros();

        let b = (usize::BITS - 1 - lz) as u8;
        let sbin_idx = ((b << 2u8) as u64 | ((w >> (b as u64 - 2)) & 0x03)) - 4;
        assert(sbin_idx == smallest_sbin_fitting_size(slice_count as int));

        sbin_idx as usize
    }
}
}
