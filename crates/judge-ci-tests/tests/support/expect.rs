use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

pub fn expect(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

}
