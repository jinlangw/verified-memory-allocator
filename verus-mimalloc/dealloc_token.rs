#![allow(unused_imports)]
use vstd::raw_ptr::*;
use crate::tokens::{Mim, BlockId, DelayState};
use crate::types::*;
use crate::layout::*;
use vstd::prelude::*;
use vstd::set_lib::*;

verus!{

pub tracked struct MimDealloc {
    pub(crate) tracked padding: PointsToRaw,

    // Size of the allocation from the user perspective, <= the block size
    pub(crate) ghost _size: int,

    // Memory to make up the difference between user size and block size
    pub(crate) tracked inner: MimDeallocInner,
}

pub tracked struct MimDeallocInner {
    pub tracked mim_instance: Mim::Instance,
    pub tracked mim_block: Mim::block,

    pub ghost ptr: *mut u8,
}

pub uninterp spec fn valid_block_token(block: Mim::block, instance: Mim::Instance) -> bool;


impl MimDeallocInner {
    #[verifier(inline)]
    pub open spec fn block_id(&self) -> BlockId {
        self.mim_block.key()
    }

    pub uninterp spec fn wf(&self) -> bool;


}

impl MimDealloc {
    pub uninterp spec fn block_id(&self) -> BlockId;

    pub uninterp spec fn ptr(&self) -> *mut u8;

    pub uninterp spec fn inst(&self) -> Mim::Instance;

    pub uninterp spec fn size(&self) -> int;

    #[verifier::type_invariant]
    uninterp spec fn wf(&self) -> bool;


    #[verifier::external_body]
    pub(crate) proof fn into_internal(tracked self, tracked points_to_raw: PointsToRaw)
        -> (tracked res: (MimDeallocInner, PointsToRaw))
    {
        unimplemented!();
    }
}

}
